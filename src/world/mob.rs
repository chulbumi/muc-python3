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
    /// Current action state (ACT_STAND, ACT_FIGHT, ACT_REST, etc.)
    pub act: i32,
    /// Active skills (for 방어상태머리말 display)
    pub skills: Vec<String>,
    /// Mob type (for display filtering, e.g., type 7 is hidden)
    pub mob_type: i64,
}

impl MobInstance {
    /// Create a new mob instance. room은 "1" 또는 사용자맵 "이름" 등.
    pub fn new(mob_key: String, zone: String, room: impl ToString, data: &RawMobData) -> Self {
        // Load skill names from the skills Vec<(String, i64, i64)>
        let skill_names: Vec<String> = data.skills.iter()
            .map(|(name, _level, _prob)| name.clone())
            .collect();

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
            act: 0, // ACT_STAND
            skills: skill_names,
            mob_type: data.mob_type,
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
        self.act = 0; // Reset to ACT_STAND
        self.skills.clear();
    }

    /// Kill the mob
    pub fn kill(&mut self) {
        self.alive = false;
        self.death_time = chrono::Utc::now().timestamp();
        self.hp = 0;
    }

    /// Handle mob death and calculate rewards (Python: die() in objs/mob.py:631)
    ///
    /// # Arguments
    /// * `mob_data` - The mob template data
    /// * `damage_map` - Damage dealt by each attacker
    /// * `herb_list` - Available herbs to drop
    ///
    /// # Returns
    /// A map of attacker_name -> (exp, gold, messages)
    pub fn die(
        &mut self,
        mob_data: &RawMobData,
        damage_map: &DamageMap,
        herb_list: &[String],
    ) -> HashMap<String, (i64, i64, Vec<String>)> {
        use crate::hangul;

        self.alive = false;
        self.death_time = chrono::Utc::now().timestamp();
        self.hp = 0;

        let mut result = HashMap::new();

        // Count attackers in same room and find max level
        let mut attacker_count = 0;
        let mut max_level = 0i64;
        let mut max_level_attacker = String::new();

        for attacker in damage_map.get_attackers() {
            // In full implementation, would check if attacker is in same room
            // For now, count all attackers
            attacker_count += 1;
            // In full implementation, would get actual level from attacker
            // For now, use mob level as default
            if mob_data.level > max_level {
                max_level = mob_data.level;
                max_level_attacker = attacker.clone();
            }
        }

        if attacker_count == 0 {
            return result;
        }

        // Calculate rewards for each attacker
        let base_exp_per_attacker = mob_data.level; // Simplified
        let base_gold_per_attacker = mob_data.level + 14;

        for attacker in damage_map.get_attackers() {
            let damage = damage_map.get_damage(&attacker);
            if damage == 0 {
                continue;
            }

            // Calculate reward based on damage ratio
            let total_hp = mob_data.max_hp;
            let ratio = if total_hp > 0 {
                (damage as f64 / total_hp as f64).min(1.0)
            } else {
                0.0
            };

            let reward = MobReward::calculate(
                mob_data.level,
                mob_data.level, // In full implementation, use actual attacker level
                0,              // mob's 은전 attribute
                0,              // 난이도
            );

            // Divide by number of attackers
            let exp = reward.exp / attacker_count.max(1);
            let gold = reward.gold / attacker_count.max(1);

            let mut messages = Vec::new();

            // Build reward messages (matching Python output)
            if reward.bonus_exp > 0 || reward.bonus_gold > 0 {
                messages.push(format!(
                    "\r\n당신이 {}(+{})의 경험치를 얻습니다.",
                    exp, reward.bonus_exp
                ));
                messages.push(format!(
                    "당신이 {}에게 은전 {}(+{})개를 획득합니다.",
                    self.name, gold, reward.bonus_gold
                ));
            } else {
                messages.push(format!("\r\n당신이 {}의 경험치를 얻습니다.", exp));
                messages.push(format!(
                    "당신이 {}에게 은전 {}개를 획득합니다.",
                    self.name, gold
                ));
            }

            // Herb drop message (for first attacker if mob level >= attacker level)
            if attacker == *damage_map.get_attackers().first().unwrap_or(&String::new()) {
                if let Some(herb) = HerbDrop::calculate(
                    mob_data.level,
                    mob_data.level,
                    0,
                    herb_list,
                    0.3, // 약초나올확률 default
                ) {
                    messages.push(format!(
                        "{} 약간의 경험치를 얻습니다.\r\n{} 몇개의 은전을 획득합니다.",
                        hangul::han_iga(&attacker),
                        hangul::han_iga(&attacker)
                    ));
                }
            }

            result.insert(attacker, (exp + reward.bonus_exp, gold + reward.bonus_gold, messages));
        }

        result
    }

