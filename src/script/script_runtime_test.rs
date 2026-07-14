use super::*;

#[test]
fn every_current_rhai_source_is_syntax_valid() {
    let mut storage = ScriptStorage::default();
    storage
        .load_all_scripts_checked()
        .expect("every cmds/*.rhai source must compile before registration");
    assert_eq!(storage.script_names().len(), 210);
}
#[test]
fn test_script_preserves_self_output_with_targeted_sends() {
    let root = std::env::temp_dir().join(format!(
        "muc_script_combined_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("combined.rhai"),
        r#"fn main(ob, line) {
                send_line(ob, "self");
                send_to_user("other", "target");
            }"#,
    )
    .unwrap();

    let config = ScriptConfig {
        script_dir: root.clone(),
        ..ScriptConfig::default()
    };
    let storage = ScriptStorage::new(config);
    let mut body = Body::new();
    body.set("이름", "self");
    let (output, special) = storage
        .execute("combined", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["self"]);
    assert!(matches!(
        special,
        Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
            if own == "self" && sends == &vec![("other".to_string(), "target".to_string())]
    ));

    let _ = std::fs::remove_dir_all(root);
}
#[test]
fn test_json_debug_command_requires_level_2000() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("제이슨"));
    let mut body = Body::new();
    body.set("이름", "일반사용자");
    body.set("관리자등급", 0i64);

    let (output, _) = storage
        .execute("제이슨", &mut body, "../user/밍밍", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 2000_i64);
    body.set("한글키", "값😀");
    let (admin_output, special) = storage
        .execute("제이슨", &mut body, "무시", None, None, None)
        .unwrap();
    assert!(
        admin_output.is_empty(),
        "Python writes only to the server console"
    );
    assert!(special.is_none());
    assert_eq!(python_json_ensure_ascii("한😀"), "\\ud55c\\ud83d\\ude00");
}
#[test]
fn test_read_text_file_is_confined_to_public_data() {
    assert!(read_public_text_file("/etc/passwd").is_empty());
    assert!(read_public_text_file("data/config/../user/밍밍.json").is_empty());
    if Path::new("data/text/notice.txt").exists() {
        assert!(!read_public_text_file("data/text/notice.txt").is_empty());
    }
}
#[test]
fn test_has_script() {
    let storage = ScriptStorage::default();
    assert!(!storage.has_script("nonexistent"));
}
#[test]
fn test_script_storage_new() {
    let storage = ScriptStorage::default();
    assert!(storage.config.script_dir.ends_with("cmds"));
}

#[test]
fn cached_ast_reloads_only_after_valid_script_or_library_change() {
    let root = std::env::temp_dir().join(format!(
        "muc_script_ast_cache_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let commands = root.join("cmds");
    let libraries = root.join("lib");
    std::fs::create_dir_all(&commands).unwrap();
    std::fs::create_dir_all(&libraries).unwrap();
    let command_path = commands.join("cached.rhai");
    let library_path = libraries.join("value.rhai");
    std::fs::write(&library_path, r#"fn lib_value() { "one" }"#).unwrap();
    std::fs::write(
        &command_path,
        r#"fn main(ob, line) { send_line(ob, "cmd1-" + lib_value()); }"#,
    )
    .unwrap();

    let config = ScriptConfig {
        script_dir: commands,
        lib_dir: libraries,
        ..ScriptConfig::default()
    };
    let mut storage = ScriptStorage::new(config);
    let mut body = Body::new();
    let run = |storage: &ScriptStorage, body: &mut Body| {
        storage
            .execute("cached", body, "", None, None, None)
            .unwrap()
            .0
    };
    assert_eq!(run(&storage, &mut body), vec!["cmd1-one"]);

    // Execution uses the cached AST, not the retained source string.
    storage.scripts.get_mut("cached").unwrap().source = "invalid source".to_string();
    assert_eq!(run(&storage, &mut body), vec!["cmd1-one"]);

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(
        &command_path,
        r#"fn main(ob, line) { send_line(ob, "cmd2-" + lib_value()); }"#,
    )
    .unwrap();
    assert!(storage.reload_script("cached").unwrap());
    assert_eq!(run(&storage, &mut body), vec!["cmd2-one"]);

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&library_path, r#"fn lib_value() { "two" }"#).unwrap();
    assert!(storage.reload_all().unwrap() >= 1);
    assert_eq!(run(&storage, &mut body), vec!["cmd2-two"]);

    // A bad hot reload never replaces the last valid source/revision/AST.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&command_path, "fn main(").unwrap();
    assert!(storage.reload_script("cached").is_err());
    assert_eq!(run(&storage, &mut body), vec!["cmd2-two"]);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn different_users_do_not_share_a_global_rhai_execution_mutex() {
    let root = std::env::temp_dir().join(format!(
        "muc_script_parallel_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("parallel.rhai"),
        r#"fn main(ob, line) { __test_sleep_ms(500); send_line(ob, line); }"#,
    )
    .unwrap();
    let storage = Arc::new(ScriptStorage::new(ScriptConfig {
        script_dir: root.clone(),
        lib_dir: root.join("lib"),
        ..ScriptConfig::default()
    }));
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let started = std::time::Instant::now();
    let handles = ["first", "second"].map(|line| {
        let storage = storage.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let mut body = Body::new();
            barrier.wait();
            storage
                .execute("parallel", &mut body, line, None, None, None)
                .unwrap()
                .0
        })
    });
    barrier.wait();
    let outputs = handles.map(|handle| handle.join().unwrap());
    let elapsed = started.elapsed();

    assert_eq!(outputs[0], vec!["first"]);
    assert_eq!(outputs[1], vec!["second"]);
    assert!(
        elapsed < std::time::Duration::from_millis(850),
        "two 500ms Rhai commands were serialized: {elapsed:?}"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_script_config_default() {
    let config = ScriptConfig::default();
    assert_eq!(config.script_dir, PathBuf::from("cmds"));
    assert!(config.hot_reload);
    assert_eq!(config.extension, ".rhai");
}
#[test]
fn password_hash_uses_bcrypt_and_reads_legacy_formats() {
    let stored = password_hash("새암호");
    assert!(stored.starts_with("$2"));
    assert_eq!(stored.split('$').nth(2), Some("10"));
    assert!(password_verify(&stored, "새암호"));
    assert!(!password_verify(&stored, "틀린암호"));
    assert!(!password_needs_upgrade(&stored));

    let old_bcrypt = bcrypt::hash("옛암호", bcrypt::DEFAULT_COST).unwrap();
    assert!(password_verify(&old_bcrypt, "옛암호"));
    assert!(password_needs_upgrade(&old_bcrypt));

    let prehashed = bcrypt::hash(password_sha256("전처리암호"), bcrypt::DEFAULT_COST).unwrap();
    let tagged_prehashed = format!("{BCRYPT_SHA256_PREFIX}{prehashed}");
    assert!(password_verify(&tagged_prehashed, "전처리암호"));
    assert!(password_needs_upgrade(&tagged_prehashed));

    assert!(password_verify("평문암호", "평문암호"));

    use sha2::{Digest, Sha512};
    let old_sha512 = format!("{:x}", Sha512::digest("해시암호".as_bytes()));
    assert!(password_verify(&old_sha512, "해시암호"));
    assert!(password_needs_upgrade(&old_sha512));
}
