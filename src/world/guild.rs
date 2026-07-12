//! 길드(GUILD) 모듈. data/config/guild.json 로드/저장.
//! Python objs/guild.py, GUILD.attr[guild_id] = { 이름, 방주리스트, 부방주리스트, N명칭, N리스트, ... }

use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

const DEFAULT_PATH: &str = "data/config/guild.json";

/// 길드 데이터. attr[guild_id] = json object.
#[derive(Debug)]
pub struct Guild {
    path: PathBuf,
    /// guild_id -> { 이름, 방주리스트, 부방주리스트, ... }
    pub attr: HashMap<String, Map<String, Value>>,
    order: Vec<String>,
}

impl Default for Guild {
    fn default() -> Self {
        Self::new()
    }
}

impl Guild {
    pub fn new() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_PATH),
            attr: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            attr: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn load(&mut self) {
        let Ok(s) = std::fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<indexmap::IndexMap<String, Value>>(&s) else {
            return;
        };
        self.attr.clear();
        self.order.clear();
        for (k, v) in root {
            if let Some(m) = v.as_object() {
                self.order.push(k.clone());
                self.attr.insert(k, m.clone());
            }
        }
    }

    pub fn save(&self) -> bool {
        let root: indexmap::IndexMap<String, Value> = self
            .order
            .iter()
            .filter_map(|key| {
                self.attr
                    .get(key)
                    .map(|value| (key.clone(), Value::Object(value.clone())))
            })
            .collect();
        let s = serde_json::to_string_pretty(&root).unwrap_or_default();
        std::fs::write(&self.path, s).is_ok()
    }

    /// guild_get(guild_id, key) -> 문자열. 없으면 "".
    pub fn get_string(&self, id: &str, key: &str) -> String {
        self.attr
            .get(id)
            .and_then(|m| m.get(key))
            .map(|v| match v {
                Value::String(value) => value.clone(),
                Value::Number(value) => value.to_string(),
                Value::Bool(value) => value.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default()
    }

    /// guild_set(guild_id, key, value). value는 문자열. 리스트 등은 JSON 문자열로.
    pub fn set(&mut self, id: &str, key: &str, value: &str) {
        if !self.attr.contains_key(id) {
            self.order.push(id.to_string());
        }
        self.attr
            .entry(id.to_string())
            .or_default()
            .insert(key.to_string(), Value::String(value.to_string()));
    }

    /// guild_attr_keys(guild_id) -> 키 목록.
    pub fn attr_keys(&self, id: &str) -> Vec<String> {
        self.attr
            .get(id)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// guild_list() -> guild_id 목록.
    pub fn list(&self) -> Vec<String> {
        self.order.clone()
    }

    /// guild_has(id) -> bool
    pub fn has(&self, id: &str) -> bool {
        self.attr.contains_key(id)
    }

    /// guild_remove(id). 방파 삭제.
    pub fn remove(&mut self, id: &str) -> bool {
        let removed = self.attr.remove(id).is_some();
        if removed {
            self.order.retain(|saved| saved != id);
        }
        removed
    }
}

static GUILD: std::sync::OnceLock<std::sync::RwLock<Guild>> = std::sync::OnceLock::new();

fn get_guild() -> &'static std::sync::RwLock<Guild> {
    GUILD.get_or_init(|| {
        let mut g = Guild::new();
        g.load();
        std::sync::RwLock::new(g)
    })
}

pub fn guild_get(id: &str, key: &str) -> String {
    get_guild().read().unwrap().get_string(id, key)
}

pub fn guild_set(id: &str, key: &str, value: &str) {
    let mut g = get_guild().write().unwrap();
    g.set(id, key, value);
    let _ = g.save();
}

pub fn guild_attr_keys(id: &str) -> Vec<String> {
    get_guild().read().unwrap().attr_keys(id)
}

pub fn guild_list() -> Vec<String> {
    get_guild().read().unwrap().list()
}

pub fn guild_has(id: &str) -> bool {
    get_guild().read().unwrap().has(id)
}

pub fn guild_remove(id: &str) -> bool {
    let mut g = get_guild().write().unwrap();
    let ok = g.remove(id);
    if ok {
        let _ = g.save();
    }
    ok
}

fn clear_room_guild_owner(map_root: &std::path::Path, location: &str) -> Vec<String> {
    let Some((zone, room)) = location.split_once(':') else {
        return Vec::new();
    };
    let path = map_root.join(zone).join(format!("{room}.json"));
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(mut root) = serde_json::from_str::<Value>(&raw) else {
        return Vec::new();
    };
    let Some(info) = root.get_mut("맵정보").and_then(Value::as_object_mut) else {
        return Vec::new();
    };
    let entrances = match info.get("방파입구") {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(value)) => value
            .split("\r\n")
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };
    info.remove("방파주인");
    if let Ok(serialized) = serde_json::to_string_pretty(&root) {
        let _ = std::fs::write(path, serialized);
    }
    entrances
}

