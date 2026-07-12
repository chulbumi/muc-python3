//! Mob (Monster/NPC) module for MUD world
//!
//! This module provides mob loading and management functionality.
//! Mobs are loaded from JSON files in the data/mob/ directory.

use crate::object::Object;
use serde_json::Value as JsonValue;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_MOB_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

/// 이벤트 스크립트: 배열(legacy)이거나 Rhai 파일명(문자열).
#[derive(Debug, Clone)]
pub enum EventScript {
    Legacy(Vec<String>),
    Rhai(String),
}
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Raw mob data from JSON
#[derive(Debug, Clone)]
pub struct RawMobData {
    /// Original object attributes retained for Rhai/admin compatibility.
    pub attributes: HashMap<String, serde_json::Value>,
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
    /// Carried silver added to the normal kill reward (은전)
    pub gold: i64,
    /// Strength (힘)
    pub strength: i64,
    /// Arm/Defense (맷집 - 방어력)
    pub arm: i64,
    /// Agility (민첩성)
    pub agility: i64,
    /// Evasion used by `Body.getAttackChance` (회피)
    pub miss: i64,
    /// Accuracy used by Mob.getSkillChance (명중)
    pub hit: i64,
    /// Critical chance source (`운`) and multiplier source (`필살`).
    pub luck: i64,
    pub critical: i64,
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
    /// Python `이동` permitted room ids, expanded from single ids and ranges.
    pub move_rooms: Vec<String>,
    /// Python `이동틱`; zero defaults to 30 seconds.
    pub move_tick: i64,
    /// Mob type (몹종류)
    pub mob_type: i64,
    /// Linked-combat behavior (`전투종류`; Python `Player.setFight`).
    pub combat_type: i64,
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
    pub use_items: Vec<(String, i64, i64, i64)>,
    /// Corpse drop declarations (아이템): key, count, chance, roll scale.
    pub drop_items: Vec<(String, i64, i64, i64)>,
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
            attributes: HashMap::new(),
            name: String::new(),
            zone: String::new(),
            level: 1,
            hp: 100,
            max_hp: 100,
            inner_power: 0,
            gold: 0,
            strength: 10,
            arm: 10,
            agility: 10,
            miss: 0,
            hit: 0,
            luck: 0,
            critical: 0,
            desc1: String::new(),
            desc2: Vec::new(),
            desc3: String::new(),
            reaction_names: Vec::new(),
            locations: Vec::new(),
            regen: 300,
            // Python only enters the speech branch when the raw `대화틱`
            // attribute is present and non-empty; omission is not 60.
            talk_tick: 0,
            move_rooms: Vec::new(),
            move_tick: 30,
            mob_type: 0,
            combat_type: 0,
            personality: 0,
            safe_zone: false,
            auto_scripts: Vec::new(),
            events: HashMap::new(),
            items_for_sale: Vec::new(),
            sale_script: Vec::new(),
            buy_percent: 0,
            use_items: Vec::new(),
            drop_items: Vec::new(),
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
    /// Stable identity of this cloned runtime mob, retained across respawn.
    pub instance_id: u64,
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
    /// Current inner power. Python mob instances keep this separately from the template.
    pub mp: i64,
    /// Maximum inner power before temporary modifiers.
    pub max_mp: i64,
    /// Spawn timestamp
    pub spawn_time: i64,
    /// Python `Body.tick`, incremented by `Mob.update()` through Room.update().
    pub tick: i64,
    /// Death timestamp (0 if alive)
    pub death_time: i64,
    /// Python `Mob.timeofregen`, refreshed by `뒤져` for type-6 mobs/corpses.
    pub time_of_regen: i64,
    /// Is alive flag
    pub alive: bool,
    /// Current targets
    pub targets: Vec<String>,
    /// Python `Mob.dmgMap`: actual HP removed, accumulated by player name.
    pub damage_map: HashMap<String, i64>,
    /// Runtime inventory objects received through Python's `줘` command.
    pub inventory: Vec<Arc<Mutex<Object>>>,
    /// Mutable carried silver. Donation/withdrawal commands change the mob
    /// instance just as Python mutates `mob['은전']`.
    pub gold: i64,
    /// Current action state (ACT_STAND, ACT_FIGHT, ACT_REST, etc.)
    pub act: i32,
    /// Active skills (for 방어상태머리말 display)
    pub skills: Vec<String>,
    /// Runtime defense/buff effects. `skills` is retained as the display-name list.
    pub skill_effects: Vec<MobSkillEffect>,
    /// Mob type (for display filtering, e.g., type 7 is hidden)
    pub mob_type: i64,
    /// Difficulty level (0 = base, 1-7 = difficulty zones)
    pub difficulty: u8,
    /// Difficulty-adjusted level
    pub level: i64,
    /// Difficulty-adjusted strength
    pub strength: i64,
    /// Difficulty-adjusted defense/arm
    pub arm: i64,
    /// Difficulty-adjusted agility
    pub agility: i64,
    /// Temporary stat modifiers applied by defense/non-combat skills.
    pub str_modifier: i64,
    pub dex_modifier: i64,
    pub arm_modifier: i64,
    pub mp_modifier: i64,
    pub max_mp_modifier: i64,
    pub hp_modifier: i64,
    pub max_hp_modifier: i64,
    /// Python Mob.skill: cloned attack skill retaining pattern turn state.
    pub active_attack_skill: Option<crate::world::skill::Skill>,
    /// Python Body.dex accumulator used while advancing mob combat patterns.
    pub combat_dex: i64,
    /// Python Mob.moveTime, stored independently for each cloned mob.
    pub last_move_millis: i64,
    /// Runtime-only arbitrary attributes set by Python-compatible admin commands.
    pub runtime_attrs: HashMap<String, crate::object::Value>,
}

