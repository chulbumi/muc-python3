//! Mob (Monster/NPC) module for MUD world
//!
//! This module provides mob loading and management functionality.
//! Mobs are loaded from JSON files in the data/mob/ directory.

use serde_json::Value as JsonValue;

/// 이벤트 스크립트: 배열(legacy)이거나 Rhai 파일명(문자열).
#[derive(Debug, Clone)]
pub enum EventScript {
    Legacy(Vec<String>),
    Rhai(String),
}
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Raw mob data from JSON
#[derive(Debug, Clone)]
pub struct RawMobData {
    /// Mob name (이름)
    pub name: String,
    /// Zone name (존이름)
    pub zone: String,
    /// Level (레벨)
    pub level: i64,
    /// HP (체력)
    pub hp: i64,
    /// Max HP (맷집 or 최대체력)
    pub max_hp: i64,
    /// Internal power (내공)
    pub inner_power: i64,
    /// Strength (힘)
    pub strength: i64,
    /// Agility (민첩성)
    pub agility: i64,
    /// Description 1 (short description for room display)
    pub desc1: String,
    /// Description 2 (long description when looking)
    pub desc2: Vec<String>,
    /// Description 3 (appearing message)
    pub desc3: String,
    /// Reaction names (aliases)
    pub reaction_names: Vec<String>,
    /// Spawn locations (위치)
    pub locations: Vec<i64>,
    /// Regen time in seconds (리젠)
    pub regen: i64,
    /// Talk tick (대화틱)
    pub talk_tick: i64,
    /// Mob type (몹종류)
    pub mob_type: i64,
    /// Personality (성격)
    pub personality: i64,
    /// Safe zone flag
    pub safe_zone: bool,
    /// Auto scripts (자동스크립)
    pub auto_scripts: Vec<String>,
    /// Events map (이벤트:...). 값이 배열이면 Legacy, 문자열이면 Rhai 스크립트 경로(예: "83_편지.rhai").
    pub events: HashMap<String, EventScript>,
    /// Items for sale (물건판매)
    pub items_for_sale: Vec<(String, i64)>,
    /// Sale menu script lines (물건판매스크립)
    pub sale_script: Vec<String>,
    /// Buy-from-player percent (물건구입 e.g. "고물상 40" -> 40)
    pub buy_percent: i64,
    /// Items mob uses (사용아이템)
    pub use_items: Vec<(String, i64, i64)>,
    /// Skills mob knows (무공)
    pub skills: Vec<(String, i64, i64)>,
    /// Death script (소멸스크립)
    pub death_script: String,
    /// Combat start script (전투시작)
    pub combat_start_script: String,
    /// Combat script (전투스크립)
    pub combat_script: String,
    /// HP display type (체력스크립)
    pub hp_display_type: String,
    /// Corpse drop time (시체)
    pub corpse_time: i64,
}

impl RawMobData {
    /// Create empty mob data
    pub fn new() -> Self {
        Self {
            name: String::new(),
            zone: String::new(),
            level: 1,
            hp: 100,
            max_hp: 100,
            inner_power: 0,
            strength: 10,
            agility: 10,
            desc1: String::new(),
            desc2: Vec::new(),
            desc3: String::new(),
            reaction_names: Vec::new(),
            locations: Vec::new(),
            regen: 300,
            talk_tick: 60,
            mob_type: 0,
            personality: 0,
            safe_zone: false,
            auto_scripts: Vec::new(),
            events: HashMap::new(),
            items_for_sale: Vec::new(),
            sale_script: Vec::new(),
            buy_percent: 0,
            use_items: Vec::new(),
            skills: Vec::new(),
            death_script: String::new(),
            combat_start_script: String::new(),
            combat_script: String::new(),
            hp_display_type: "사람".to_string(),
            corpse_time: 10,
        }
    }
}

impl Default for RawMobData {
    fn default() -> Self {
        Self::new()
    }
}

/// Active mob instance in the game world
#[derive(Debug, Clone)]
pub struct MobInstance {
    /// Original mob data key (zone:filename)
    pub mob_key: String,
    /// Current zone
    pub zone: String,
    /// Current room id ("1" or 사용자맵 "이름")
    pub room: String,
    /// Instance name (might have customization)
    pub name: String,
    /// Current HP
    pub hp: i64,
    /// Max HP
    pub max_hp: i64,
    /// Spawn timestamp
    pub spawn_time: i64,
    /// Death timestamp (0 if alive)
    pub death_time: i64,
    /// Is alive flag
    pub alive: bool,
    /// Current targets
    pub targets: Vec<String>,
}

