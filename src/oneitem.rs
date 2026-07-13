//! ONEITEM 시스템: 단일아이템(기연) 소유/상태 추적
//!
//! Python objs/oneitem.py, ONEITEM 전역과 동일.
//!
//! Files:
//! - data/config/oneitem.json: { "단일아이템": { "index": "owner [버림|보관|떨굼]" } }
//! - data/config/oneitem_index.json: { "단일아이템인덱스": { "이름": "index" } }
//!
//! 로드 시 "버림" 상태는 attr에 넣지 않음. have/drop/keep/destroy 시 save.

use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::RwLock;
use tracing::{info, warn};

const ATTR_PATH: &str = "data/config/oneitem.json";
const INDEX_PATH: &str = "data/config/oneitem_index.json";

/// ONEITEM 인메모리 상태. load 시 oneitem.json(버림 제외), oneitem_index.json에서 로드.
pub struct OneitemState {
    /// index -> "owner" or "owner 보관" or "owner 떨굼" (로드 시 "버림" 제외)
    pub attr: HashMap<String, String>,
    /// Python dict insertion order for the current process.
    attr_order: Vec<String>,
    /// name -> index (oneitem_index의 단일아이템인덱스)
    pub index: HashMap<String, String>,
    index_order: Vec<String>,
}

impl OneitemState {
    pub fn new() -> Self {
        Self {
            attr: HashMap::new(),
            attr_order: Vec::new(),
            index: HashMap::new(),
            index_order: Vec::new(),
        }
    }

    /// oneitem.json, oneitem_index.json 로드. 버림은 attr에 넣지 않음.
    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.attr.clear();
        self.attr_order.clear();
        self.index.clear();
        self.index_order.clear();

        if Path::new(ATTR_PATH).exists() {
            let s = fs::read_to_string(ATTR_PATH)?;
            let v: indexmap::IndexMap<String, indexmap::IndexMap<String, JsonValue>> =
                serde_json::from_str(&s)?;
            if let Some(obj) = v.get("단일아이템") {
                for (k, val) in obj {
                    let vs = val.as_str().unwrap_or("");
                    let words: Vec<&str> = vs.split_whitespace().collect();
                    if words.len() > 1 && words[1] == "버림" {
                        continue;
                    }
                    self.attr.insert(k.clone(), vs.to_string());
                    self.attr_order.push(k.clone());
                }
            }
        }

        if Path::new(INDEX_PATH).exists() {
            let s = fs::read_to_string(INDEX_PATH)?;
            let v: indexmap::IndexMap<String, indexmap::IndexMap<String, JsonValue>> =
                serde_json::from_str(&s)?;
            if let Some(obj) = v.get("단일아이템인덱스") {
                for (name, idx) in obj {
                    let idx_s = if let Some(n) = idx.as_i64() {
                        n.to_string()
                    } else if let Some(s) = idx.as_str() {
                        s.to_string()
                    } else {
                        idx.to_string()
                    };
                    self.index.insert(name.clone(), idx_s);
                    self.index_order.push(name.clone());
                }
            }
        }

