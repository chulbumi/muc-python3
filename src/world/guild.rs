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
        }
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            attr: HashMap::new(),
        }
    }

    pub fn load(&mut self) {
        let Ok(s) = std::fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Map<String, Value>>(&s) else {
            return;
        };
        self.attr.clear();
        for (k, v) in root {
            if let Some(m) = v.as_object() {
                self.attr.insert(k, m.clone());
            }
        }
    }

    pub fn save(&self) -> bool {
        let root: Map<String, Value> = self
            .attr
            .iter()
            .map(|(k, v)| (k.clone(), Value::Object(v.clone())))
            .collect();
        let s = serde_json::to_string_pretty(&root).unwrap_or_default();
        std::fs::write(&self.path, s).is_ok()
    }

    /// guild_get(guild_id, key) -> 문자열. 없으면 "".
    pub fn get_string(&self, id: &str, key: &str) -> String {
        self.attr
            .get(id)
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default()
    }

    /// guild_set(guild_id, key, value). value는 문자열. 리스트 등은 JSON 문자열로.
    pub fn set(&mut self, id: &str, key: &str, value: &str) {
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
        self.attr.keys().cloned().collect()
    }

    /// guild_has(id) -> bool
    pub fn has(&self, id: &str) -> bool {
        self.attr.contains_key(id)
    }

    /// guild_remove(id). 방파 삭제.
    pub fn remove(&mut self, id: &str) -> bool {
        self.attr.remove(id).is_some()
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

/// Remove a guild and clear its membership from persisted player records.
/// Python's administrator reset leaves no player attached to the deleted guild.
pub fn guild_reset(id: &str) -> bool {
    let removed = guild_remove(id);
    if !removed {
        return false;
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