impl MobInstance {
    /// Create a new mob instance. room은 "1" 또는 사용자맵 "이름" 등.
    pub fn new(mob_key: String, zone: String, room: impl ToString, data: &RawMobData) -> Self {
        Self {
            mob_key,
            zone,
            room: room.to_string(),
            name: data.name.clone(),
            hp: data.hp,
            max_hp: data.max_hp,
            spawn_time: chrono::Utc::now().timestamp(),
            death_time: 0,
            alive: true,
            targets: Vec::new(),
        }
    }

    /// Check if mob should respawn
    pub fn should_respawn(&self, regen_seconds: i64) -> bool {
        if self.alive {
            return false;
        }
        let now = chrono::Utc::now().timestamp();
        (now - self.death_time) >= regen_seconds
    }

    /// Respawn the mob
    pub fn respawn(&mut self, data: &RawMobData) {
        self.hp = data.max_hp;
        self.alive = true;
        self.death_time = 0;
        self.spawn_time = chrono::Utc::now().timestamp();
        self.targets.clear();
    }

    /// Kill the mob
    pub fn kill(&mut self) {
        self.alive = false;
        self.death_time = chrono::Utc::now().timestamp();
        self.hp = 0;
    }

    /// Get HP display string
    pub fn get_hp_display(&self, display_type: &str) -> String {
        if !self.alive {
            return "사체".to_string();
        }

        let ratio = self.hp as f64 / self.max_hp as f64;

        match display_type {
            "사람" => {
                if ratio > 0.8 {
                    "건강함".to_string()
                } else if ratio > 0.6 {
                    "약간 다침".to_string()
                } else if ratio > 0.4 {
                    "다침".to_string()
                } else if ratio > 0.2 {
                    "많이 다침".to_string()
                } else {
                    "죽어가는 중".to_string()
                }
            }
            "동물" => {
                if ratio > 0.7 {
                    "날쌔고 건장함".to_string()
                } else if ratio > 0.4 {
                    "쇠약해 보임".to_string()
                } else {
                    "죽어가는 중".to_string()
                }
            }
            _ => format!("{}/{}", self.hp, self.max_hp),
        }
    }
}

/// Mob cache for storing loaded mob templates
#[derive(Debug)]
pub struct MobCache {
    /// Cached mob data indexed by zone:filename
    mobs: HashMap<String, RawMobData>,
    /// Active mob instances indexed by zone:room
    instances: HashMap<String, Vec<MobInstance>>,
    /// Data directory path
    data_dir: PathBuf,
}

impl MobCache {
    /// Create a new mob cache
    pub fn new() -> Self {
        Self {
            mobs: HashMap::new(),
            instances: HashMap::new(),
            data_dir: PathBuf::from("data/mob"),
        }
    }

