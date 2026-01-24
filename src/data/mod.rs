//! Global Data Cache Module
//!
//! 게임 데이터를 글로벌하게 캐싱하고 접근하는 시스템입니다.
//! data/config/*.json 파일들을 로드하고, Rhai 스크립트에서 접근 가능합니다.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;
use tracing::{info, warn, error};

/// data/config/skill.json에서 스킬별 방어상태머리말 캐시. get_desc_for_look 등에서 사용.
static SKILL_DEFENSE_HEAD_CACHE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    if let Ok(s) = std::fs::read_to_string("data/config/skill.json") {
        if let Ok(v) = serde_json::from_str::<JsonValue>(&s) {
            if let Some(obj) = v.as_object() {
                for (k, skill) in obj {
                    if let Some(val) = skill.get("방어상태머리말").and_then(|v| v.as_str()) {
                        m.insert(k.clone(), val.to_string());
                    }
                }
            }
        }
    }
    RwLock::new(m)
});

/// 스킬 이름에 해당하는 방어상태머리말. 파이썬 getDesc의 for s in self.skills: s['방어상태머리말'].
/// data/config/skill.json을 로드해 캐시함. 없으면 "".
pub fn get_skill_defense_head(skill_name: &str) -> String {
    SKILL_DEFENSE_HEAD_CACHE
        .read()
        .unwrap()
        .get(skill_name)
        .cloned()
        .unwrap_or_default()
}

/// 글로벌 데이터 캐시
///
/// data/config/*.json 파일들의 데이터를 저장합니다.
#[derive(Debug)]
pub struct GlobalData {
    /// JSON 파일 데이터 캐시 (파일명 -> JSON 데이터)
    data: HashMap<String, JsonValue>,
    /// 데이터 디렉토리 경로
    data_dir: PathBuf,
}