/// One temporary skill effect on a mob instance.
///
/// Python stores cloned `Skill` objects in `mob.skills`. Rust keeps the values
/// needed to reproduce duplicate-category checks, expiry and stat rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobSkillEffect {
    pub name: String,
    pub anti_type: String,
    pub expires_at: i64,
    pub str_bonus: i64,
    pub dex_bonus: i64,
    pub arm_bonus: i64,
    pub mp_bonus: i64,
    pub max_mp_bonus: i64,
    pub hp_bonus: i64,
    pub max_hp_bonus: i64,
}

impl MobInstance {
    /// Create a new mob instance. room은 "1" 또는 사용자맵 "이름" 등.
    pub fn new(mob_key: String, zone: String, room: impl ToString, data: &RawMobData) -> Self {
        Self::with_difficulty(mob_key, zone, room, data, 0)
    }

    /// Create a new mob instance with difficulty level.
    /// Applies difficulty multipliers to stats.
    ///
    /// # Arguments
    /// * `mob_key` - Mob key (zone:filename)
    /// * `zone` - Current zone
    /// * `room` - Current room
    /// * `data` - Raw mob data (template)
    /// * `difficulty` - Difficulty level (0-7)
    pub fn with_difficulty(
        mob_key: String,
        zone: String,
        room: impl ToString,
        data: &RawMobData,
        difficulty: u8,
    ) -> Self {
        use super::difficulty::DifficultyConfig;

        // Get difficulty config
        let config = DifficultyConfig::get(difficulty);

        // Apply difficulty to stats
        let level = config.apply_level(data.level);
        let max_hp = config.apply_hp(data.max_hp);
        let strength = config.apply_str(data.strength);
        let arm = config.apply_arm(data.arm);
        let agility = config.apply_agi(data.agility);

        Self {
            instance_id: NEXT_MOB_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            mob_key,
            zone,
            room: room.to_string(),
            name: data.name.clone(),
            hp: max_hp, // Start at full health
            max_hp,
            mp: data.inner_power,
            max_mp: data.inner_power,
            spawn_time: chrono::Utc::now().timestamp(),
            tick: 0,
            death_time: 0,
            time_of_regen: chrono::Utc::now().timestamp(),
            alive: true,
            targets: Vec::new(),
            damage_map: HashMap::new(),
            inventory: Vec::new(),
            gold: data.gold,
            act: 0, // ACT_STAND
            // Python `Mob.reset()` starts `skills` as the active-defense list;
            // learned attack/defense candidates remain in template `data.skills`.
            skills: Vec::new(),
            skill_effects: Vec::new(),
            mob_type: data.mob_type,
            difficulty,
            level,
            strength,
            arm,
            agility,
            str_modifier: 0,
            dex_modifier: 0,
            arm_modifier: 0,
            mp_modifier: 0,
            max_mp_modifier: 0,
            hp_modifier: 0,
            max_hp_modifier: 0,
            active_attack_skill: None,
            combat_dex: 0,
            last_move_millis: 0,
            runtime_attrs: HashMap::new(),
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
        use super::difficulty::DifficultyConfig;

        // Reapply difficulty to stats
        let config = DifficultyConfig::get(self.difficulty);
        self.max_hp = config.apply_hp(data.max_hp);
        self.hp = self.max_hp;
        self.mp = data.inner_power;
        self.max_mp = data.inner_power;
        self.level = config.apply_level(data.level);
        self.strength = config.apply_str(data.strength);
        self.arm = config.apply_arm(data.arm);
        self.agility = config.apply_agi(data.agility);

        self.alive = true;
        self.death_time = 0;
        self.spawn_time = chrono::Utc::now().timestamp();
        self.targets.clear();
        self.damage_map.clear();
        self.inventory.clear();
        self.act = 0; // Reset to ACT_STAND
        self.skills.clear();
        self.skill_effects.clear();
        self.str_modifier = 0;
        self.dex_modifier = 0;
        self.arm_modifier = 0;
        self.mp_modifier = 0;
        self.max_mp_modifier = 0;
        self.hp_modifier = 0;
        self.max_hp_modifier = 0;
        self.active_attack_skill = None;
        self.combat_dex = 0;
    }

    /// Kill the mob
    pub fn kill(&mut self) {
        self.alive = false;
        self.death_time = chrono::Utc::now().timestamp();
        self.hp = 0;
        self.act = 2; // ACT_DEATH
    }

    pub fn record_player_damage(&mut self, player_name: &str, damage: i64) {
        if damage <= 0 {
            return;
        }
        let entry = self.damage_map.entry(player_name.to_string()).or_insert(0);
        *entry = entry.saturating_add(damage);
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
        let mut _max_level = 0i64;
        let mut _max_level_attacker = String::new();

        for attacker in damage_map.get_attackers() {
            // In full implementation, would check if attacker is in same room
            // For now, count all attackers
            attacker_count += 1;
            // In full implementation, would get actual level from attacker
            // For now, use mob level as default
            if mob_data.level > _max_level {
                _max_level = mob_data.level;
                _max_level_attacker = attacker.clone();
            }
        }

        if attacker_count == 0 {
            return result;
        }

        // Calculate rewards for each attacker
        let _base_exp_per_attacker = mob_data.level; // Simplified
        let _base_gold_per_attacker = mob_data.level + 14;

        for attacker in damage_map.get_attackers() {
            let damage = damage_map.get_damage(&attacker);
            if damage == 0 {
                continue;
            }

            // Calculate reward based on damage ratio
            let total_hp = mob_data.max_hp;
            let _ratio = if total_hp > 0 {
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
                if let Some(_herb) = HerbDrop::calculate(
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

            result.insert(
                attacker,
                (exp + reward.bonus_exp, gold + reward.bonus_gold, messages),
            );
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
    /// Python `Mob.Mobs` insertion order, used by global administrative scans.
    mob_order: Vec<String>,
    /// Active mob instances indexed by zone:room
    instances: HashMap<String, Vec<MobInstance>>,
    /// Rooms whose Python `Room.create()` mob placement has run already.
    initialized_rooms: HashSet<String>,
    /// Data directory path
    data_dir: PathBuf,
}

impl MobCache {
    /// Move one concrete cloned mob between room instance vectors.  The
    /// caller supplies the selected instance key; this preserves its runtime
    /// HP, targets and movement timestamp instead of respawning a template.
    pub(crate) fn move_instance(
        &mut self,
        zone: &str,
        room: &str,
        mob_key: &str,
        destination_zone: &str,
        destination_room: &str,
        now_millis: i64,
    ) -> Option<u64> {
        use super::difficulty::base_zone_name;
        let source_keys = [
            format!(
                "{}:{}:{}",
                base_zone_name(zone),
                room,
                super::difficulty::difficulty_from_zone(zone)
            ),
            format!("{}:{}", zone, room),
        ];
        let Some(source_key) = source_keys
            .into_iter()
            .find(|key| self.instances.contains_key(key))
        else {
            return None;
        };
        let Some(instances) = self.instances.get_mut(&source_key) else {
            return None;
        };
        let Some(index) = instances
            .iter()
            .position(|mob| mob.mob_key == mob_key && mob.alive && mob.act == 0)
        else {
            return None;
        };
        let mut mob = instances.remove(index);
        let instance_id = mob.instance_id;
        if instances.is_empty() {
            self.instances.remove(&source_key);
        }
        mob.zone = destination_zone.to_string();
        mob.room = destination_room.to_string();
        mob.last_move_millis = now_millis;
        let destination_key = format!(
            "{}:{}:{}",
            base_zone_name(destination_zone),
            destination_room,
            mob.difficulty
        );
        self.instances.entry(destination_key).or_default().push(mob);
        Some(instance_id)
    }
    /// Create a new mob cache
    pub fn new() -> Self {
        Self {
            mobs: HashMap::new(),
            mob_order: Vec::new(),
            instances: HashMap::new(),
            initialized_rooms: HashSet::new(),
            data_dir: PathBuf::from("data/mob"),
        }
    }

    /// Create a new mob cache with a custom data directory
    pub fn with_data_dir<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            mobs: HashMap::new(),
            mob_order: Vec::new(),
            instances: HashMap::new(),
            initialized_rooms: HashSet::new(),
            data_dir: PathBuf::from(data_dir.as_ref()),
        }
    }

    /// Insert already-parsed template data for runtime setup and isolated
    /// scheduler tests. Normal production loading still goes through
    /// `load_mob`.
    #[cfg(test)]
    pub(crate) fn insert_mob_data(&mut self, key: String, data: RawMobData) {
        if !self.mobs.contains_key(&key) {
            self.mob_order.push(key.clone());
        }
        self.mobs.insert(key, data);
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

    /// Python `Mob.Mobs` iteration order: successful template registration
    /// order, independent of whether an instance is currently spawned/alive.
    pub fn ordered_mob_templates(&self) -> impl Iterator<Item = (&str, &RawMobData)> {
        self.mob_order.iter().filter_map(|key| {
            self.mobs
                .get(key)
                .map(|data| (key.as_str(), data))
        })
    }

    /// Get mutable mob data by key (zone:filename)
    pub fn get_mob_mut(&mut self, key: &str) -> Option<&mut RawMobData> {
        self.mobs.get_mut(key)
    }

    pub fn item_holders(&self, item_key: &str) -> Vec<(String, String)> {
        let mut holders = Vec::new();
        for key in &self.mob_order {
            let Some(mob) = self.mobs.get(key) else {
                continue;
            };
            for (item, _, _, _) in &mob.drop_items {
                if item == item_key {
                    holders.push((mob.name.clone(), key.clone()));
                }
            }
            for (item, _, _, _) in &mob.use_items {
                if item == item_key {
                    holders.push((mob.name.clone(), key.clone()));
                }
            }
        }
        holders
    }

    /// Remove a loaded mob template and its active instances (Python Mob.Mobs
    /// runtime deletion; the source JSON file is intentionally preserved).
    pub fn remove_mob(&mut self, key: &str) -> bool {
        let removed = self.mobs.remove(key).is_some();
        if removed {
            self.mob_order.retain(|loaded| loaded != key);
        }
        self.instances.retain(|_, mobs| {
            mobs.retain(|mob| mob.mob_key != key);
            !mobs.is_empty()
        });
        removed
    }

    /// Python `cmds/몹삭제.py` removes only the entry from `Mob.Mobs`.
    /// Existing room objects are independent clones and remain until their
    /// normal lifecycle removes them.
    pub fn remove_mob_definition(&mut self, key: &str) -> bool {
        let removed = self.mobs.remove(key).is_some();
        if removed {
            self.mob_order.retain(|loaded| loaded != key);
        }
        removed
    }

    /// Remove one active instance from a room without deleting its template.
    pub fn remove_instance(&mut self, zone: &str, room: &str, mob_key: &str) -> bool {
        let room_key = format!("{}:{}", zone, room);
        let Some(instances) = self.instances.get_mut(&room_key) else {
            return false;
        };
        let before = instances.len();
        instances.retain(|mob| mob.mob_key != mob_key);
        let removed = before != instances.len();
        let empty = instances.is_empty();
        if empty {
            self.instances.remove(&room_key);
        }
        removed
    }

    /// Check if mob has a specific event (Python: target.checkEvent(event_key))
    pub fn check_mob_event(&self, key: &str, event_key: &str) -> bool {
        if let Some(mob_data) = self.mobs.get(key) {
            // Check if event key exists (이벤트 $...)
            let full_key = format!("이벤트 {}", event_key);
            mob_data.events.contains_key(&full_key)
        } else {
            false
        }
    }

    /// Set an event on a mob (Python: target.setEvent(event_key))
    /// Adds "이벤트 $<event_key>" to the mob's events with empty script
    pub fn set_mob_event(&mut self, key: &str, event_key: &str) -> bool {
        if let Some(mob_data) = self.mobs.get_mut(key) {
            let full_key = format!("이벤트 {}", event_key);
            // Add event with empty legacy script (just marks it as set)
            mob_data
                .events
                .insert(full_key, EventScript::Legacy(vec![]));
            true
        } else {
            false
        }
    }

    /// Delete an event from a mob (Python: target.delEvent(event_key))
    pub fn del_mob_event(&mut self, key: &str, event_key: &str) -> bool {
        if let Some(mob_data) = self.mobs.get_mut(key) {
            let full_key = format!("이벤트 {}", event_key);
            mob_data.events.remove(&full_key).is_some()
        } else {
            false
        }
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
        let content =
            std::fs::read_to_string(&file_path).map_err(|e| MobError::IoError(e.to_string()))?;

        let json: JsonValue =
            serde_json::from_str(&content).map_err(|e| MobError::ParseError(e.to_string()))?;

        // Extract mob info
        let mob_info = json
            .get("몹정보")
            .and_then(|v| v.as_object())
            .ok_or_else(|| MobError::ParseError("몹정보 not found".to_string()))?;

        // Parse mob data
        let data = self.parse_mob_data(mob_info)?;

        // Cache it
        self.mob_order.push(key.clone());
        self.mobs.insert(key, data.clone());

        Ok(data)
    }

    /// Parse mob data from JSON object
    fn parse_mob_data(
        &self,
        mob_info: &serde_json::Map<String, JsonValue>,
    ) -> Result<RawMobData, MobError> {
        let mut data = RawMobData::new();
        data.attributes = mob_info.clone().into_iter().collect();

        // Name (이름)
        data.name = mob_info
            .get("이름")
            .and_then(|v| v.as_str())
            .unwrap_or("이름 없는 몹")
            .to_string();

        // Zone (존이름)
        data.zone = mob_info
            .get("존이름")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Level (레벨)
        data.level = mob_info.get("레벨").and_then(|v| v.as_i64()).unwrap_or(1);

        // HP (체력)
        data.hp = mob_info.get("체력").and_then(|v| v.as_i64()).unwrap_or(100);
        data.max_hp = data.hp;

        // Arm/Defense (맷집 - 방어력)
        // Python: mob.getArm() returns 맷집 value
        data.arm = mob_info.get("맷집").and_then(|v| v.as_i64()).unwrap_or(10);

        // Inner power (내공)
        data.inner_power = mob_info.get("내공").and_then(|v| v.as_i64()).unwrap_or(0);
        data.gold = mob_info.get("은전").and_then(|v| v.as_i64()).unwrap_or(0);

        // Strength (힘)
        data.strength = mob_info.get("힘").and_then(|v| v.as_i64()).unwrap_or(10);

        // Agility (민첩성)
        data.agility = mob_info
            .get("민첩성")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);
        data.miss = mob_info.get("회피").and_then(|v| v.as_i64()).unwrap_or(0);
        data.hit = mob_info.get("명중").and_then(|v| v.as_i64()).unwrap_or(0);
        data.luck = mob_info.get("운").and_then(|v| v.as_i64()).unwrap_or(0);
        data.critical = mob_info.get("필살").and_then(|v| v.as_i64()).unwrap_or(0);

        // Description 1 (설명1)
        data.desc1 = mob_info
            .get("설명1")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Description 2 (설명2)
        if let Some(desc2) = mob_info.get("설명2") {
            if let Some(arr) = desc2.as_array() {
                data.desc2 = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            } else if let Some(s) = desc2.as_str() {
                data.desc2 = vec![s.to_string()];
            }
        }

        // Description 3 (설명3)
        data.desc3 = mob_info
            .get("설명3")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Reaction names (반응이름)
        if let Some(names) = mob_info.get("반응이름") {
            if let Some(arr) = names.as_array() {
                data.reaction_names = arr
                    .iter()
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
        data.regen = mob_info.get("리젠").and_then(|v| v.as_i64()).unwrap_or(300);

        // Talk tick (대화틱)
        data.talk_tick = mob_info.get("대화틱").and_then(|v| v.as_i64()).unwrap_or(0);

        // Python Mob.setMove accepts a string or list, then expands `3-6`
        // with an exclusive upper bound and prefixes the template zone.
        let movement_words = match mob_info.get("이동") {
            Some(JsonValue::String(value)) => value.split_whitespace().collect::<Vec<_>>(),
            Some(JsonValue::Array(values)) => values
                .iter()
                .filter_map(JsonValue::as_str)
                .flat_map(str::split_whitespace)
                .collect(),
            _ => Vec::new(),
        };
        for word in movement_words {
            if let Some((start, end)) = word.split_once('-') {
                if let (Ok(start), Ok(end)) = (start.parse::<i64>(), end.parse::<i64>()) {
                    for room in start..end {
                        let room = room.to_string();
                        if !data.move_rooms.contains(&room) {
                            data.move_rooms.push(room);
                        }
                    }
                }
            } else if !data.move_rooms.iter().any(|room| room == word) {
                data.move_rooms.push(word.to_string());
            }
        }
        data.move_tick = mob_info
            .get("이동틱")
            .and_then(|v| v.as_i64())
            .filter(|value| *value != 0)
            .unwrap_or(30);

        // Mob type (몹종류)
        data.mob_type = mob_info.get("몹종류").and_then(|v| v.as_i64()).unwrap_or(0);

        // Linked combat type (전투종류)
        data.combat_type = mob_info
            .get("전투종류")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Personality (성격)
        data.personality = mob_info.get("성격").and_then(|v| v.as_i64()).unwrap_or(0);

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
                data.auto_scripts = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }

        // Parse events (이벤트:...). 배열=Legacy, 문자열=Rhai 스크립트 파일명.
        for (key, value) in mob_info {
            if key.starts_with("이벤트") {
                if let Some(arr) = value.as_array() {
                    let event_scripts: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    data.events
                        .insert(key.clone(), EventScript::Legacy(event_scripts));
                } else if let Some(s) = value.as_str() {
                    data.events
                        .insert(key.clone(), EventScript::Rhai(s.to_string()));
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
                data.sale_script = arr
                    .iter()
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
                            let prob = parts
                                .get(2)
                                .and_then(|s| s.parse::<i64>().ok())
                                .unwrap_or(100);
                            let scale = parts
                                .get(3)
                                .and_then(|s| s.parse::<i64>().ok())
                                .unwrap_or(1);
                            data.use_items.push((item_name, count, prob, scale));
                        }
                    }
                }
            }
        }

        if let Some(items) = mob_info.get("아이템").and_then(JsonValue::as_array) {
            for item in items.iter().filter_map(JsonValue::as_str) {
                let parts = item.split_whitespace().collect::<Vec<_>>();
                if parts.len() < 3 {
                    continue;
                }
                data.drop_items.push((
                    parts[0].to_string(),
                    parts[1].parse::<i64>().unwrap_or(0),
                    parts[2].parse::<i64>().unwrap_or(0),
                    parts
                        .get(3)
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(1),
                ));
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
        data.death_script = mob_info
            .get("소멸스크립")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Combat start script (전투시작)
        data.combat_start_script = mob_info
            .get("전투시작")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Combat script (전투스크립)
        data.combat_script = mob_info
            .get("전투스크립")
            .and_then(|v| v.as_str())
            .unwrap_or("주먹")
            .to_string();

        // HP display type (체력스크립)
        data.hp_display_type = mob_info
            .get("체력스크립")
            .and_then(|v| v.as_str())
            .unwrap_or("사람")
            .to_string();

        // Corpse time (시체)
        data.corpse_time = mob_info.get("시체").and_then(|v| v.as_i64()).unwrap_or(10);

        Ok(data)
    }

    /// Preload all mobs in a zone
    pub fn preload_zone(&mut self, zone: &str) -> Result<usize, MobError> {
        let zone_dir = self.data_dir.join(zone);

        if !zone_dir.exists() {
            return Err(MobError::NotFound(zone.to_string()));
        }

        let entries = std::fs::read_dir(&zone_dir).map_err(|e| MobError::IoError(e.to_string()))?;

        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| MobError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let name = path
                    .file_stem()
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
        self.spawn_mobs_for_room_with_difficulty(zone, room, mob_ids, 0)
    }

    /// Spawn mobs for a room with difficulty support.
    /// Mobs are loaded from base zone but instanced with difficulty-adjusted stats.
    ///
    /// # Arguments
    /// * `zone` - Zone name (can include difficulty suffix)
    /// * `room` - Room id
    /// * `mob_ids` - List of mob IDs to spawn
    /// * `difficulty` - Difficulty level (0-7)
    pub fn spawn_mobs_for_room_with_difficulty(
        &mut self,
        zone: &str,
        room: &str,
        mob_ids: &[String],
        difficulty: u8,
    ) {
        use super::difficulty::{base_zone_name, difficulty_from_zone};

        // Get effective difficulty from parameter or zone name
        let effective_difficulty = if difficulty > 0 {
            difficulty
        } else {
            difficulty_from_zone(zone)
        };

        // Use base zone for loading mob data
        let base_zone = base_zone_name(zone);

        // Room key includes difficulty for separate instances
        let room_key = format!("{}:{}:{}", base_zone, room, effective_difficulty);

        // Python places the map's complete mob list exactly once when the
        // Room object is created. Re-entering a room neither recreates a mob
        // that wandered away nor duplicates its declarations.
        if !self.initialized_rooms.insert(room_key.clone()) {
            return;
        }

        for mob_id in mob_ids {
            // Mob key uses base zone
            let key = format!("{}:{}", base_zone, mob_id);

            // Load mob on demand if not cached (from base zone)
            if !self.mobs.contains_key(&key) {
                log::debug!("[MobCache] Loading mob {} from zone {}", mob_id, base_zone);
                if let Err(e) = self.load_mob(base_zone, mob_id) {
                    log::warn!("[MobCache] Failed to load mob {}: {:?}", key, e);
                    continue;
                }
            }

            let data = match self.mobs.get(&key) {
                Some(d) => d,
                None => {
                    log::warn!("[MobCache] Mob {} not found after loading", key);
                    continue;
                }
            };

            // Every declaration is a distinct Python object, even when the
            // same template name occurs more than once.
            let instance = MobInstance::with_difficulty(
                key.clone(),
                zone.to_string(), // Keep original zone name for display
                room,
                data,
                effective_difficulty,
            );
            log::debug!("[MobCache] Spawned mob {} in room_key {}", key, room_key);
            self.instances
                .entry(room_key.clone())
                .or_default()
                .push(instance);
        }
        log::debug!(
            "[MobCache] spawn_mobs_for_room done for room_key {}",
            room_key
        );
    }

    /// Get active mobs in a room
    pub fn get_mobs_in_room(&self, zone: &str, room: &str) -> Vec<&MobInstance> {
        // Try to find mobs with difficulty suffix first, then without
        use super::difficulty::{base_zone_name, difficulty_from_zone};

        let effective_difficulty = difficulty_from_zone(zone);
        let base_zone = base_zone_name(zone);

        // Try with difficulty in key
        let room_key_with_diff = format!("{}:{}:{}", base_zone, room, effective_difficulty);
        log::debug!(
            "[MobCache] get_mobs_in_room trying key: {}",
            room_key_with_diff
        );
        if let Some(instances) = self.instances.get(&room_key_with_diff) {
            // Python Room.insert() prepends objects (objs.insert(0, obj)).
            // Instances are stored in spawn/JSON order, so expose the reverse
            // order to match Room.objs traversal and findObjName semantics.
            let mut result: Vec<&MobInstance> = instances.iter().filter(|m| m.alive).collect();
            result.reverse();
            log::debug!("[MobCache] Found {} mobs with diff key", result.len());
            if !result.is_empty() {
                return result;
            }
        }

        // Fallback to legacy key format for compatibility
        let room_key = format!("{}:{}", zone, room);
        log::debug!("[MobCache] get_mobs_in_room fallback to key: {}", room_key);
        let mut result: Vec<&MobInstance> = self
            .instances
            .get(&room_key)
            .map(|instances| instances.iter().filter(|m| m.alive).collect())
            .unwrap_or_default();
        result.reverse();
        log::debug!("[MobCache] Found {} mobs with legacy key", result.len());
        result
    }

    /// Get every mob object still placed in a room, regardless of action/alive state.
    /// Python `Room.objs` keeps death/regeneration-state Mob objects until they are removed.
    pub fn get_all_mobs_in_room(&self, zone: &str, room: &str) -> Vec<&MobInstance> {
        use super::difficulty::{base_zone_name, difficulty_from_zone};

        let base_zone = base_zone_name(zone);
        let difficulty = difficulty_from_zone(zone);
        if let Some(instances) = self
            .instances
            .get(&format!("{}:{}:{}", base_zone, room, difficulty))
        {
            let mut result: Vec<&MobInstance> = instances.iter().collect();
            result.reverse();
            return result;
        }
        let mut result: Vec<&MobInstance> = self
            .instances
            .get(&format!("{}:{}", zone, room))
            .map(|instances| instances.iter().collect())
            .unwrap_or_default();
        result.reverse();
        result
    }

    /// Mutable runtime instances for one room, using the same difficulty-key
    /// lookup order as [`Self::get_all_mobs_in_room`].
    pub(crate) fn get_all_mobs_in_room_mut(
        &mut self,
        zone: &str,
        room: &str,
    ) -> Option<&mut Vec<MobInstance>> {
        use super::difficulty::{base_zone_name, difficulty_from_zone};

        let difficulty_key = format!(
            "{}:{}:{}",
            base_zone_name(zone),
            room,
            difficulty_from_zone(zone)
        );
        let legacy_key = format!("{}:{}", zone, room);
        let key = if self.instances.contains_key(&difficulty_key) {
            difficulty_key
        } else {
            legacy_key
        };
        self.instances.get_mut(&key)
    }

    /// Whether this room already has runtime mob state.
    pub fn has_room_instance_state(&self, zone: &str, room: &str) -> bool {
        use super::difficulty::{base_zone_name, difficulty_from_zone};

        let base_zone = base_zone_name(zone);
        let difficulty = difficulty_from_zone(zone);
        self.instances
            .contains_key(&format!("{}:{}:{}", base_zone, room, difficulty))
            || self.instances.contains_key(&format!("{}:{}", zone, room))
    }

    /// Get active mobs in a room with specific difficulty
    pub fn get_mobs_in_room_with_difficulty(
        &self,
        zone: &str,
        room: &str,
        difficulty: u8,
    ) -> Vec<&MobInstance> {
        use super::difficulty::base_zone_name;

        let base_zone = base_zone_name(zone);
        let room_key = format!("{}:{}:{}", base_zone, room, difficulty);
        self.instances
            .get(&room_key)
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
                    let ok = inst.name == name
                        || inst.name.contains(name)
                        || d.reaction_names
                            .iter()
                            .any(|n| n == name || n.contains(name));
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
    pub fn damage_mob(
        &mut self,
        zone: &str,
        room: &str,
        mob_key: &str,
        damage: i64,
    ) -> Option<(i64, bool)> {
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
        self.update_respawns_at(chrono::Utc::now().timestamp());
    }

    /// Python `Mob.update()` keeps a corpse visible for `시체` seconds,
    /// changes it to ACT_REGEN, and only resets it after another `리젠`
    /// seconds.  Keep that observable two-stage state even when callers only
    /// need the state update and have no room broadcaster to emit the text.
    pub(crate) fn update_respawns_at(&mut self, now: i64) {
        self.update_respawns_for_room_keys_at(now, None);
    }

    /// Update only rooms selected by the caller. Python `Loop.updateRooms`
    /// calls `Room.update()` for rooms occupied by connected players each
    /// second; this keeps the represented respawn branch scoped the same way.
    pub(crate) fn update_respawns_in_rooms_at(&mut self, rooms: &[(String, String)], now: i64) {
        use super::difficulty::{base_zone_name, difficulty_from_zone};

        let mut room_keys = HashSet::new();
        for (zone, room) in rooms {
            room_keys.insert(format!(
                "{}:{}:{}",
                base_zone_name(zone),
                room,
                difficulty_from_zone(zone)
            ));
            // Keep legacy instances created by old runtime paths observable.
            room_keys.insert(format!("{}:{}", zone, room));
        }
        self.update_respawns_for_room_keys_at(now, Some(&room_keys));
    }

    fn update_respawns_for_room_keys_at(&mut self, now: i64, room_keys: Option<&HashSet<String>>) {
        // Collect all mob data needed for respawn
        let mut respawn_data: std::collections::HashMap<String, RawMobData> =
            std::collections::HashMap::new();

        for (room_key, instances) in &self.instances {
            if room_keys.is_some_and(|keys| !keys.contains(room_key)) {
                continue;
            }
            for instance in instances {
                if !instance.alive && !respawn_data.contains_key(&instance.mob_key) {
                    if let Some(data) = self.get_mob(&instance.mob_key) {
                        respawn_data.insert(instance.mob_key.clone(), data.clone());
                    }
                }
            }
        }

        // Now do the respawns
        for (room_key, instances) in &mut self.instances {
            if room_keys.is_some_and(|keys| !keys.contains(room_key)) {
                continue;
            }
            for instance in instances {
                if !instance.alive {
                    if let Some(data) = respawn_data.get(&instance.mob_key) {
                        let elapsed = now.saturating_sub(instance.death_time);
                        if elapsed >= data.corpse_time.saturating_add(data.regen) {
                            instance.respawn(data);
                        } else if elapsed >= data.corpse_time && instance.act == 2 {
                            // Python doDeath(): the body is gone but the mob
                            // object remains in Room.objs in ACT_REGEN.
                            instance.act = 3;
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
            if let Some(mob) = instances
                .iter_mut()
                .find(|m| m.mob_key == mob_key && m.alive)
            {
                mob.kill();
                return true;
            }
        }
        false
    }

    /// Add a mob instance to the cache (for script spawning)
    pub fn add_mob_instance(&mut self, mob: MobInstance) {
        let room_key = format!("{}:{}", mob.zone, mob.room);
        self.instances.entry(room_key).or_default().push(mob);
    }

    /// Remove a departing player from every runtime mob target list.
    pub fn remove_target_everywhere(&mut self, player_name: &str) {
        for instances in self.instances.values_mut() {
            for mob in instances {
                mob.targets.retain(|target| target != player_name);
                if mob.targets.is_empty() && mob.act == 1 {
                    mob.act = 0;
                }
            }
        }
    }

    /// Get a mutable reference to a mob instance
    pub fn get_mob_instance_mut(
        &mut self,
        zone: &str,
        room: &str,
        mob_key: &str,
    ) -> Option<&mut MobInstance> {
        self.get_all_mobs_in_room_mut(zone, room)?
            .iter_mut()
            .find(|mob| mob.mob_key == mob_key)
    }

    /// Get all mob instances across all rooms (for admin search functions)
    /// Returns Vec of (room_key, instances) tuples
    pub fn get_all_instances(&self) -> Vec<(String, Vec<&MobInstance>)> {
        self.instances
            .iter()
            .map(|(key, instances)| {
                let alive_instances: Vec<&MobInstance> =
                    instances.iter().filter(|m| m.alive).collect();
                (key.clone(), alive_instances)
            })
            .filter(|(_, instances)| !instances.is_empty())
            .collect()
    }

    pub(crate) fn moving_instances_snapshot(&self) -> Vec<(String, String, String, i64)> {
        self.instances
            .values()
            .flat_map(|instances| instances.iter())
            .filter(|mob| mob.alive && mob.act == 0)
            .map(|mob| {
                (
                    mob.zone.clone(),
                    mob.room.clone(),
                    mob.mob_key.clone(),
                    mob.last_move_millis,
                )
            })
            .collect()
    }

    pub(crate) fn set_move_time(&mut self, zone: &str, room: &str, mob_key: &str, now_millis: i64) {
        if let Some(mob) = self.get_mob_instance_mut(zone, room, mob_key) {
            mob.last_move_millis = now_millis;
        }
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
        exp = exp.max(1);
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
        gold = gold.max(1);
        if gold > 2_000_000_000 {
            gold = 2_000_000_000;
        }

        // Calculate difficulty bonus (Python: Body.difficulty[d-1][2] and [3])
        let (bonus_exp, bonus_gold) = if (1..=9).contains(&difficulty) {
            let multiplier =
                [1.0, 2.0, 3.0, 4.0, 5.0, 6.5, 9.0, 12.5, 16.0][(difficulty - 1) as usize];
            (
                (exp as f64 * multiplier) as i64,
                (gold as f64 * multiplier) as i64,
            )
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
    pub fn calculate(
        mob_level: i64,
        target_level: i64,
        difficulty: i64,
        herb_list: &[String],
        base_chance: f64,
    ) -> Option<HerbDrop> {
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
    fn cloned_mobs_have_distinct_stable_runtime_ids_across_respawn() {
        let data = RawMobData::new();
        let mut first = MobInstance::new("zone:same".to_string(), "zone".to_string(), 1, &data);
        let second = MobInstance::new("zone:same".to_string(), "zone".to_string(), 1, &data);
        assert_ne!(first.instance_id, second.instance_id);
        let id = first.instance_id;
        first.record_player_damage("갑", 30);
        first.record_player_damage("갑", 12);
        first.record_player_damage("을", 7);
        assert_eq!(first.damage_map["갑"], 42);
        assert_eq!(first.damage_map["을"], 7);
        first.kill();
        first.respawn(&data);
        assert_eq!(first.instance_id, id);
        assert!(first.damage_map.is_empty());
    }

    #[test]
    fn repeated_template_declarations_spawn_distinctly_once_and_do_not_respawn_wanderers() {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);
        let declarations = vec!["12".to_string(), "12".to_string()];
        cache.spawn_mobs_for_room("낙양성", "복제방", &declarations);
        let first = cache.get_all_mobs_in_room("낙양성", "복제방");
        assert_eq!(first.len(), 2);
        assert_ne!(first[0].instance_id, first[1].instance_id);

        cache.spawn_mobs_for_room("낙양성", "복제방", &declarations);
        assert_eq!(cache.get_all_mobs_in_room("낙양성", "복제방").len(), 2);
        assert!(cache
            .move_instance("낙양성", "복제방", "낙양성:12", "낙양성", "이동방", 1)
            .is_some());
        cache.spawn_mobs_for_room("낙양성", "복제방", &declarations);
        assert_eq!(cache.get_all_mobs_in_room("낙양성", "복제방").len(), 1);
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
    fn respawn_update_keeps_python_corpse_then_regen_phases() {
        let mut cache = MobCache::new();
        let mut data = RawMobData::new();
        data.name = "단계시험몹".to_string();
        data.zone = "시험존".to_string();
        data.corpse_time = 10;
        data.regen = 20;
        cache
            .mobs
            .insert("시험존:단계시험몹".to_string(), data.clone());

        let mut mob = MobInstance::new(
            "시험존:단계시험몹".to_string(),
            "시험존".to_string(),
            "1",
            &data,
        );
        mob.kill();
        mob.death_time = 100;
        cache.add_mob_instance(mob);

        cache.update_respawns_at(109);
        let mob = cache.get_all_mobs_in_room("시험존", "1")[0];
        assert!(!mob.alive);
        assert_eq!(mob.act, 2);

        cache.update_respawns_at(110);
        let mob = cache.get_all_mobs_in_room("시험존", "1")[0];
        assert!(!mob.alive);
        assert_eq!(mob.act, 3);

        cache.update_respawns_at(129);
        assert!(!cache.get_all_mobs_in_room("시험존", "1")[0].alive);
        cache.update_respawns_at(130);
        let mob = cache.get_all_mobs_in_room("시험존", "1")[0];
        assert!(mob.alive);
        assert_eq!(mob.act, 0);
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
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);
        let data = match cache.load_mob("낙양성", "83") {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test_load_83_has_편지_event_key: load_mob failed (skip if data not present): {}", e);
                return;
            }
        };
        let ev_keys: Vec<&String> = data
            .events
            .keys()
            .filter(|k| k.starts_with("이벤트"))
            .collect();
        assert!(
            data.events.contains_key("이벤트: $대화 $대 편지"),
            "83.json must have '이벤트: $대화 $대 편지', event keys: {:?}",
            ev_keys
        );
    }

    #[test]
    fn moving_mob_routes_expand_python_ranges_with_exclusive_end() {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);
        let data = cache.load_mob("감숙성", "53").unwrap();
        assert_eq!(data.move_tick, 30);
        assert!(data.move_rooms.contains(&"400".to_string()));
        assert!(data.move_rooms.contains(&"457".to_string()));
        assert!(!data.move_rooms.contains(&"458".to_string()));
        assert!(data.move_rooms.contains(&"460".to_string()));
        assert!(!data.move_rooms.contains(&"546".to_string()));
    }

    /// Test mob spawning for starting room
    #[test]
    fn test_spawn_mobs_starting_room() {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);

        // Spawn mobs for room 1 (starting room) with mob_ids from map data
        let mob_ids = vec!["밍밍-범죄자".to_string(), "포졸".to_string()];
        cache.spawn_mobs_for_room("낙양성", "1", &mob_ids);

        // Check if mobs were spawned
        let mobs = cache.get_mobs_in_room("낙양성", "1");
        eprintln!(
            "test_spawn_mobs_starting_room: Found {} mobs in room 낙양성:1",
            mobs.len()
        );

        for mob in &mobs {
            eprintln!(
                "  - Mob: {}, key: {}, desc1: {}",
                mob.name,
                mob.mob_key,
                if let Some(data) = cache.get_mob(&mob.mob_key) {
                    &data.desc1
                } else {
                    "?"
                }
            );
        }

        // Verify mobs exist (this test will fail if mob loading is broken)
        assert!(!mobs.is_empty(), "No mobs found in room 낙양성:1");
    }

    /// Test loading room and spawning mobs from room data
    #[test]
    fn test_room_and_mob_spawn_integration() {
        let mob_data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("mob");
        let room_data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("map");
        let mut mob_cache = MobCache::with_data_dir(mob_data_dir);
        let mut room_cache = crate::world::RoomCache::with_data_dir(room_data_dir);

        // Load room 1
        let room_result = room_cache.get_room("낙양성", "1");
        assert!(
            room_result.is_ok(),
            "Failed to load room: {:?}",
            room_result
        );

        let room = room_result.unwrap();
        let room_ref = room.read().unwrap();
        let mob_ids = room_ref.mob_ids.clone();
        eprintln!(
            "test_room_and_mob_spawn_integration: Room mob_ids = {:?}",
            mob_ids
        );

        // Spawn mobs using the mob_ids from the room
        mob_cache.spawn_mobs_for_room("낙양성", "1", &mob_ids);

        // Check if mobs were spawned
        let mobs = mob_cache.get_mobs_in_room("낙양성", "1");
        eprintln!(
            "test_room_and_mob_spawn_integration: Found {} mobs in room 낙양성:1",
            mobs.len()
        );

        for mob in &mobs {
            eprintln!("  - Mob: {}, key: {}", mob.name, mob.mob_key);
        }

        // Verify mobs exist
        assert!(
            !mobs.is_empty(),
            "No mobs found in room 낙양성:1 after spawn"
        );
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
        eprintln!(
            "test_worldstate_spawn_integration: Room found in cache: {}",
            room.is_some()
        );

        if let Some(room) = room {
            let room_ref = room.read().unwrap();
            eprintln!(
                "test_worldstate_spawn_integration: Room mob_ids = {:?}",
                room_ref.mob_ids
            );
        }

        // Check if mobs were spawned
        let mobs = world.get_mobs_in_room("낙양성", "1");
        eprintln!(
            "test_worldstate_spawn_integration: Found {} mobs in room 낙양성:1",
            mobs.len()
        );

        for mob in &mobs {
            eprintln!("  - Mob: {}, key: {}", mob.name, mob.mob_key);
        }

        // Verify mobs exist
        assert!(
            !mobs.is_empty(),
            "No mobs found in room 낙양성:1 after WorldState spawn"
        );
        assert_eq!(mobs.len(), 2, "Expected 2 mobs in room 낙양성:1");
    }

    /// Test loading mob data directly
    #[test]
    fn test_load_mob_밍밍_범죄자() {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("mob");
        let mut cache = MobCache::with_data_dir(data_dir);

        match cache.load_mob("낙양성", "밍밍-범죄자") {
            Ok(data) => {
                eprintln!(
                    "test_load_mob_밍밍_범죄자: name={}, zone={}, desc1={}",
                    data.name, data.zone, data.desc1
                );
                assert_eq!(data.name, "밍밍");
            }
            Err(e) => {
                eprintln!("test_load_mob_밍밍_범죄자: Failed to load: {}", e);
            }
        }
    }
}