    /// Create a new mob cache with a custom data directory
    pub fn with_data_dir<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            mobs: HashMap::new(),
            instances: HashMap::new(),
            data_dir: PathBuf::from(data_dir.as_ref()),
        }
    }

    /// Get mob data by key (zone:filename)
    pub fn get_mob(&self, key: &str) -> Option<&RawMobData> {
        self.mobs.get(key)
    }

    /// Get mob data by zone and filename
    pub fn get_mob_by_zone(&self, zone: &str, filename: &str) -> Option<&RawMobData> {
        let key = format!("{}:{}", zone, filename);
        self.mobs.get(&key)
    }

    /// Load a mob from JSON file
    pub fn load_mob(&mut self, zone: &str, filename: &str) -> Result<RawMobData, MobError> {
        let key = format!("{}:{}", zone, filename);

        // Check cache first
        if let Some(data) = self.mobs.get(&key) {
            return Ok(data.clone());
        }

        // Build file path
        let file_path = self.data_dir.join(zone).join(format!("{}.json", filename));

        if !file_path.exists() {
            return Err(MobError::NotFound(format!("{}:{}", zone, filename)));
        }

        // Read and parse JSON
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| MobError::IoError(e.to_string()))?;

        let json: JsonValue = serde_json::from_str(&content)
            .map_err(|e| MobError::ParseError(e.to_string()))?;

        // Extract mob info
        let mob_info = json.get("몹정보")
            .and_then(|v| v.as_object())
            .ok_or_else(|| MobError::ParseError("몹정보 not found".to_string()))?;

        // Parse mob data
        let data = self.parse_mob_data(mob_info)?;

        // Cache it
        self.mobs.insert(key, data.clone());

        Ok(data)
    }

    /// Parse mob data from JSON object
    fn parse_mob_data(&self, mob_info: &serde_json::Map<String, JsonValue>) -> Result<RawMobData, MobError> {
        let mut data = RawMobData::new();

        // Name (이름)
        data.name = mob_info.get("이름")
            .and_then(|v| v.as_str())
            .unwrap_or("이름 없는 몹")
            .to_string();

        // Zone (존이름)
        data.zone = mob_info.get("존이름")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Level (레벨)
        data.level = mob_info.get("레벨")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        // HP (체력)
        data.hp = mob_info.get("체력")
            .and_then(|v| v.as_i64())
            .unwrap_or(100);
        data.max_hp = data.hp;

        // Also check for 맷집
        if let Some(hp) = mob_info.get("맷집").and_then(|v| v.as_i64()) {
            data.max_hp = hp;
            if data.hp == 100 {
                data.hp = hp;
            }
        }

        // Inner power (내공)
        data.inner_power = mob_info.get("내공")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Strength (힘)
        data.strength = mob_info.get("힘")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);

        // Agility (민첩성)
        data.agility = mob_info.get("민첩성")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);

        // Description 1 (설명1)
        data.desc1 = mob_info.get("설명1")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Description 2 (설명2)
        if let Some(desc2) = mob_info.get("설명2") {
            if let Some(arr) = desc2.as_array() {
                data.desc2 = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            } else if let Some(s) = desc2.as_str() {
                data.desc2 = vec![s.to_string()];
            }
        }

        // Description 3 (설명3)
        data.desc3 = mob_info.get("설명3")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Reaction names (반응이름)
        if let Some(names) = mob_info.get("반응이름") {
            if let Some(arr) = names.as_array() {
                data.reaction_names = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }

        // Spawn locations (위치)
        if let Some(locs) = mob_info.get("위치") {
            if let Some(arr) = locs.as_array() {
                for loc in arr {
                    if let Some(s) = loc.as_str() {
                        // Try to parse as room number
                        if let Ok(room) = s.parse::<i64>() {
                            data.locations.push(room);
                        }
                    } else if let Some(n) = loc.as_i64() {
                        data.locations.push(n);
                    }
                }
            }
        }

        // Regen time (리젠)
        data.regen = mob_info.get("리젠")
            .and_then(|v| v.as_i64())
            .unwrap_or(300);

        // Talk tick (대화틱)
        data.talk_tick = mob_info.get("대화틱")
            .and_then(|v| v.as_i64())
            .unwrap_or(60);

        // Mob type (몹종류)
        data.mob_type = mob_info.get("몹종류")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Personality (성격)
        data.personality = mob_info.get("성격")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Safe zone from properties
        if let Some(props) = mob_info.get("맵속성") {
            if let Some(arr) = props.as_array() {
                for prop in arr {
                    if let Some(s) = prop.as_str() {
                        if s.contains("안전지대") || s.contains("사용자전투금지") {
                            data.safe_zone = true;
                        }
                    }
                }
            }
        }

        // Auto scripts (자동스크립)
        if let Some(scripts) = mob_info.get("자동스크립") {
            if let Some(arr) = scripts.as_array() {
                data.auto_scripts = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }

        // Parse events (이벤트:...). 배열=Legacy, 문자열=Rhai 스크립트 파일명.
        for (key, value) in mob_info {
            if key.starts_with("이벤트") {
                if let Some(arr) = value.as_array() {
                    let event_scripts: Vec<String> = arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    data.events.insert(key.clone(), EventScript::Legacy(event_scripts));
                } else if let Some(s) = value.as_str() {
                    data.events.insert(key.clone(), EventScript::Rhai(s.to_string()));
                }
            }
        }

        // Items for sale (물건판매)
        if let Some(items) = mob_info.get("물건판매") {
            if let Some(arr) = items.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        let parts: Vec<&str> = s.split_whitespace().collect();
                        if parts.len() == 2 {
                            if let Ok(price) = parts[1].parse::<i64>() {
                                data.items_for_sale.push((parts[0].to_string(), price));
                            }
                        }
                    }
                }
            }
        }

        // Sale script / menu (물건판매스크립)
        if let Some(scr) = mob_info.get("물건판매스크립") {
            if let Some(arr) = scr.as_array() {
                data.sale_script = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            } else if let Some(s) = scr.as_str() {
                if !s.is_empty() {
                    data.sale_script = vec![s.to_string()];
                }
            }
        }

        // Buy-from-player (물건구입) e.g. "고물상 40" -> buy_percent=40
        if let Some(s) = mob_info.get("물건구입").and_then(|v| v.as_str()) {
            let w: Vec<&str> = s.split_whitespace().collect();
            if w.len() >= 2 {
                if let Ok(p) = w[1].parse::<i64>() {
                    data.buy_percent = p;
                }
            }
        }

        // Use items (사용아이템)
        if let Some(items) = mob_info.get("사용아이템") {
            if let Some(arr) = items.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        let parts: Vec<&str> = s.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let item_name = parts[0].to_string();
                            let count = parts[1].parse::<i64>().unwrap_or(1);
                            let prob = parts.get(2).and_then(|s| s.parse::<i64>().ok()).unwrap_or(100);
                            data.use_items.push((item_name, count, prob));
                        }
                    }
                }
            }
        }

        // Skills (무공)
        if let Some(skills) = mob_info.get("무공") {
            if let Some(arr) = skills.as_array() {
                for skill in arr {
                    if let Some(s) = skill.as_str() {
                        let parts: Vec<&str> = s.split_whitespace().collect();
                        if parts.len() >= 3 {
                            let skill_name = parts[0].to_string();
                            let level = parts[1].parse::<i64>().unwrap_or(100);
                            let prob = parts[2].parse::<i64>().unwrap_or(100);
                            data.skills.push((skill_name, level, prob));
                        }
                    }
                }
            }
        }

        // Death script (소멸스크립)
        data.death_script = mob_info.get("소멸스크립")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Combat start script (전투시작)
        data.combat_start_script = mob_info.get("전투시작")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Combat script (전투스크립)
        data.combat_script = mob_info.get("전투스크립")
            .and_then(|v| v.as_str())
            .unwrap_or("주먹")
            .to_string();

        // HP display type (체력스크립)
        data.hp_display_type = mob_info.get("체력스크립")
            .and_then(|v| v.as_str())
            .unwrap_or("사람")
            .to_string();

        // Corpse time (시체)
        data.corpse_time = mob_info.get("시체")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);

        Ok(data)
    }

    /// Preload all mobs in a zone
    pub fn preload_zone(&mut self, zone: &str) -> Result<usize, MobError> {
        let zone_dir = self.data_dir.join(zone);

        if !zone_dir.exists() {
            return Err(MobError::NotFound(zone.to_string()));
        }

        let entries = std::fs::read_dir(&zone_dir)
            .map_err(|e| MobError::IoError(e.to_string()))?;

        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| MobError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| MobError::ParseError("Invalid file name".to_string()))?;

                self.load_mob(zone, name)?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Spawn mobs for a room from map's `몹` list (on-demand load, no zone-wide preload). room은 "1" 또는 사용자맵 "이름".
    pub fn spawn_mobs_for_room(&mut self, zone: &str, room: &str, mob_ids: &[String]) {
        let room_key = format!("{}:{}", zone, room);

        for mob_id in mob_ids {
            let key = format!("{}:{}", zone, mob_id);

            // Load mob on demand if not cached
            if !self.mobs.contains_key(&key) {
                if self.load_mob(zone, mob_id).is_err() {
                    continue;
                }
            }

            let data = match self.mobs.get(&key) {
                Some(d) => d,
                None => continue,
            };

            let exists = self.instances.get(&room_key)
                .map(|instances| instances.iter().any(|m| m.alive && m.mob_key == key))
                .unwrap_or(false);

            if !exists {
                let instance = MobInstance::new(key.clone(), zone.to_string(), room, data);
                self.instances.entry(room_key.clone()).or_insert_with(Vec::new).push(instance);
            }
        }
    }

    /// Get active mobs in a room
    pub fn get_mobs_in_room(&self, zone: &str, room: &str) -> Vec<&MobInstance> {
        let room_key = format!("{}:{}", zone, room);
        self.instances.get(&room_key)
            .map(|instances| instances.iter().filter(|m| m.alive).collect())
            .unwrap_or_default()
    }

    /// Get mob data for an instance
    pub fn get_instance_data(&self, instance: &MobInstance) -> Option<&RawMobData> {
        self.get_mob(&instance.mob_key)
    }

    /// 리젠: 시체(이름)를 즉시 리젠. 시체만 가능. room에 있는 dead 몹 중 name 매칭(이름 또는 반응이름).
    pub fn do_regen(&mut self, zone: &str, room: &str, name: &str) -> bool {
        let room_key = format!("{}:{}", zone, room);
        let mut to_respawn = None::<(String, RawMobData)>;
        {
            let list = match self.instances.get(&room_key) {
                Some(v) => v,
                None => return false,
            };
            for inst in list.iter() {
                if !inst.alive {
                    let d = match self.get_mob(&inst.mob_key) {
                        Some(x) => x.clone(),
                        None => continue,
                    };
                    let ok = inst.name == name || inst.name.contains(name)
                        || d.reaction_names.iter().any(|n| n == name || n.contains(name));
                    if ok {
                        to_respawn = Some((inst.mob_key.clone(), d));
                        break;
                    }
                }
            }
        }
        let (mob_key, data) = match to_respawn {
            Some(x) => x,
            None => return false,
        };
        let list = match self.instances.get_mut(&room_key) {
            Some(v) => v,
            None => return false,
        };
        for inst in list.iter_mut() {
            if !inst.alive && inst.mob_key == mob_key {
                inst.respawn(&data);
                return true;
            }
        }
        false
    }

    /// Update respawn state for all instances
    pub fn update_respawns(&mut self) {
        let now = chrono::Utc::now().timestamp();

        // Collect all mob data needed for respawn
        let mut respawn_data: std::collections::HashMap<String, RawMobData> = std::collections::HashMap::new();

        for (_room_key, instances) in &self.instances {
            for instance in instances {
                if !instance.alive && !respawn_data.contains_key(&instance.mob_key) {
                    if let Some(data) = self.get_mob(&instance.mob_key) {
                        respawn_data.insert(instance.mob_key.clone(), data.clone());
                    }
                }
            }
        }

        // Now do the respawns
        for (_room_key, instances) in &mut self.instances {
            for instance in instances {
                if !instance.alive {
                    if let Some(data) = respawn_data.get(&instance.mob_key) {
                        if (now - instance.death_time) >= data.regen {
                            instance.respawn(data);
                        }
                    }
                }
            }
        }
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.mobs.clear();
        self.instances.clear();
    }

    /// Get the number of cached mob templates
    pub fn len(&self) -> usize {
        self.mobs.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.mobs.is_empty()
    }
}

