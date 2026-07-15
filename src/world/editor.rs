//! Persistent runtime world authoring.
//!
//! The editor owns path validation and atomic JSON/script replacement. Live
//! cache updates remain in `WorldState`, so administrator commands and trusted
//! item/fixture events share one persistence format without embedding game
//! presentation in Rust.

use rhai::Engine;
use serde_json::{Map, Value};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum WorldEditError {
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Rhai error: {0}")]
    Rhai(String),
}

#[derive(Debug, Clone)]
pub struct WorldEditor {
    data_root: PathBuf,
}

impl Default for WorldEditor {
    fn default() -> Self {
        Self::new("data")
    }
}

impl WorldEditor {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            data_root: root.into(),
        }
    }

    pub fn room_path(&self, zone: &str, room: &str) -> Result<PathBuf, WorldEditError> {
        validate_segment(zone)?;
        validate_segment(room)?;
        Ok(self
            .data_root
            .join("map")
            .join(zone)
            .join(format!("{room}.json")))
    }

    pub fn mob_path(&self, zone: &str, key: &str) -> Result<PathBuf, WorldEditError> {
        validate_segment(zone)?;
        validate_segment(key)?;
        Ok(self
            .data_root
            .join("mob")
            .join(zone)
            .join(format!("{key}.json")))
    }

    pub fn item_path(&self, key: &str) -> Result<PathBuf, WorldEditError> {
        validate_segment(key)?;
        Ok(self.data_root.join("item").join(format!("{key}.json")))
    }

    pub fn event_path(&self, zone: &str, relative: &str) -> Result<PathBuf, WorldEditError> {
        validate_segment(zone)?;
        let path = Path::new(relative.trim());
        if path.as_os_str().is_empty()
            || path.is_absolute()
            || path.components().any(|component| {
                !matches!(component, Component::Normal(_))
                    || component.as_os_str().to_string_lossy().contains('\0')
            })
        {
            return Err(WorldEditError::InvalidIdentifier(relative.to_string()));
        }
        let path = if path.extension().is_some() {
            path.to_path_buf()
        } else {
            path.with_extension("rhai")
        };
        Ok(self.data_root.join("script").join(zone).join(path))
    }

    pub fn upsert_room(
        &self,
        zone: &str,
        room: &str,
        mut patch: Map<String, Value>,
        create_only: bool,
    ) -> Result<PathBuf, WorldEditError> {
        let path = self.room_path(zone, room)?;
        let existed = path.exists();
        if create_only && existed {
            return Err(WorldEditError::AlreadyExists(format!("{zone}:{room}")));
        }
        let mut info = if existed {
            read_section(&path, "맵정보")?
        } else {
            Map::new()
        };
        info.append(&mut patch);
        info.entry("존이름")
            .or_insert_with(|| Value::String(zone.to_string()));
        info.entry("이름")
            .or_insert_with(|| Value::String(format!("{zone} {room}")));
        info.entry("설명")
            .or_insert_with(|| Value::Array(Vec::new()));
        info.entry("맵속성")
            .or_insert_with(|| Value::Array(Vec::new()));
        info.entry("출구")
            .or_insert_with(|| Value::Array(Vec::new()));
        write_section(&path, "맵정보", info)?;
        Ok(path)
    }

    pub fn upsert_exit(
        &self,
        zone: &str,
        room: &str,
        name: &str,
        destination_zone: &str,
        destination_room: &str,
        hidden: bool,
    ) -> Result<PathBuf, WorldEditError> {
        validate_segment(name)?;
        validate_segment(destination_zone)?;
        validate_segment(destination_room)?;
        let path = self.room_path(zone, room)?;
        let mut info = read_section(&path, "맵정보")?;
        let mut exits = string_array(info.remove("출구"));
        exits.retain(|entry| {
            entry
                .split_whitespace()
                .next()
                .map(|value| value.trim_end_matches('$') != name)
                .unwrap_or(true)
        });
        let marker = if hidden {
            format!("{name}$")
        } else {
            name.to_string()
        };
        exits.push(format!("{marker} {destination_zone}:{destination_room}"));
        info.insert(
            "출구".into(),
            Value::Array(exits.into_iter().map(Value::String).collect()),
        );
        write_section(&path, "맵정보", info)?;
        Ok(path)
    }

    pub fn upsert_mob(
        &self,
        zone: &str,
        key: &str,
        mut patch: Map<String, Value>,
        create_only: bool,
    ) -> Result<PathBuf, WorldEditError> {
        let path = self.mob_path(zone, key)?;
        let existed = path.exists();
        if create_only && existed {
            return Err(WorldEditError::AlreadyExists(format!("{zone}:{key}")));
        }
        let mut info = if existed {
            read_section(&path, "몹정보")?
        } else {
            Map::new()
        };
        info.append(&mut patch);
        info.entry("존이름")
            .or_insert_with(|| Value::String(zone.to_string()));
        info.entry("이름")
            .or_insert_with(|| Value::String(key.to_string()));
        write_section(&path, "몹정보", info)?;
        Ok(path)
    }

    pub fn place_mob(
        &self,
        zone: &str,
        room: &str,
        mob_key: &str,
    ) -> Result<PathBuf, WorldEditError> {
        let (mob_zone, mob_name) = split_key(mob_key)?;
        if mob_zone != zone {
            return Err(WorldEditError::InvalidData(
                "persistent room mobs must use the room zone".into(),
            ));
        }
        if !self.mob_path(mob_zone, mob_name)?.exists() {
            return Err(WorldEditError::NotFound(mob_key.to_string()));
        }
        let path = self.room_path(zone, room)?;
        let mut info = read_section(&path, "맵정보")?;
        let mut mobs = string_array(info.remove("몹"));
        mobs.push(mob_name.to_string());
        info.insert(
            "몹".into(),
            Value::Array(mobs.into_iter().map(Value::String).collect()),
        );
        write_section(&path, "맵정보", info)?;
        Ok(path)
    }

    pub fn upsert_item(
        &self,
        key: &str,
        mut patch: Map<String, Value>,
        create_only: bool,
    ) -> Result<PathBuf, WorldEditError> {
        let path = self.item_path(key)?;
        let existed = path.exists();
        if create_only && existed {
            return Err(WorldEditError::AlreadyExists(key.to_string()));
        }
        let mut info = if existed {
            read_section(&path, "아이템정보")?
        } else {
            Map::new()
        };
        info.append(&mut patch);
        info.entry("이름")
            .or_insert_with(|| Value::String(key.to_string()));
        info.entry("인덱스")
            .or_insert_with(|| Value::String(key.to_string()));
        write_section(&path, "아이템정보", info)?;
        Ok(path)
    }

    pub fn upsert_fixture(
        &self,
        zone: &str,
        room: &str,
        key: &str,
        kind: &str,
        mut patch: Map<String, Value>,
    ) -> Result<PathBuf, WorldEditError> {
        validate_segment(key)?;
        crate::world::FixtureKind::parse(kind)
            .ok_or_else(|| WorldEditError::InvalidData(format!("fixture kind: {kind}")))?;
        let path = self.room_path(zone, room)?;
        let mut info = read_section(&path, "맵정보")?;
        let mut fixtures = info
            .remove("fixtures")
            .or_else(|| info.remove("고정물"))
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let existing = fixtures
            .iter()
            .position(|value| value.get("key").and_then(Value::as_str) == Some(key));
        let mut fixture = existing
            .and_then(|index| fixtures.get(index))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        fixture.append(&mut patch);
        fixture.insert("key".into(), Value::String(key.to_string()));
        fixture.insert("kind".into(), Value::String(kind.to_string()));
        if let Some(index) = existing {
            fixtures[index] = Value::Object(fixture);
        } else {
            fixtures.push(Value::Object(fixture));
        }
        info.insert("fixtures".into(), Value::Array(fixtures));
        write_section(&path, "맵정보", info)?;
        Ok(path)
    }

    pub fn write_event(
        &self,
        zone: &str,
        relative: &str,
        source: &str,
    ) -> Result<PathBuf, WorldEditError> {
        Engine::new()
            .compile(source)
            .map_err(|error| WorldEditError::Rhai(error.to_string()))?;
        let path = self.event_path(zone, relative)?;
        atomic_write_bytes(&path, source.as_bytes())?;
        Ok(path)
    }

    pub fn room_owner(&self, zone: &str, room: &str) -> Option<String> {
        read_section(&self.room_path(zone, room).ok()?, "맵정보")
            .ok()?
            .get("주인")
            .and_then(Value::as_str)
            .map(str::to_string)
    }

    pub fn mob_owner(&self, zone: &str, key: &str) -> Option<String> {
        read_section(&self.mob_path(zone, key).ok()?, "몹정보")
            .ok()?
            .get("주인")
            .and_then(Value::as_str)
            .map(str::to_string)
    }
}