fn clear_guild_rooms(map_root: &std::path::Path, home: &str) -> Vec<(String, String)> {
    let Some((zone, _)) = home.split_once(':') else {
        return Vec::new();
    };
    let entrances = clear_room_guild_owner(map_root, home);
    let mut cleared = Vec::new();
    if let Some((home_zone, home_room)) = home.split_once(':') {
        cleared.push((home_zone.to_string(), home_room.to_string()));
    }
    for entrance in entrances {
        let location = if entrance.contains(':') {
            entrance
        } else {
            format!("{zone}:{entrance}")
        };
        clear_room_guild_owner(map_root, &location);
        if let Some((entry_zone, entry_room)) = location.split_once(':') {
            cleared.push((entry_zone.to_string(), entry_room.to_string()));
        }
    }
    cleared
}

/// Remove a guild and clear its membership from persisted player records.
/// Python's administrator reset leaves no player attached to the deleted guild.
pub fn guild_reset(id: &str) -> bool {
    let home = get_guild().read().unwrap().get_string(id, "방파맵");
    let removed = guild_remove(id);
    if !removed {
        return false;
    }
    if !home.is_empty() {
        let cleared = clear_guild_rooms(std::path::Path::new("data/map"), &home);
        if let Ok(mut world) = crate::world::get_world_state().write() {
            for (zone, room) in cleared {
                world.room_cache.remove_room(&zone, &room);
            }
        }
    }
    let Ok(entries) = std::fs::read_dir("data/user") else {
        return true;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut json) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let Some(attrs) = json
            .get_mut("사용자오브젝트")
            .and_then(|v| v.get_mut("attr"))
            .and_then(Value::as_object_mut)
        else {
            continue;
        };
        let belongs = attrs.get("소속").and_then(Value::as_str) == Some(id);
        if belongs {
            attrs.insert("소속".to_string(), Value::String(String::new()));
            attrs.insert("직위".to_string(), Value::String(String::new()));
            let _ = std::fs::write(
                path,
                serde_json::to_string_pretty(&json).unwrap_or_default(),
            );
        }
    }
    true
}

pub fn guild_save() -> bool {
    get_guild().read().unwrap().save()
}

fn role_members(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(raw)) => raw
            .split(['\r', '\n', ','])
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

pub fn guild_role_members(id: &str, role: &str) -> Vec<String> {
    let guild = get_guild().read().unwrap();
    let key = format!("{}리스트", role);
    guild
        .attr
        .get(id)
        .map(|attrs| role_members(attrs.get(&key)))
        .unwrap_or_default()
}