        info!(
            "ONEITEM loaded: attr={} index={}",
            self.attr.len(),
            self.index.len()
        );
        Ok(())
    }

    /// oneitem.json에 attr만 저장 (단일아이템 루트).
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let values: indexmap::IndexMap<String, String> = self
            .attr_order
            .iter()
            .filter_map(|index| {
                self.attr
                    .get(index)
                    .map(|owner| (index.clone(), owner.clone()))
            })
            .collect();
        let root = indexmap::IndexMap::from([("단일아이템", values)]);
        let pretty = serde_json::to_string_pretty(&root)?;
        let mut f = fs::File::create(ATTR_PATH)?;
        f.write_all(pretty.as_bytes())?;
        Ok(())
    }

    /// index에 해당하는 이름. index는 name->index 이므로 값이 index인 name 반환.
    pub fn get_name(&self, index: &str) -> String {
        self.index_order
            .iter()
            .find(|name| self.index.get(*name).is_some_and(|value| value == index))
            .cloned()
            .unwrap_or_default()
    }

    /// attr[index] (owner 문자열). 없으면 "".
    pub fn get(&self, index: &str) -> String {
        self.attr.get(index).cloned().unwrap_or_default()
    }

    /// 소유 이전: attr[index]=name, save.
    pub fn have(&mut self, index: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.attr.contains_key(index) {
            self.attr_order.push(index.to_string());
        }
        self.attr.insert(index.to_string(), name.to_string());
        self.save()
    }

    /// 버림: attr[index]= "name 버림", save. (이름 충돌 회피로 do_drop)
    pub fn do_drop(&mut self, index: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.attr.contains_key(index) {
            self.attr_order.push(index.to_string());
        }
        self.attr
            .insert(index.to_string(), format!("{} 버림", name));
        self.save()
    }

    /// 떨굼: attr[index]= "name 떨굼", save.
    pub fn drop2(&mut self, index: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.attr.contains_key(index) {
            self.attr_order.push(index.to_string());
        }
        self.attr
            .insert(index.to_string(), format!("{} 떨굼", name));
        self.save()
    }

    /// 보관: attr[index]= "name 보관", save.
    pub fn keep(&mut self, index: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.attr.contains_key(index) {
            self.attr_order.push(index.to_string());
        }
        self.attr
            .insert(index.to_string(), format!("{} 보관", name));
        self.save()
    }

    /// 삭제: attr에서 제거, save.
    pub fn destroy(&mut self, index: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.attr.remove(index);
        self.attr_order.retain(|saved| saved != index);
        self.save()
    }

    /// name(인덱스 이름)으로 검색: index에 있으면 index, attr에 있으면 (true, owner). 없으면 (false, None).
    pub fn check_name(&self, name: &str) -> (bool, Option<String>) {
        let idx = match self.index.get(name) {
            Some(s) => s.as_str(),
            None => return (false, None),
        };
        let owner = self.attr.get(idx).map(|s| {
            s.split_whitespace()
                .next()
                .unwrap_or(s.as_str())
                .to_string()
        });
        (owner.is_some(), owner)
    }

    /// index로 검색: attr에 있으면 (true, Some(owner)).
    pub fn check_index(&self, index: &str) -> (bool, Option<String>) {
        let owner = self
            .attr
            .get(index)
            .map(|s| s.split_whitespace().next().unwrap_or(s).to_string());
        (owner.is_some(), owner)
    }

    /// 기연 형식: "%-16s (%-16s) : %s\r\n" for (name, index, owner). 파이썬 ONEITEM.attr 순회.
    pub fn list(&self) -> String {
        let mut out = String::new();
        for index in &self.attr_order {
            let Some(owner) = self.attr.get(index) else {
                continue;
            };
            let name = self.get_name(index);
            out.push_str(&format!("{:<16} ({:<16}) : {}\r\n", name, index, owner));
        }
        out
    }

    /// attr 비우고 save.
    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.attr.clear();
        self.attr_order.clear();
        self.save()
    }

    /// attr 키 목록 (스크립트용).
    pub fn attr_keys(&self) -> Vec<String> {
        self.attr_order.clone()
    }
}

impl Default for OneitemState {
    fn default() -> Self {
        Self::new()
    }
}

static ONEITEM: Lazy<RwLock<OneitemState>> = Lazy::new(|| {
    let mut s = OneitemState::new();
    if let Err(e) = s.load() {
        warn!("ONEITEM load failed: {}", e);
    }
    RwLock::new(s)
});

/// 스크립트 efunc: ONEITEM.get_name(index)
pub fn oneitem_get_name(index: &str) -> String {
    ONEITEM.read().unwrap().get_name(index)
}

/// 스크립트 efunc: ONEITEM.get(index)
pub fn oneitem_get(index: &str) -> String {
    ONEITEM.read().unwrap().get(index)
}

