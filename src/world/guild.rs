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
        let Ok(s) = std::fs::read_to_string(&self.path) else { return; };
        let Ok(root) = serde_json::from_str::<Map<String, Value>>(&s) else { return; };
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

pub fn guild_save() -> bool {
    get_guild().read().unwrap().save()
}