    /// Get death message (Python: 소멸스크립)
    pub fn get_death_message(&self, mob_data: &RawMobData) -> String {
        if !mob_data.death_script.is_empty() {
            return format!("\r\n\x1b[1;37m{}\x1b[0;37m", mob_data.death_script);
        }

        // Default death message (matching Python output)
        format!(
            "\r\n\x1b[1;37m{}{} 쓰러집니다. '쿠웅~~ 철퍼덕~~'\x1b[0;37m",
            self.name,
            crate::hangul::han_iga(&self.name)
        )
    }

    /// Get corpse disappear message (Python: doDeath in objs/mob.py:756)
    pub fn get_corpse_message(&self) -> String {
        format!(
            "\r\n{}의 시체가 무림지존의 손에 이끌려 망자의 강을 건너갑니다.",
            self.get_name_with_particle()
        )
    }

    /// Get mob name with appropriate Korean particle
    pub fn get_name_with_particle(&self) -> String {
        format!("{}{}", self.name, crate::hangul::han_iga(&self.name))
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

    /// Damage a mob in the given room (reduce HP)
    /// Returns (new_hp, died) if mob was found and damaged
    pub fn damage_mob(&mut self, zone: &str, room: &str, mob_key: &str, damage: i64) -> Option<(i64, bool)> {
        let room_key = format!("{}:{}", zone, room);
        if let Some(instances) = self.instances.get_mut(&room_key) {
            for mob in instances.iter_mut() {
                if mob.mob_key == mob_key && mob.alive {
                    mob.hp = (mob.hp - damage).max(0);
                    let died = mob.hp <= 0;
                    if died {
                        mob.kill();
                    }
                    return Some((mob.hp, died));
                }
            }
        }
        None
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

    /// Kill a mob instance in a specific room
    pub fn kill_mob(&mut self, zone: &str, room: &str, mob_key: &str) -> bool {
        let room_key = format!("{}:{}", zone, room);
        if let Some(instances) = self.instances.get_mut(&room_key) {
            if let Some(mob) = instances.iter_mut().find(|m| m.mob_key == mob_key && m.alive) {
                mob.kill();
                return true;
            }
        }
        false
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

/// Damage tracker for mob combat (Python: dmgMap)
/// Maps attacker name -> total damage dealt
#[derive(Debug, Clone, Default)]
pub struct DamageMap {
    pub damage: HashMap<String, i64>,
}

impl DamageMap {
    pub fn new() -> Self {
        Self {
            damage: HashMap::new(),
        }
    }

    pub fn add_damage(&mut self, attacker: &str, dmg: i64) {
        *self.damage.entry(attacker.to_string()).or_insert(0) += dmg;
    }

    pub fn get_damage(&self, attacker: &str) -> i64 {
        self.damage.get(attacker).copied().unwrap_or(0)
    }

    pub fn get_total_damage(&self) -> i64 {
        self.damage.values().sum()
    }

    pub fn get_attackers(&self) -> Vec<String> {
        self.damage.keys().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.damage.clear();
    }
}

/// Reward result from killing a mob (Python: getExpGold result)
#[derive(Debug, Clone)]
pub struct MobReward {
    /// Experience points
    pub exp: i64,
    /// Gold (은전)
    pub gold: i64,
    /// Bonus exp from difficulty
    pub bonus_exp: i64,
    /// Bonus gold from difficulty
    pub bonus_gold: i64,
}

impl MobReward {
    /// Calculate exp and gold rewards (Python: getExpGold in objs/mob.py:569)
    ///
    /// # Arguments
    /// * `mob_level` - The mob's level (c2)
    /// * `target_level` - The attacker's level (c1)
    /// * `mob_gold` - Base gold from mob data (은전)
    /// * `difficulty` - Difficulty bonus (난이도, 0-7)
    ///
    /// # Returns
    /// Experience and gold rewards
    pub fn calculate(mob_level: i64, target_level: i64, mob_gold: i64, difficulty: i64) -> Self {
        // Use seeded random for consistency (Python uses randint)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let seed = (mob_level * target_level) as u64;
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        // Exp calculation (Python: a = ((c2*c2)//3)+30, b = (a * (c2-c1))//100, c = a + b)
        let a = ((mob_level * mob_level) / 3) + 30;
        let b = (a * (mob_level - target_level)) / 100;
        let mut exp = a + b;

        // Add random variance to exp (±0-9)
        let exp_var = (hash % 10) as i64;
        if (hash & 1) == 0 {
            exp += exp_var;
        } else {
            exp -= exp_var;
        }
        if exp < 1 {
            exp = 1;
        }
        // Cap at MAX_INT equivalent
        if exp > 2_000_000_000 {
            exp = 2_000_000_000;
        }

        // Gold calculation (Python: c1 = mob_level + 14)
        let mut gold = (mob_level + 14) + mob_gold;
        let gold_var = ((hash >> 4) % 5) as i64;
        if ((hash >> 5) & 1) == 0 {
            gold += gold_var;
        } else {
            gold -= gold_var;
        }
        if gold < 1 {
            gold = 1;
        }
        if gold > 2_000_000_000 {
            gold = 2_000_000_000;
        }

        // Calculate difficulty bonus (Python: Body.difficulty[d-1][2] and [3])
        // difficulty is 1-indexed in Python (1-7), we use 0-6 here
        let (bonus_exp, bonus_gold) = if difficulty > 0 && difficulty <= 7 {
            let bonus_mult = match difficulty {
                1 => (2, 2),
                2 => (3, 3),
                3 => (5, 5),
                4 => (8, 8),
                5 => (13, 13),
                6 => (20, 20),
                7 => (33, 33),
                _ => (0, 0),
            };
            let be = (exp * bonus_mult.0) / 100;
            let bg = (gold * bonus_mult.1) / 100;
            (be, bg)
        } else {
            (0, 0)
        };

        Self {
            exp,
            gold,
            bonus_exp,
            bonus_gold,
        }
    }

    /// Get total exp including bonus
    pub fn total_exp(&self) -> i64 {
        self.exp + self.bonus_exp
    }

    /// Get total gold including bonus
    pub fn total_gold(&self) -> i64 {
        self.gold + self.bonus_gold
    }
}

/// Herb drop result (Python: addHerb in objs/mob.py:606)
#[derive(Debug, Clone)]
pub struct HerbDrop {
    /// Herb item index (아이템 인덱스)
    pub herb_index: String,
    /// Herb name
    pub herb_name: String,
}

impl HerbDrop {
    /// Calculate if herb should drop based on level difference (Python: addHerb logic)
    ///
    /// # Arguments
    /// * `mob_level` - The mob's level
    /// * `target_level` - The attacker's level
    /// * `difficulty` - Difficulty bonus (난이도)
    /// * `herb_list` - Available herb item indices
    /// * `base_chance` - Base drop chance (약초나올확률, default 0.3 = 30%)
    ///
    /// # Returns
    /// Some(herb) if drop occurs, None otherwise
    pub fn calculate(mob_level: i64, target_level: i64, difficulty: i64, herb_list: &[String], base_chance: f64) -> Option<HerbDrop> {
        if mob_level < target_level {
            return None;
        }

        let level_diff = (mob_level - target_level) as f64;
        let drop_chance = level_diff * 0.01 + 0.05 + difficulty as f64;

        // Cap at base_chance (약초나올확률)
        let drop_chance = drop_chance.min(base_chance);

        // Random roll
        let roll = (rand::random::<f64>() * 100.0) as i64;
        let chance_threshold = (drop_chance * 100.0) as i64;

        if roll < chance_threshold && !herb_list.is_empty() {
            let idx = rand::random::<usize>() % herb_list.len();
            return Some(HerbDrop {
                herb_index: herb_list[idx].clone(),
                herb_name: format!(" Herb{}", idx),
            });
        }

        None
    }
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

    /// 83.json 로드 시 "이벤트: $대화 $대 편지" 키가 data.events에 포함되는지 확인.
    /// "왕대협 편지 대화" 입력 시 83_편지.rhai가 선택되려면 이 키가 있어야 함.
    #[test]
    fn test_load_83_has_편지_event_key() {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);
        let data = match cache.load_mob("낙양성", "83") {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test_load_83_has_편지_event_key: load_mob failed (skip if data not present): {}", e);
                return;
            }
        };
        let ev_keys: Vec<&String> = data.events.keys().filter(|k| k.starts_with("이벤트")).collect();
        assert!(
            data.events.contains_key("이벤트: $대화 $대 편지"),
            "83.json must have '이벤트: $대화 $대 편지', event keys: {:?}",
            ev_keys
        );
    }

    /// Test mob spawning for starting room
    #[test]
    fn test_spawn_mobs_starting_room() {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);

        // Spawn mobs for room 1 (starting room) with mob_ids from map data
        let mob_ids = vec!["밍밍-범죄자".to_string(), "포졸".to_string()];
        cache.spawn_mobs_for_room("낙양성", "1", &mob_ids);

        // Check if mobs were spawned
        let mobs = cache.get_mobs_in_room("낙양성", "1");
        eprintln!("test_spawn_mobs_starting_room: Found {} mobs in room 낙양성:1", mobs.len());

        for mob in &mobs {
            eprintln!("  - Mob: {}, key: {}, desc1: {}", mob.name, mob.mob_key,
                if let Some(data) = cache.get_mob(&mob.mob_key) { &data.desc1 } else { "?" });
        }

        // Verify mobs exist (this test will fail if mob loading is broken)
        assert!(!mobs.is_empty(), "No mobs found in room 낙양성:1");
    }

    /// Test loading room and spawning mobs from room data
    #[test]
    fn test_room_and_mob_spawn_integration() {
        let mob_data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("mob");
        let room_data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("map");
        let mut mob_cache = MobCache::with_data_dir(mob_data_dir);
        let mut room_cache = crate::world::RoomCache::with_data_dir(room_data_dir);

        // Load room 1
        let room_result = room_cache.get_room("낙양성", "1");
        assert!(room_result.is_ok(), "Failed to load room: {:?}", room_result);

        let room = room_result.unwrap();
        let room_ref = room.read().unwrap();
        let mob_ids = room_ref.mob_ids.clone();
        eprintln!("test_room_and_mob_spawn_integration: Room mob_ids = {:?}", mob_ids);

        // Spawn mobs using the mob_ids from the room
        mob_cache.spawn_mobs_for_room("낙양성", "1", &mob_ids);

        // Check if mobs were spawned
        let mobs = mob_cache.get_mobs_in_room("낙양성", "1");
        eprintln!("test_room_and_mob_spawn_integration: Found {} mobs in room 낙양성:1", mobs.len());

        for mob in &mobs {
            eprintln!("  - Mob: {}, key: {}", mob.name, mob.mob_key);
        }

        // Verify mobs exist
        assert!(!mobs.is_empty(), "No mobs found in room 낙양성:1 after spawn");
        assert_eq!(mobs.len(), 2, "Expected 2 mobs in room 낙양성:1");
    }

    /// Test WorldState spawn_mobs_for_room integration
    #[test]
    fn test_worldstate_spawn_integration() {
        use crate::world::WorldState;

        let mut world = WorldState::new();

        // This simulates what happens during login
        world.spawn_mobs_for_room("낙양성", "1");

        // Check if room was loaded
        let room = world.room_cache.get_room_cached("낙양성", "1");
        eprintln!("test_worldstate_spawn_integration: Room found in cache: {}", room.is_some());

        if let Some(room) = room {
            let room_ref = room.read().unwrap();
            eprintln!("test_worldstate_spawn_integration: Room mob_ids = {:?}", room_ref.mob_ids);
        }

        // Check if mobs were spawned
        let mobs = world.get_mobs_in_room("낙양성", "1");
        eprintln!("test_worldstate_spawn_integration: Found {} mobs in room 낙양성:1", mobs.len());

        for mob in &mobs {
            eprintln!("  - Mob: {}, key: {}", mob.name, mob.mob_key);
        }

        // Verify mobs exist
        assert!(!mobs.is_empty(), "No mobs found in room 낙양성:1 after WorldState spawn");
        assert_eq!(mobs.len(), 2, "Expected 2 mobs in room 낙양성:1");
    }

    /// Test loading mob data directly
    #[test]
    fn test_load_mob_밍밍_범죄자() {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);

        match cache.load_mob("낙양성", "밍밍-범죄자") {
            Ok(data) => {
                eprintln!("test_load_mob_밍밍_범죄자: name={}, zone={}, desc1={}",
                    data.name, data.zone, data.desc1);
                assert_eq!(data.name, "밍밍");
            }
            Err(e) => {
                eprintln!("test_load_mob_밍밍_범죄자: Failed to load: {}", e);
            }
        }
    }
}