/// 스크립트 efunc: ONEITEM.have(index, name). 성공 true, 실패 false.
pub fn oneitem_have(index: &str, name: &str) -> bool {
    ONEITEM.write().unwrap().have(index, name).is_ok()
}

/// 스크립트 efunc: ONEITEM.drop(index, name)
pub fn oneitem_drop(index: &str, name: &str) -> bool {
    ONEITEM.write().unwrap().do_drop(index, name).is_ok()
}

/// 스크립트 efunc: ONEITEM.drop2(index, name)
pub fn oneitem_drop2(index: &str, name: &str) -> bool {
    ONEITEM.write().unwrap().drop2(index, name).is_ok()
}

/// 스크립트 efunc: ONEITEM.keep(index, name)
pub fn oneitem_keep(index: &str, name: &str) -> bool {
    ONEITEM.write().unwrap().keep(index, name).is_ok()
}

/// 스크립트 efunc: ONEITEM.destroy(index)
pub fn oneitem_destroy(index: &str) -> bool {
    let mut state = ONEITEM.write().unwrap();
    if !state.attr.contains_key(index) {
        return false;
    }
    state.destroy(index).is_ok()
}

pub fn oneitem_reload() -> bool {
    ONEITEM.write().unwrap().load().is_ok()
}

/// 스크립트 efunc: ONEITEM.checkOneItemName(name) -> map { found, owner }
pub fn oneitem_check_name(name: &str) -> rhai::Map {
    let (found, owner) = ONEITEM.read().unwrap().check_name(name);
    let mut m = rhai::Map::new();
    m.insert("found".into(), rhai::Dynamic::from(found));
    m.insert(
        "owner".into(),
        rhai::Dynamic::from(owner.unwrap_or_default()),
    );
    m
}

/// 스크립트 efunc: ONEITEM.checkOneItemIndex(index) -> map { found, owner }
pub fn oneitem_check_index(index: &str) -> rhai::Map {
    let (found, owner) = ONEITEM.read().unwrap().check_index(index);
    let mut m = rhai::Map::new();
    m.insert("found".into(), rhai::Dynamic::from(found));
    m.insert(
        "owner".into(),
        rhai::Dynamic::from(owner.unwrap_or_default()),
    );
    m
}

/// 스크립트 efunc: ONEITEM.list() — 기연 출력 문자열
pub fn oneitem_list() -> String {
    ONEITEM.read().unwrap().list()
}

/// 스크립트 efunc: ONEITEM.clear()
pub fn oneitem_clear() -> bool {
    ONEITEM.write().unwrap().clear().is_ok()
}

/// 스크립트 efunc: ONEITEM.attr 키 배열 (기연 등에서 index 순회용)
pub fn oneitem_attr_keys() -> rhai::Array {
    ONEITEM
        .read()
        .unwrap()
        .attr_keys()
        .into_iter()
        .map(rhai::Dynamic::from)
        .collect()
}

/// 스크립트 efunc: ONEITEM.index[name] — 기연이름으로 index 얻기. 없으면 "".
pub fn oneitem_get_index_by_name(name: &str) -> String {
    ONEITEM
        .read()
        .unwrap()
        .index
        .get(name)
        .cloned()
        .unwrap_or_default()
}