impl GlobalData {
    /// 새로운 글로벌 데이터 캐시를 생성합니다.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data: HashMap::new(),
            data_dir,
        }
    }

    /// 모든 JSON 파일을 로드합니다.
    pub fn load_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = self.data_dir.join("config");

        if !config_dir.exists() {
            warn!("Config directory does not exist: {:?}", config_dir);
            return Ok(());
        }

        let entries = std::fs::read_dir(&config_dir)?;
        let mut loaded_count = 0;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // .json 파일만 로드
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // 파일명에서 확장자 제거
            let file_name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // JSON 로드
            match self.load_file(&file_name, &path) {
                Ok(_) => {
                    loaded_count += 1;
                }
                Err(e) => {
                    error!("Failed to load {}: {}", file_name, e);
                }
            }
        }

        info!("Loaded {} config files from {:?}", loaded_count, config_dir);
        Ok(())
    }

    /// 단일 JSON 파일을 로드합니다.
    fn load_file(&mut self, name: &str, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let json: JsonValue = serde_json::from_str(&content)?;

        self.data.insert(name.to_string(), json);
        info!("Loaded config: {}", name);
        Ok(())
    }

    /// 특정 파일을 다시 로드합니다 (핫리로드).
    pub fn reload(&mut self, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let config_dir = self.data_dir.join("config");
        let json_path = config_dir.join(format!("{}.json", name));

        if !json_path.exists() {
            return Ok(false);
        }

        self.load_file(name, &json_path)?;
        info!("Reloaded config: {}", name);
        Ok(true)
    }

    /// 모든 파일을 다시 로드합니다 (핫리로드).
    pub fn reload_all(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let names: Vec<String> = self.data.keys().cloned().collect();
        let mut reloaded = 0;

        for name in names {
            if self.reload(&name)? {
                reloaded += 1;
            }
        }

        info!("Reloaded {} config files", reloaded);
        Ok(reloaded)
    }

    /// 데이터를 가져옵니다.
    pub fn get(&self, name: &str) -> Option<&JsonValue> {
        self.data.get(name)
    }

    /// 데이터를 가져옵니다 (값 복사).
    pub fn get_clone(&self, name: &str) -> Option<JsonValue> {
        self.data.get(name).cloned()
    }

    /// 특정 경로의 데이터를 가져옵니다.
    /// 예: get_path("skill", "가의신공")
    pub fn get_path(&self, file: &str, key: &str) -> Option<&JsonValue> {
        self.data.get(file)?.get(key)
    }

    /// skill.json에서 스킬 데이터를 가져옵니다.
    pub fn get_skill(&self, name: &str) -> Option<&JsonValue> {
        self.get_path("skill", name)
    }

    /// murim.json에서 설정을 가져옵니다.
    pub fn get_murim_config(&self, key: &str) -> Option<&JsonValue> {
        self.get_path("murim", key)
    }

    /// mappath.json에서 맵 경로를 가져옵니다.
    pub fn get_map_path(&self, zone: &str) -> Option<&JsonValue> {
        self.get_path("mappath", zone)
            .or_else(|| self.get_path("mappath", "디렉토리설정")?.get(zone))
    }

    /// 데이터에 키가 있는지 확인합니다.
    pub fn contains(&self, name: &str) -> bool {
        self.data.contains_key(name)
    }

    /// 특정 파일에 키가 있는지 확인합니다.
    pub fn contains_key(&self, file: &str, key: &str) -> bool {
        self.data.get(file)
            .and_then(|v| v.as_object())
            .map(|obj| obj.contains_key(key))
            .unwrap_or(false)
    }

    /// 모든 파일 이름을 가져옵니다.
    pub fn file_names(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// 특정 파일의 모든 키를 가져옵니다.
    pub fn keys(&self, file: &str) -> Vec<String> {
        self.data.get(file)
            .and_then(|v| v.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// 공유 글로벌 데이터
pub type SharedGlobalData = Arc<RwLock<GlobalData>>;

/// 공유 글로벌 데이터를 생성합니다.
pub fn create_global_data(data_dir: PathBuf) -> SharedGlobalData {
    let mut global_data = GlobalData::new(data_dir);

    if let Err(e) = global_data.load_all() {
        warn!("Failed to load initial config data: {}", e);
    }

    Arc::new(RwLock::new(global_data))
}

/// Rhai 스크립트에서 사용할 수 있도록 JsonValue를 Dynamic으로 변환합니다.
pub fn json_to_dynamic(value: &JsonValue) -> rhai::Dynamic {
    match value {
        JsonValue::Null => rhai::Dynamic::UNIT,
        JsonValue::Bool(b) => rhai::Dynamic::from(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                rhai::Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                rhai::Dynamic::from(f as i64)
            } else {
                rhai::Dynamic::UNIT
            }
        }
        JsonValue::String(s) => rhai::Dynamic::from(s.clone()),
        JsonValue::Array(arr) => {
            let rhai_arr: rhai::Array = arr.iter()
                .map(json_to_dynamic)
                .collect();
            rhai::Dynamic::from(rhai_arr)
        }
        JsonValue::Object(obj) => {
            let rhai_map: rhai::Map = obj.iter()
                .map(|(k, v)| (k.clone().into(), json_to_dynamic(v)))
                .collect();
            rhai::Dynamic::from(rhai_map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_data_new() {
        let data = GlobalData::new(PathBuf::from("data"));
        assert_eq!(data.data.len(), 0);
    }

    #[test]
    fn test_global_data_contains() {
        let data = GlobalData::new(PathBuf::from("data"));
        assert!(!data.contains("nonexistent"));
    }

    #[test]
    fn test_json_to_dynamic_null() {
        let val = JsonValue::Null;
        let dynamic = json_to_dynamic(&val);
        assert!(dynamic.is_unit());
    }

    #[test]
    fn test_json_to_dynamic_bool() {
        let val = JsonValue::Bool(true);
        let dynamic = json_to_dynamic(&val);
        assert!(dynamic.is_bool());
    }

    #[test]
    fn test_json_to_dynamic_number() {
        let val = JsonValue::Number(serde_json::Number::from(42));
        let dynamic = json_to_dynamic(&val);
        assert!(dynamic.is_int());
    }

    #[test]
    fn test_json_to_dynamic_string() {
        let val = JsonValue::String("test".to_string());
        let dynamic = json_to_dynamic(&val);
        assert!(dynamic.is_string());
    }

    #[test]
    fn test_json_to_dynamic_array() {
        let val = JsonValue::Array(vec![JsonValue::Number(1.into()), JsonValue::Number(2.into())]);
        let dynamic = json_to_dynamic(&val);
        assert!(dynamic.is_array());
    }

    #[test]
    fn test_json_to_dynamic_object() {
        let mut obj = serde_json::Map::new();
        obj.insert("key".to_string(), JsonValue::String("value".to_string()));
        let val = JsonValue::Object(obj);
        let dynamic = json_to_dynamic(&val);
        assert!(dynamic.is_map());
    }
}