fn validate_segment(value: &str) -> Result<(), WorldEditError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains(['/', '\\', '\0', ':'])
    {
        Err(WorldEditError::InvalidIdentifier(value.to_string()))
    } else {
        Ok(())
    }
}

fn split_key(value: &str) -> Result<(&str, &str), WorldEditError> {
    let (zone, key) = value
        .split_once(':')
        .ok_or_else(|| WorldEditError::InvalidIdentifier(value.to_string()))?;
    validate_segment(zone)?;
    validate_segment(key)?;
    Ok((zone, key))
}

fn write_section(
    path: &Path,
    section: &str,
    info: Map<String, Value>,
) -> Result<(), WorldEditError> {
    let mut document = if path.exists() {
        serde_json::from_str::<Value>(&std::fs::read_to_string(path)?)?
            .as_object()
            .cloned()
            .ok_or_else(|| WorldEditError::InvalidData("document must be an object".into()))?
    } else {
        Map::new()
    };
    document.insert(section.to_string(), Value::Object(info));
    atomic_write_json(path, &Value::Object(document))
}

fn read_section(path: &Path, section: &str) -> Result<Map<String, Value>, WorldEditError> {
    if !path.exists() {
        return Err(WorldEditError::NotFound(path.display().to_string()));
    }
    let document: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    document
        .get(section)
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| WorldEditError::InvalidData(format!("missing {section}")))
}