impl Default for MobCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when working with mobs
#[derive(Debug, thiserror::Error)]
pub enum MobError {
    #[error("Mob not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Global mob cache accessor
pub fn get_mob_cache() -> &'static RwLock<MobCache> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<RwLock<MobCache>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(MobCache::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_mob_data_new() {
        let data = RawMobData::new();
        assert_eq!(data.level, 1);
        assert_eq!(data.hp, 100);
        assert!(data.locations.is_empty());
    }

    #[test]
    fn test_mob_instance_new() {
        let mut data = RawMobData::new();
        data.name = "Test Mob".to_string();
        data.hp = 200;
        data.max_hp = 200;

        let instance = MobInstance::new("zone:test".to_string(), "zone".to_string(), 1, &data);

        assert_eq!(instance.name, "Test Mob");
        assert_eq!(instance.hp, 200);
        assert_eq!(instance.max_hp, 200);
        assert!(instance.alive);
    }

    #[test]
    fn test_mob_instance_kill() {
        let mut data = RawMobData::new();
        data.name = "Test Mob".to_string();

        let mut instance = MobInstance::new("zone:test".to_string(), "zone".to_string(), 1, &data);

        assert!(instance.alive);
        assert_eq!(instance.hp, 100);

        instance.kill();

        assert!(!instance.alive);
        assert_eq!(instance.hp, 0);
        assert!(instance.death_time > 0);
    }

    #[test]
    fn test_mob_instance_respawn() {
        let mut data = RawMobData::new();
        data.name = "Test Mob".to_string();
        data.max_hp = 500;

        let mut instance = MobInstance::new("zone:test".to_string(), "zone".to_string(), 1, &data);

        instance.kill();
        assert!(!instance.alive);

        // Set death_time to past for immediate respawn
        instance.death_time = chrono::Utc::now().timestamp() - 400;

        instance.respawn(&data);

        assert!(instance.alive);
        assert_eq!(instance.hp, 500);
    }

    #[test]
    fn test_get_hp_display() {
        let mut data = RawMobData::new();
        data.name = "Test Mob".to_string();
        data.max_hp = 100;

        let mut instance = MobInstance::new("zone:test".to_string(), "zone".to_string(), 1, &data);

        // Test full health
        instance.hp = 100;
        assert_eq!(instance.get_hp_display("사람"), "건강함");

        // Test damaged
        instance.hp = 50;
        assert_eq!(instance.get_hp_display("사람"), "다침");

        // Test near death
        instance.hp = 10;
        assert_eq!(instance.get_hp_display("사람"), "죽어가는 중");

        // Test dead
        instance.kill();
        assert_eq!(instance.get_hp_display("사람"), "사체");
    }

    #[test]
    fn test_mob_cache_new() {
        let cache = MobCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_mob_cache_with_data_dir() {
        let cache = MobCache::with_data_dir("/custom/path");
        assert_eq!(cache.data_dir, PathBuf::from("/custom/path"));
    }
}