/// Move one member between Python `GUILD` role lists and persist the guild.
/// `None` means the destination role is unlimited (`방파인`).
pub fn guild_reassign_position(
    id: &str,
    member: &str,
    old_position: &str,
    new_position: &str,
    limit: Option<usize>,
) -> &'static str {
    let mut guild = get_guild().write().unwrap();
    let Some(attrs) = guild.attr.get_mut(id) else {
        return "missing_guild";
    };
    let old_key = format!("{old_position}리스트");
    let new_key = format!("{new_position}리스트");
    let mut destination = role_members(attrs.get(&new_key));
    if limit.is_some_and(|maximum| destination.len() >= maximum) {
        return "full";
    }
    let mut source = role_members(attrs.get(&old_key));
    let Some(index) = source.iter().position(|name| name == member) else {
        return "missing_member";
    };
    source.remove(index);
    destination.push(member.to_string());
    attrs.insert(
        old_key,
        Value::Array(source.into_iter().map(Value::String).collect()),
    );
    attrs.insert(
        new_key,
        Value::Array(destination.into_iter().map(Value::String).collect()),
    );
    if guild.save() {
        "ok"
    } else {
        "save_failed"
    }
}

/// Remove a member from the role list used by Python's GUILD object and update
/// the persisted member count. Both legacy CRLF strings and JSON arrays are
/// accepted because old character data uses both representations.
pub fn guild_kick_member(id: &str, position: &str, member: &str) -> bool {
    let mut guild = get_guild().write().unwrap();
    let Some(attrs) = guild.attr.get_mut(id) else {
        return false;
    };
    let key = format!("{}리스트", position);
    let Some(value) = attrs.get_mut(&key) else {
        return false;
    };
    let removed = match value {
        Value::Array(items) => {
            let before = items.len();
            items.retain(|item| item.as_str() != Some(member));
            before != items.len()
        }
        Value::String(raw) => {
            let mut parts: Vec<String> = raw
                .split(['\r', '\n', ','])
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            let before = parts.len();
            parts.retain(|item| item != member);
            let changed = before != parts.len();
            *raw = parts.join("\r\n");
            changed
        }
        _ => false,
    };
    if !removed {
        return false;
    }
    if let Some(count) = attrs.get_mut("방파원수") {
        if let Some(n) = count.as_i64() {
            *count = Value::String(n.saturating_sub(1).to_string());
        } else if let Some(raw) = count.as_str().map(str::to_string) {
            if let Ok(n) = raw.parse::<i64>() {
                *count = Value::String(n.saturating_sub(1).to_string());
            }
        }
    }
    let _ = guild.save();
    true
}

/// Move a member between role lists without changing the guild member count.
pub fn guild_move_member_role(id: &str, from: &str, to: &str, member: &str) -> bool {
    let mut guild = get_guild().write().unwrap();
    let Some(attrs) = guild.attr.get_mut(id) else {
        return false;
    };
    let remove_from = |value: &mut Value| -> bool {
        match value {
            Value::Array(items) => {
                let before = items.len();
                items.retain(|v| v.as_str() != Some(member));
                before != items.len()
            }
            Value::String(raw) => {
                let mut parts: Vec<_> = raw
                    .split(['\r', '\n', ','])
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                let before = parts.len();
                parts.retain(|v| v != member);
                *raw = parts.join("\r\n");
                before != parts.len()
            }
            _ => false,
        }
    };
    let from_key = format!("{}리스트", from);
    let to_key = format!("{}리스트", to);
    let removed = attrs.get_mut(&from_key).map(remove_from).unwrap_or(false);
    if !removed {
        return false;
    }
    match attrs.entry(to_key) {
        serde_json::map::Entry::Vacant(entry) => {
            entry.insert(Value::String(member.to_string()));
        }
        serde_json::map::Entry::Occupied(mut entry) => match entry.get_mut() {
            Value::Array(items) => items.push(Value::String(member.to_string())),
            Value::String(raw) => {
                if !raw.is_empty() {
                    raw.push_str("\r\n");
                }
                raw.push_str(member);
            }
            _ => *entry.get_mut() = Value::String(member.to_string()),
        },
    }
    let _ = guild.save();
    true
}