/// 스크립트 efunc: 기연리스트용. index(name->index) 전체를 [{name, index}, ...] 로. (단일아이템 필터는 Item 쪽 연동 시 적용)
pub fn oneitem_list_index_entries() -> rhai::Array {
    let guard = ONEITEM.read().unwrap();
    let mut arr = rhai::Array::new();
    // Python `기연리스트` iterates Item.Items, not ONEITEM.index.  Recreate
    // that source order by scanning the same item JSON files and retaining
    // only records whose 아이템속성 contains 단일아이템.
    let item_files = std::fs::read_dir("data/item")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    let mut ordered_paths = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    // Mob.init() calls getItem(사용아이템) before loadAllItem(), so those
    // unique items precede the regular item-directory scan in Python.
    if let Ok(zones) = std::fs::read_dir("data/mob") {
        for zone in zones.flatten() {
            let Ok(files) = std::fs::read_dir(zone.path()) else {
                continue;
            };
            for file in files.flatten() {
                if file.path().extension().and_then(|x| x.to_str()) != Some("json") {
                    continue;
                }
                let Ok(source) = std::fs::read_to_string(file.path()) else {
                    continue;
                };
                let Ok(root) = serde_json::from_str::<JsonValue>(&source) else {
                    continue;
                };
                let Some(info) = root.get("몹정보").and_then(|v| v.as_object()) else {
                    continue;
                };
                let uses = info
                    .get("사용아이템")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(|| {
                        info.get("사용아이템")
                            .and_then(|v| v.as_str())
                            .map(|v| vec![JsonValue::String(v.to_string())])
                            .unwrap_or_default()
                    });
                for use_item in uses {
                    let Some(key) = use_item.as_str().and_then(|v| v.split_whitespace().next())
                    else {
                        continue;
                    };
                    let path = std::path::Path::new("data/item").join(format!("{key}.json"));
                    if path.exists() && seen_paths.insert(path.clone()) {
                        ordered_paths.push(path);
                    }
                }
            }
        }
    }
    for entry in item_files {
        let path = entry.path();
        if seen_paths.insert(path.clone()) {
            ordered_paths.push(path);
        }
    }
    for path in ordered_paths {
        let index = path.file_stem().and_then(|x| x.to_str()).unwrap_or("");
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(root) = serde_json::from_str::<JsonValue>(&source) else {
            continue;
        };
        let Some(info) = root.get("아이템정보").and_then(|v| v.as_object()) else {
            continue;
        };
        // Python Object.checkAttr uses `attr in keydata`: substring
        // membership for strings and exact element membership for lists.
        let is_one = match info.get("아이템속성") {
            Some(JsonValue::String(value)) => value.contains("단일아이템"),
            Some(JsonValue::Array(values)) => values
                .iter()
                .any(|value| value.as_str() == Some("단일아이템")),
            _ => false,
        };
        if !is_one {
            continue;
        }
        let name = info.get("이름").and_then(|v| v.as_str()).unwrap_or(index);
        let mut m = rhai::Map::new();
        m.insert("name".into(), rhai::Dynamic::from(name.to_string()));
        m.insert("index".into(), rhai::Dynamic::from(index.to_string()));
        arr.push(rhai::Dynamic::from(m));
    }
    // Keep legacy-index-only entries, which Python can materialize lazily via
    // getItem even when their JSON file is absent.
    if arr.is_empty() {
        for name in &guard.index_order {
            let Some(index) = guard.index.get(name) else {
                continue;
            };
            let mut m = rhai::Map::new();
            m.insert("name".into(), rhai::Dynamic::from(name.clone()));
            m.insert("index".into(), rhai::Dynamic::from(index.clone()));
            arr.push(rhai::Dynamic::from(m));
        }
    }
    arr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_and_name_lookup_follow_python_dict_insertion_order() {
        let mut state = OneitemState::new();
        state.index.insert("둘째기연".into(), "200".into());
        state.index_order.push("둘째기연".into());
        state.index.insert("첫기연".into(), "100".into());
        state.index_order.push("첫기연".into());
        state.attr.insert("100".into(), "첫소유자".into());
        state.attr_order.push("100".into());
        state.attr.insert("200".into(), "둘소유자 보관".into());
        state.attr_order.push("200".into());

        assert_eq!(state.get_name("100"), "첫기연");
        assert_eq!(
            state.list(),
            format!(
                "{:<16} ({:<16}) : 첫소유자\r\n{:<16} ({:<16}) : 둘소유자 보관\r\n",
                "첫기연", "100", "둘째기연", "200"
            )
        );
        assert_eq!(state.attr_keys(), vec!["100", "200"]);
    }
}