fn string_array(value: Option<Value>) -> Vec<String> {
    match value {
        Some(Value::Array(values)) => values
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(value)) => value
            .split(['\r', '\n'])
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<(), WorldEditError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write_bytes(path, &bytes)
}

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), WorldEditError> {
    let Some(parent) = path.parent() else {
        return Err(WorldEditError::InvalidIdentifier(
            path.display().to_string(),
        ));
    };
    std::fs::create_dir_all(parent)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = parent.join(format!(".world-edit-{}-{nonce}.tmp", std::process::id()));
    std::fs::write(&temp, bytes)?;
    if let Err(error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(WorldEditError::Io(error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "muc-world-editor-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn creates_and_updates_every_persistent_world_definition() {
        let root = root();
        let editor = WorldEditor::new(&root);
        editor
            .upsert_room(
                "시험존",
                "1",
                Map::from_iter([
                    ("이름".into(), Value::String("첫 방".into())),
                    ("주인".into(), Value::String("소유자".into())),
                ]),
                true,
            )
            .unwrap();
        let room_path = editor.room_path("시험존", "1").unwrap();
        let mut document: Value =
            serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
        document
            .as_object_mut()
            .unwrap()
            .insert("확장정보".into(), serde_json::json!({"future": true}));
        atomic_write_json(&room_path, &document).unwrap();
        editor.upsert_room("시험존", "2", Map::new(), true).unwrap();
        editor
            .upsert_exit("시험존", "1", "북", "시험존", "2", false)
            .unwrap();
        editor
            .upsert_mob(
                "시험존",
                "문지기",
                Map::from_iter([("이름".into(), Value::String("청의문지기".into()))]),
                true,
            )
            .unwrap();
        editor.place_mob("시험존", "1", "시험존:문지기").unwrap();
        editor
            .upsert_item(
                "시험패",
                Map::from_iter([("이름".into(), Value::String("청동 시험패".into()))]),
                true,
            )
            .unwrap();
        editor
            .upsert_fixture(
                "시험존",
                "1",
                "기관문",
                "door",
                Map::from_iter([("name".into(), Value::String("청동 기관문".into()))]),
            )
            .unwrap();
        editor
            .write_event(
                "시험존",
                "기관문_열어.rhai",
                "fn main(ob, fixture_id, cmdline) { output(\"열렸다\"); }",
            )
            .unwrap();

        let mut rooms = crate::world::RoomCache::with_data_dir(root.join("map"));
        let room = rooms.get_room("시험존", "1").unwrap();
        let room = room.read().unwrap();
        assert_eq!(room.display_name, "첫 방");
        assert_eq!(
            room.get_exit_by_name("북")
                .and_then(|exit| exit.destination("시험존")),
            Some(("시험존".into(), "2".into()))
        );
        assert_eq!(room.mob_ids, vec!["문지기"]);
        assert_eq!(room.fixture_placements[0].key, "기관문");

        let mut mobs = crate::world::MobCache::with_data_dir(root.join("mob"));
        assert_eq!(
            mobs.load_mob("시험존", "문지기").unwrap().name,
            "청의문지기"
        );
        let mut items = crate::world::ItemCache::with_data_dir(root.join("item"));
        assert_eq!(items.load_item("시험패").unwrap().name, "청동 시험패");
        assert_eq!(editor.room_owner("시험존", "1").as_deref(), Some("소유자"));
        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(room_path).unwrap()).unwrap();
        assert_eq!(saved["확장정보"]["future"], true);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_path_escape_and_invalid_rhai_without_replacing_files() {
        let root = root();
        let editor = WorldEditor::new(&root);
        assert!(matches!(
            editor.upsert_room("../밖", "1", Map::new(), true),
            Err(WorldEditError::InvalidIdentifier(_))
        ));
        let result = editor.write_event("시험존", "문.rhai", "fn broken(");
        assert!(matches!(result, Err(WorldEditError::Rhai(_))));
        assert!(!root.join("script/시험존/문.rhai").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