/// Python 방주권한양도 list mutation: the target leaves 부방주리스트,
/// the former leader is appended there, and 방주이름 changes. Python does
/// not maintain a separate 방주리스트 in this command.
pub fn guild_transfer_leader(id: &str, former: &str, target: &str) -> bool {
    let mut guild = get_guild().write().unwrap();
    let Some(attrs) = guild.attr.get_mut(id) else {
        return false;
    };
    let mut deputies = role_members(attrs.get("부방주리스트"));
    let Some(index) = deputies.iter().position(|member| member == target) else {
        return false;
    };
    deputies.remove(index);
    deputies.push(former.to_string());
    attrs.insert(
        "부방주리스트".to_string(),
        Value::Array(deputies.into_iter().map(Value::String).collect()),
    );
    attrs.insert("방주이름".to_string(), Value::String(target.to_string()));
    guild.save()
}

/// Add a member to a guild role list, preserving the legacy list encoding.
pub fn guild_add_member(id: &str, role: &str, member: &str) -> bool {
    let mut guild = get_guild().write().unwrap();
    let Some(attrs) = guild.attr.get_mut(id) else {
        return false;
    };
    let key = format!("{}리스트", role);
    let value = attrs
        .entry(key)
        .or_insert_with(|| Value::String(String::new()));
    let exists = match value {
        Value::Array(items) => items.iter().any(|v| v.as_str() == Some(member)),
        Value::String(raw) => raw.split(['\r', '\n', ',']).any(|v| v == member),
        _ => false,
    };
    if exists {
        return false;
    }
    match value {
        Value::Array(items) => items.push(Value::String(member.to_string())),
        Value::String(raw) => {
            if !raw.is_empty() {
                raw.push_str("\r\n");
            }
            raw.push_str(member);
        }
        _ => return false,
    }
    let count = attrs
        .entry("방파원수".to_string())
        .or_insert_with(|| Value::String("0".to_string()));
    let n = count
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| count.as_i64())
        .unwrap_or(0);
    *count = Value::String(n.saturating_add(1).to_string());
    let _ = guild.save();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_load_save_and_list_preserve_python_json_order_and_numeric_values() {
        let path = std::env::temp_dir().join(format!("muc-guild-order-{}.json", std::process::id()));
        std::fs::write(
            &path,
            r#"{
  "둘째": {"이름": "둘째방파", "방파원수": 12},
  "첫째": {"이름": "첫방파", "방파원수": 3}
}"#,
        )
        .unwrap();
        let mut guild = Guild::with_path(&path);
        guild.load();
        assert_eq!(guild.list(), vec!["둘째", "첫째"]);
        assert_eq!(guild.get_string("둘째", "방파원수"), "12");
        guild.set("셋째", "이름", "셋방파");
        guild.set("둘째", "방파맵", "존:1");
        assert_eq!(guild.list(), vec!["둘째", "첫째", "셋째"]);
        assert!(guild.save());
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.find("\"둘째\"").unwrap() < saved.find("\"첫째\"").unwrap());
        assert!(saved.find("\"첫째\"").unwrap() < saved.find("\"셋째\"").unwrap());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reset_room_cleanup_removes_owner_from_home_and_relative_or_absolute_entrances() {
        let root = std::env::temp_dir().join(format!(
            "muc-guild-rooms-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for (zone, room, entrances) in [
            ("시험존", "1", serde_json::json!(["2", "다른존:3"])),
            ("시험존", "2", serde_json::json!([])),
            ("다른존", "3", serde_json::json!([])),
        ] {
            let dir = root.join(zone);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("{room}.json")),
                serde_json::to_string_pretty(&serde_json::json!({
                    "맵정보": {
                        "이름": "시험방",
                        "방파주인": "시험방파",
                        "방파입구": entrances
                    }
                }))
                .unwrap(),
            )
            .unwrap();
        }

        let cleared = clear_guild_rooms(&root, "시험존:1");
        assert_eq!(
            cleared,
            vec![
                ("시험존".to_string(), "1".to_string()),
                ("시험존".to_string(), "2".to_string()),
                ("다른존".to_string(), "3".to_string())
            ]
        );
        for path in [
            root.join("시험존/1.json"),
            root.join("시험존/2.json"),
            root.join("다른존/3.json"),
        ] {
            let json: Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert!(json["맵정보"].get("방파주인").is_none());
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
