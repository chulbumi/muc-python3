//! Runtime persistence layout for a private `data/` checkout.
//!
//! Static game content lives in `data/`. Mutable state lives in the sibling
//! `runtime/` tree and is reached through symlinks committed by the private
//! data repository. Keeping initialization here makes a fresh deployment safe
//! without teaching every Python-compatible `data/...` call site a new path.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const RUNTIME_DIRS: &[&str] = &["user", "log/group", "soul", "box", "state"];
const STATE_FILES: &[&str] = &[
    "book.json",
    "guild.json",
    "rank.json",
    "oneitem.json",
    "oneitem_index.json",
];

pub fn initialize() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let default_data = root.join("data");
    let default_runtime = root.join("runtime");
    let data_root = std::env::var_os("MUC_DATA_DIR").map_or_else(
        || default_data.clone(),
        |path| absolute_from(&root, PathBuf::from(path)),
    );
    let runtime_root = std::env::var_os("MUC_RUNTIME_DIR").map_or_else(
        || default_runtime.clone(),
        |path| absolute_from(&root, PathBuf::from(path)),
    );
    ensure_compatibility_link(&default_data, &data_root)?;
    ensure_compatibility_link(&default_runtime, &runtime_root)?;
    initialize_at(&data_root, &runtime_root)
}

fn absolute_from(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn ensure_compatibility_link(default_path: &Path, configured_path: &Path) -> io::Result<()> {
    if default_path == configured_path {
        return Ok(());
    }
    if default_path.exists() {
        let current = fs::canonicalize(default_path)?;
        let configured = fs::canonicalize(configured_path)?;
        if current == configured {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "{} already exists and does not point to configured path {}",
                default_path.display(),
                configured_path.display()
            ),
        ));
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(configured_path, default_path)
    }
    #[cfg(not(unix))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "custom data/runtime roots require symlink support",
        ))
    }
}

pub fn initialize_at(data_root: &Path, runtime_root: &Path) -> io::Result<()> {
    if !data_root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "game data directory is missing: {} (initialize the private data submodule)",
                data_root.display()
            ),
        ));
    }

    for relative in RUNTIME_DIRS {
        fs::create_dir_all(runtime_root.join(relative))?;
    }

    let defaults = data_root.join("defaults/state");
    let state = runtime_root.join("state");
    for filename in STATE_FILES {
        let destination = state.join(filename);
        if destination.exists() {
            continue;
        }
        let source = defaults.join(filename);
        if !source.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("runtime state seed is missing: {}", source.display()),
            ));
        }
        fs::copy(source, destination)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_runtime_directories_and_only_seeds_missing_state() {
        let root = std::env::temp_dir().join(format!(
            "muc-runtime-layout-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let data = root.join("data");
        let runtime = root.join("runtime");
        fs::create_dir_all(data.join("defaults/state")).unwrap();
        for filename in STATE_FILES {
            fs::write(data.join("defaults/state").join(filename), "default").unwrap();
        }
        fs::create_dir_all(runtime.join("state")).unwrap();
        fs::write(runtime.join("state/rank.json"), "preserved").unwrap();

        initialize_at(&data, &runtime).unwrap();

        for relative in RUNTIME_DIRS {
            assert!(runtime.join(relative).is_dir());
        }
        assert_eq!(
            fs::read_to_string(runtime.join("state/rank.json")).unwrap(),
            "preserved"
        );
        assert_eq!(
            fs::read_to_string(runtime.join("state/guild.json")).unwrap(),
            "default"
        );
        let _ = fs::remove_dir_all(root);
    }
}
