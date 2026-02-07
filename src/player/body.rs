//! Body module for MUD engine
//!
//! This module provides the Body structure for managing game entities with
//! stats, combat, skills, and experience system.

use crate::data::get_skill_defense_head;
use crate::object::{Object, Value};
use crate::world::item::get_item_weight_by_key;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

/// Action states for game entities (Python: ACT_STAND=0, ACT_FIGHT=1, ACT_DEATH=2, ACT_REGEN=3, ACT_REST=4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActState {
    /// Standing/Idle state (ACT_STAND)
    #[default]
    Stand = 0,
    /// Fighting state (ACT_FIGHT)
    Fight = 1,
    /// Death state (ACT_DEATH)
    Death = 2,
    /// Regenerating state (ACT_REGEN)
    Regeneration = 3,
    /// Resting state (ACT_REST)
    Rest = 4,
    /// Moving state (additional, not in Python ACT constants)
    Move = 5,
}

impl ActState {
    /// Create ActState from i32 value (Python compatibility)
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => ActState::Fight,
            2 => ActState::Death,
            3 => ActState::Regeneration,
            4 => ActState::Rest,
            5 => ActState::Move,
            _ => ActState::Stand,
        }
    }

    /// Convert to i32 value
    pub fn to_i32(self) -> i32 {
        self as i32
    }
}

/// Skill level names and their numeric values
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillLevel {
    Primary = 1,      // 초급
    Intermediate = 2, // 중급
    Advanced = 3,     // 상급
    High = 4,         // 고급
    Special = 5,      // 특급
    Peak = 6,         // 절정
    Transcendent = 7, // 초절정
}

impl SkillLevel {
    /// Get skill level from name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "초급" => Some(SkillLevel::Primary),
            "중급" => Some(SkillLevel::Intermediate),
            "상급" => Some(SkillLevel::Advanced),
            "고급" => Some(SkillLevel::High),
            "특급" => Some(SkillLevel::Special),
            "절정" => Some(SkillLevel::Peak),
            "초절정" => Some(SkillLevel::Transcendent),
            _ => None,
        }
    }

    /// Get the name of the skill level
    pub fn name(self) -> &'static str {
        match self {
            SkillLevel::Primary => "초급",
            SkillLevel::Intermediate => "중급",
            SkillLevel::Advanced => "상급",
            SkillLevel::High => "고급",
            SkillLevel::Special => "특급",
            SkillLevel::Peak => "절정",
            SkillLevel::Transcendent => "초절정",
        }
    }

    /// Get skill level value as u8
    pub fn value(self) -> u8 {
        self as u8
    }

    /// All skill level names in order
    pub fn all_names() -> &'static [&'static str] {
        &["초급", "중급", "상급", "고급", "특급", "절정", "초절정"]
    }
}

/// Skill training data (level, experience)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillTraining {
    /// Skill proficiency level (1-12)
    pub level: u8,
    /// Skill experience points
    pub exp: u32,
}

impl SkillTraining {
    pub fn new(level: u8, exp: u32) -> Self {
        Self { level, exp }
    }
}

/// Active skill effect on a character
#[derive(Debug, Clone)]
pub struct ActiveSkill {
    /// Name of the skill
    pub name: String,
    /// Remaining duration in ticks
    pub start_time: i32,
    /// Strength bonus
    pub str_bonus: i32,
    /// Dexterity bonus
    pub dex_bonus: i32,
    /// Armor bonus
    pub arm_bonus: i32,
    /// MP bonus
    pub mp_bonus: i32,
    /// Max MP bonus
    pub max_mp_bonus: i32,
}

impl ActiveSkill {
    pub fn new(name: String, start_time: i32) -> Self {
        Self {
            name,
            start_time,
            str_bonus: 0,
            dex_bonus: 0,
            arm_bonus: 0,
            mp_bonus: 0,
            max_mp_bonus: 0,
        }
    }
}

/// Body structure - Core combat and stats system
///
/// Extends Object with combat-related functionality including:
/// - Stats (Strength, Dexterity, Armor, HP, MP)
/// - Combat (targets, attack chance, attack point)
/// - Skills (skill management, skill training)
/// - Experience and leveling
#[derive(Debug)]
pub struct Body {
    /// Base Object (contains attributes, inventory, etc.)
    pub object: Object,

    /// Current action state
    pub act: ActState,
    /// Tick counter for game updates
    pub tick: u64,
    /// Currently active skill
    pub skill: Option<String>,
    /// Last used skill (for caching)
    pub last_skill: Option<String>,
    /// Attack power modifier
    pub attpower: i32,
    /// Armor modifier from equipment
    pub armor: i32,
    /// Dexterity modifier
    pub dex: i32,
    /// Strength modifier
    pub _str: i32,
    /// Dexterity modifier
    pub _dex: i32,
    /// Armor modifier
    pub _arm: i32,
    /// MP modifier
    pub _mp: i32,
    /// Max MP modifier
    pub _maxmp: i32,
    /// HP modifier
    pub _hp: i32,
    /// Max HP modifier
    pub _maxhp: i32,
    /// Hit chance modifier
    pub _hit: i32,
    /// Miss/evasion modifier
    pub _miss: i32,
    /// Critical damage modifier
    pub _critical: i32,
    /// Critical chance modifier
    pub _critical_chance: i32,
    /// Magic chance modifier
    pub _magic_chance: i32,
    /// Experience bonus modifier
    pub _exp: i32,
    /// Currently equipped weapon item (weak reference)
    pub weapon_item: Option<Weak<Mutex<Object>>>,
    /// Combat targets
    pub targets: Vec<Weak<Mutex<Object>>>,
    /// Active defense skills
    pub active_skills: Vec<ActiveSkill>,
    /// Skill training data (skill_name -> (level, exp))
    pub skill_map: HashMap<String, SkillTraining>,
    /// List of learned skills
    pub skill_list: Vec<String>,
    /// Item skill training map
    pub item_skill_map: HashMap<String, u32>,
    /// Skill cooldown tracking (skill_name -> last_cast_timestamp)
    pub skill_cooldowns: HashMap<String, i64>,
    /// 범용 스크립트: 아이템확인에서 설정, 옵션출력/옵션확인 등에서 사용. Complete 시 클리어.
    pub script_temp_item: Option<Arc<Mutex<Object>>>,
    /// 도착 쪽지. 키 "메모:발신자이름", 값 MemoRecord. load/save 시 JSON 루트의 "메모:xxx"와 연동.
    pub memos: HashMap<String, MemoRecord>,
    /// 대화 기록 (NPC와의 대화 내용)
    pub talk_history: Vec<String>,
    /// Death progression step (0-4, for doDeath() progression)
    pub step_death: i32,
    /// Time of death (for regen timing)
    pub time_of_death: Option<std::time::Instant>,
    /// Corpse duration (seconds before becoming corpse)
    pub corpse_duration: u64,
    /// Regeneration duration (seconds before regen)
    pub regen_duration: u64,
}

/// 쪽지 한 통. 파이썬 memo[키] = {제목,시간,작성자,내용}.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoRecord {
    pub 제목: String,
    pub 시간: String,
    pub 작성자: String,
    pub 내용: String,
}

impl Default for Body {
    fn default() -> Self {
        Self::new()
    }
}

impl Body {
    /// Creates a new Body with default values
    pub fn new() -> Self {
        Body {
            object: Object::new(),
            act: ActState::Stand,
            tick: 0,
            skill: None,
            last_skill: None,
            attpower: 0,
            armor: 0,
            dex: 0,
            _str: 0,
            _dex: 0,
            _arm: 0,
            _mp: 0,
            _maxmp: 0,
            _hp: 0,
            _maxhp: 0,
            _hit: 0,
            _miss: 0,
            _critical: 0,
            _critical_chance: 0,
            _magic_chance: 0,
            _exp: 0,
            weapon_item: None,
            targets: Vec::new(),
            active_skills: Vec::new(),
            skill_map: HashMap::new(),
            skill_list: Vec::new(),
            item_skill_map: HashMap::new(),
            skill_cooldowns: HashMap::new(),
            script_temp_item: None,
            memos: HashMap::new(),
            talk_history: Vec::new(),
            step_death: 0,
            time_of_death: None,
            corpse_duration: 60, // Default 60 seconds
            regen_duration: 300, // Default 5 minutes
        }
    }

    /// Creates a Body from an existing Object
    pub fn from_object(object: Object) -> Self {
        let mut body = Self::new();
        body.object = object;
        body
    }

    // ==================== Stat Getters ====================

    /// Gets total strength (base + modifier + attribute)
    pub fn get_str(&self) -> i64 {
        let base = self.object.getInt("힘") as i32;
        let total = (self._str + base).max(0);
        total as i64
    }

    /// Gets total dexterity (base + modifier + attribute)
    pub fn get_dex(&self) -> i64 {
        let base = self.object.getInt("민첩성") as i32;
        let total = (self._dex + base).max(0);
        total as i64
    }

    /// Gets total armor (base + modifier + attribute)
    pub fn get_arm(&self) -> i64 {
        let base = self.object.getInt("맷집") as i32;
        let alpha = if self.object.getString("맷집상승").is_empty() {
            0
        } else {
            1000
        };
        let total = (self._arm + base + alpha).max(0);
        total as i64
    }

    /// Gets current MP
    pub fn get_mp(&self) -> i64 {
        if self._mp != 0 {
            let base = self.object.getInt("내공");
            let mp = base + (base * self._mp as i64 / 100);
            mp
        } else {
            self.object.getInt("내공")
        }
    }

    /// Gets maximum MP
    pub fn get_max_mp(&self) -> i64 {
        if self._maxmp != 0 {
            let base = self.object.getInt("최고내공");
            base + self._maxmp as i64
        } else {
            self.object.getInt("최고내공")
        }
    }

    /// Gets current HP
    pub fn get_hp(&self) -> i64 {
        self.object.getInt("체력")
    }

    /// Gets maximum HP (base + armor bonus)
    pub fn get_max_hp(&self) -> i64 {
        let base = self.object.getInt("최고체력");
        let h = base + self.get_arm() * 30;
        if self._maxhp != 0 {
            h + self._maxhp as i64
        } else {
            h
        }
    }

    /// Gets hit chance
    pub fn get_hit(&self) -> i64 {
        let base = self.object.getInt("명중");
        if self._hit != 0 {
            (base as i32 + self._hit) as i64
        } else {
            base
        }
    }

    /// Gets critical damage
    pub fn get_critical(&self) -> i64 {
        let base = self.object.getInt("필살");
        if self._critical != 0 {
            (base as i32 + self._critical) as i64
        } else {
            base
        }
    }

    /// Gets critical chance (luck)
    pub fn get_critical_chance(&self) -> i64 {
        let base = self.object.getInt("운");
        if self._critical_chance != 0 {
            (base as i32 + self._critical_chance) as i64
        } else {
            base
        }
    }

    /// Gets evasion/miss chance
    pub fn get_miss(&self) -> i64 {
        let base = self.object.getInt("회피");
        if self._miss != 0 {
            (base as i32 + self._miss) as i64
        } else {
            base
        }
    }

    /// Gets bonus experience percentage
    pub fn get_bonus_exp(&self) -> i32 {
        self._exp
    }

    /// Gets bonus magic chance
    pub fn get_bonus_magic_chance(&self) -> i32 {
        self._magic_chance
    }

    /// Gets armor value from equipment
    pub fn get_armor(&self) -> i32 {
        self.armor
    }

    /// Gets attack power
    pub fn get_attack_power(&self) -> i32 {
        self.attpower
    }

    // ==================== Combat Methods ====================

    /// Sets a target for combat
    pub fn set_target(&mut self, target: Arc<Mutex<Object>>) {
        self.act = ActState::Fight;
        let weak_target = Arc::downgrade(&target);
        if !self.targets.iter().any(|t| {
            t.upgrade()
                .as_ref()
                .map(|t| Arc::as_ptr(t) == Arc::as_ptr(&target))
                .unwrap_or(false)
        }) {
            self.targets.push(weak_target);
        }
    }

    /// Clears a specific target or all targets
    pub fn clear_target(&mut self, target: Option<&Arc<Mutex<Object>>>) {
        if let Some(t) = target {
            let t_ptr = Arc::as_ptr(t);
            self.targets.retain(|weak| {
                if let Some(strong) = weak.upgrade() {
                    let ptr = Arc::as_ptr(&strong);
                    let is_same = ptr != t_ptr;
                    if !is_same {
                        // Clear reverse target
                        if let Ok(_obj) = strong.lock() {
                            // Note: This would need access to the target's Body
                            // For now, just remove from our list
                        }
                    }
                    is_same
                } else {
                    false
                }
            });

            if self.act == ActState::Fight && self.targets.is_empty() {
                self.act = ActState::Stand;
                self.dex = 0;
                self.stop_skill();
            }
        } else {
            // Clear all targets
            self.clear_all_targets();
        }
    }

    /// Clears all combat targets
    pub fn clear_all_targets(&mut self) {
        self.targets.clear();
        if self.act == ActState::Fight {
            self.act = ActState::Stand;
            self.dex = 0;
            self.stop_skill();
        }
    }

    /// Gets attack chance against a target
    ///
    /// Returns -1 if level difference is too large
    /// Otherwise returns the hit percentage
    pub fn get_attack_chance(&self, target_level: i64, max_level_diff: i64) -> i32 {
        let my_level = self.object.getInt("레벨");

        // Limit attack level difference
        if (target_level - my_level) >= max_level_diff {
            return -1;
        }

        let base_chance = 100;
        let hit_bonus = self.get_hit() as f32 * 0.1; // 명중확률
        let evasion_penalty = 0; // Would need target's getMiss()

        let level_diff = ((target_level - my_level) + 90) / 3;
        (base_chance - level_diff + hit_bonus as i64 - evasion_penalty) as i32
    }

    /// Gets attack damage range
    ///
    /// Returns (damage, min_damage, max_damage)
    pub fn get_attack_point(&self) -> (i32, i32, i32) {
        let str_val = self.get_str();
        let max_mp = self.get_max_mp();

        // Calculate base damage
        let c1 = str_val * 2 + max_mp / 5;
        let c2 = self.get_attack_power() as i64;

        // Armor reduction would go here
        let m = (c1 + c2).max(1);

        // Add randomness (80% - 120%)
        let c1 = (m * 80 / 100).max(1) as i32;
        let c2 = (m * 120 / 100) as i32;

        let range = (c2 - c1 + 1).max(1);
        let damage = c1 + fastrand::i32(0..range);

        (damage.max(1), c1, c2)
    }

    // ==================== Skill Methods ====================

    /// Gets a skill by name (caches last skill)
    pub fn get_skill(&mut self, s_name: &str) -> Option<String> {
        if let Some(ref last) = self.last_skill {
            if s_name == last {
                self.skill = Some(s_name.to_string());
                return self.skill.clone();
            }
        }
        self.skill = Some(s_name.to_string());
        self.last_skill = self.skill.clone();
        self.skill.clone()
    }

    /// Stops the current skill
    pub fn stop_skill(&mut self) {
        self.last_skill = self.skill.take();
    }

    /// Clears all active skills
    pub fn clear_skills(&mut self) {
        self.stop_skill();
        self.active_skills.clear();
    }

    /// Adds training to a skill
    pub fn skill_up(&mut self, s_name: Option<&str>, skill_chance: i32) -> bool {
        let name = s_name.or(self.skill.as_deref()).unwrap_or("");

        if name.is_empty() {
            return false;
        }

        // Initialize skill if not exists
        if !self.skill_map.contains_key(name) {
            self.skill_map
                .insert(name.to_string(), SkillTraining::new(1, 0));
        }

        if let Some(training) = self.skill_map.get_mut(name) {
            training.exp += 1;

            // Check for skill level up
            if training.exp >= skill_chance as u32 && training.level < 12 {
                training.level += 1;
                training.exp = 0;
                return true; // Leveled up
            }
        }

        false
    }

    /// Gets skill training data
    pub fn get_skill_training(&self, skill_name: &str) -> Option<SkillTraining> {
        self.skill_map.get(skill_name).copied()
    }

    /// Sets skill training data
    pub fn set_skill_training(&mut self, skill_name: &str, level: u8, exp: u32) {
        self.skill_map
            .insert(skill_name.to_string(), SkillTraining::new(level, exp));
    }

    /// Checks if a skill is on cooldown
    /// Returns the remaining cooldown seconds, or 0 if not on cooldown
    pub fn get_skill_cooldown_remaining(&self, skill_name: &str) -> i64 {
        if let Some(&last_cast) = self.skill_cooldowns.get(skill_name) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            (last_cast + 3 - now).max(0) // Default 3 second cooldown
        } else {
            0
        }
    }

    /// Sets the last cast time for a skill (marking it as cast now)
    pub fn set_skill_cast_time(&mut self, skill_name: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.skill_cooldowns.insert(skill_name.to_string(), now);
    }

    /// Checks if a body can move
    pub fn is_movable(&self) -> bool {
        self.act != ActState::Fight && self.act != ActState::Rest
    }

    // ==================== Experience Methods ====================

    /// Adds experience and checks for level up
    ///
    /// Returns true if leveled up
    pub fn add_exp(&mut self, exp: i64) -> bool {
        let current_exp = self.object.getInt("현재경험치");
        self.object.set("현재경험치", current_exp + exp);

        let total_exp = self.get_total_exp();
        if self.object.getInt("현재경험치") >= total_exp {
            // Level up!
            self.object.set("현재경험치", 0);
            let level = self.object.getInt("레벨") + 1;
            self.object.set("레벨", level);
            self.level_up();
            return true;
        }
        false
    }

    /// Gets total experience needed for current level
    pub fn get_total_exp(&self) -> i64 {
        let level = self.object.getInt("레벨");
        let c = (((level * level) / 3) + 30) * (level + 4);

        if c < 1 {
            return 1;
        }

        c.min(999_999_999) // MAX_EXP
    }

    /// Levels up the character
    pub fn level_up(&mut self) {
        use fastrand;

        let hp_up = (fastrand::i32(0..10) + 25) as i64;
        let max_hp = self.object.getInt("최고체력") + hp_up;
        self.object.set("최고체력", max_hp);

        let armor = self.object.getInt("맷집") + 1;
        self.object.set("맷집", armor);

        let current_hp = self.get_max_hp();
        self.object.set("체력", current_hp);

        let current_mp = self.get_max_mp();
        self.object.set("내공", current_mp);
    }

    /// Initializes body stats for a new character
    pub fn init_body(&mut self) {
        self.object.set("레벨", 1);
        self.object.set("체력", 450);
        self.object.set("최고체력", 450);
        self.object.set("힘", 15);
        self.object.set("맷집", 15);
        self.object.set("민첩성", 0);
        self.object.set("은전", 100000);
        self.object.set("금전", 0);
        self.object.set("내공", 18);
        self.object.set("최고내공", 18);
        self.object.set("나이", 18);
        self.object.set("나이오름틱", 0);
        self.object.set("현재경험치", 0);
        self.object.set("1 숙련도", 0);
        self.object.set("2 숙련도", 0);
        self.object.set("3 숙련도", 0);
        self.object.set("4 숙련도", 0);
        self.object.set("5 숙련도", 0);
        self.object.set("1 숙련도경험치", 0);
        self.object.set("2 숙련도경험치", 0);
        self.object.set("3 숙련도경험치", 0);
        self.object.set("4 숙련도경험치", 0);
        self.object.set("5 숙련도경험치", 0);
        self.object.set("힘경험치", 0);
        self.object.set("민첩성경험치", 0);
        self.object.set("0 성격플킬", 0);
        self.object.set("1 성격플킬", 0);
        self.object.set("2 성격플킬", 0);
        self.object.set("무공숙련도", "");
        self.object.set("무공이름", "");
        self.object.set("보험료", 0);
    }

    /// Gets HP status string based on current/max HP
    pub fn get_hp_string(&self) -> String {
        let hp = self.get_hp();
        let max_hp = self.get_max_hp();

        if max_hp <= 0 {
            return String::new();
        }

        let ratio = (hp * 100 / max_hp) as usize;
        let index = (ratio * 10 / 100).min(10);

        // HP status bars
        const HP_BARS: &[&str] = &[
            "\x1b[37m━━━━━━━━━━\x1b[37m",
            "\x1b[31m━\x1b[37m━━━━━━━━━\x1b[37m",
            "\x1b[31m━━\x1b[37m━━━━━━━━\x1b[37m",
            "\x1b[31m━━━\x1b[37m━━━━━━━\x1b[37m",
            "\x1b[33m━━━━\x1b[37m━━━━━━\x1b[37m",
            "\x1b[33m━━━━━\x1b[37m━━━━━\x1b[37m",
            "\x1b[33m━━━━━━\x1b[37m━━━━\x1b[37m",
            "\x1b[32m━━━━━━━\x1b[37m━━━\x1b[37m",
            "\x1b[32m━━━━━━━━\x1b[37m━━\x1b[37m",
            "\x1b[32m━━━━━━━━━\x1b[37m━\x1b[37m",
            "\x1b[32m━━━━━━━━━━\x1b[37m",
        ];

        if index < HP_BARS.len() {
            HP_BARS[index].to_string()
        } else {
            HP_BARS[10].to_string()
        }
    }

    /// Gets item weight (objs + inv_stack 수량*무게)
    pub fn get_item_weight(&self) -> i64 {
        let mut weight = 0;
        for obj in &self.object.objs {
            if let Ok(item) = obj.lock() {
                if !item.checkAttr("아이템속성", "출력안함") {
                    weight += item.getInt("무게");
                }
            }
        }
        for (key, cnt) in &self.object.inv_stack {
            weight += cnt.saturating_mul(get_item_weight_by_key(key));
        }
        weight
    }

    /// Gets item count (objs + inv_stack 수량 합, excluding hidden)
    pub fn get_item_count(&self) -> usize {
        let mut count = 0;
        for obj in &self.object.objs {
            if let Ok(item) = obj.lock() {
                if !item.checkAttr("아이템속성", "출력안함") {
                    count += 1;
                }
            }
        }
        count += self
            .object
            .inv_stack
            .values()
            .map(|&c| c as usize)
            .sum::<usize>();
        count
    }

    /// Gets inventory item count (not in use). objs 중 inUse 아닌 것 + inv_stack 합.
    pub fn get_inven_item_count(&self) -> usize {
        let mut count = 0;
        for obj in &self.object.objs {
            if let Ok(item) = obj.lock() {
                if !item.checkAttr("아이템속성", "출력안함") && !item.getBool("inUse") {
                    count += 1;
                }
            }
        }
        count += self
            .object
            .inv_stack
            .values()
            .map(|&c| c as usize)
            .sum::<usize>();
        count
    }

    /// Adds armor experience
    pub fn add_arm(&mut self, arm: i32) {
        let mut exp = self.object.getInt("맷집경험치");
        exp += arm as i64;
        self.object.set("맷집경험치", exp);

        let armor_val = self.object.getInt("맷집");
        let threshold = (armor_val - 10) * 20;

        if exp >= threshold && threshold > 0 {
            self.object.set("맷집경험치", 0);
            self.object.set("맷집", armor_val + 1);
        }
    }

    /// Adds strength experience
    pub fn add_str(&mut self, str_val: i32, check: bool) -> bool {
        let mut exp = self.object.getInt("힘경험치");
        exp += str_val as i64;
        self.object.set("힘경험치", exp);

        if check {
            let str_stat = self.object.getInt("힘");
            let threshold = (str_stat - 10) * 20;

            if exp >= threshold && threshold > 0 {
                self.object.set("힘경험치", 0);
                self.object.set("힘", str_stat + 1);
                return true; // Leveled up
            }
        }
        false
    }

    /// Adds dexterity experience
    pub fn add_dex(&mut self, dex: i32, max_dex: i64) -> bool {
        let mut exp = self.object.getInt("민첩성경험치");
        exp += dex as i64;
        self.object.set("민첩성경험치", exp);

        let dex_stat = self.object.getInt("민첩성");
        let threshold = (dex_stat + 4) * 8;

        if exp >= threshold {
            self.object.set("민첩성경험치", 0);
            if dex_stat < max_dex {
                self.object.set("민첩성", dex_stat + 1);
                return true; // Leveled up
            }
        }
        false
    }

    /// Removes HP, returns true if died
    pub fn minus_hp(&mut self, damage: i64) -> bool {
        let mut hp = self.object.getInt("체력");
        hp -= damage;

        if hp <= 0 {
            self.object.set("체력", 0);
            return true; // Died
        }

        self.object.set("체력", hp);
        false
    }

    /// Removes MP
    pub fn minus_mp(&mut self, damage: i64) {
        let mut mp = self.object.getInt("내공");
        mp -= damage;

        if mp <= 0 {
            mp = 0;
        }

        self.object.set("내공", mp);
    }

    /// Unwear all equipment (reset bonuses)
    pub fn unwear_all(&mut self) {
        self.attpower = 0;
        self.armor = 0;
        self._str = 0;
        self._dex = 0;
        self._arm = 0;
        self._mp = 0;
        self._maxmp = 0;
        self._hp = 0;
        self._maxhp = 0;
        self._hit = 0;
        self._miss = 0;
        self._critical = 0;
        self._critical_chance = 0;
        self._magic_chance = 0;
        self._exp = 0;
        self.weapon_item = None;

        // Unmark all items as in use
        for obj in &self.object.objs {
            if let Ok(mut item) = obj.lock() {
                item.set("inUse", 0);
            }
        }
    }

    // ==================== Delegation to Object ====================

    /// Delegates to Object::set
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        self.object.set(key, value);
    }

    /// Delegates to Object::get
    pub fn get(&self, key: &str) -> Value {
        self.object.get(key)
    }

    /// Delegates to Object::getString
    pub fn get_string(&self, key: &str) -> String {
        self.object.getString(key)
    }

    /// Delegates to Object::getInt
    pub fn get_int(&self, key: &str) -> i64 {
        self.object.getInt(key)
    }

    /// Delegates to Object::getName
    pub fn get_name(&self) -> String {
        self.object.getName()
    }

    /// Delegates to Object::getNameA
    pub fn get_name_a(&self) -> String {
        self.object.getNameA()
    }

    /// Direct access to the attribute map (for command system compatibility)
    pub fn attr(&self) -> &std::collections::HashMap<String, Value> {
        &self.object.attr
    }

    /// Mutable access to attributes
    pub fn attr_mut(&mut self) -> &mut std::collections::HashMap<String, Value> {
        &mut self.object.attr
    }

    /// Get mutable reference to objs (inventory)
    pub fn objs_mut(&mut self) -> &mut Vec<Arc<Mutex<Object>>> {
        &mut self.object.objs
    }

    /// Get reference to objs (inventory)
    pub fn objs(&self) -> &Vec<Arc<Mutex<Object>>> {
        &self.object.objs
    }

    /// Get reference to temp attributes
    pub fn temp(&self) -> &HashMap<String, Value> {
        &self.object.temp
    }

    /// Get mutable reference to temp attributes
    pub fn temp_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.object.temp
    }

    /// Delegates to Object::han_iga
    pub fn han_iga(&self) -> String {
        self.object.han_iga()
    }

    /// Delegates to Object::han_obj
    pub fn han_obj(&self) -> String {
        self.object.han_obj()
    }

    /// Delegates to Object::han_un
    pub fn han_un(&self) -> String {
        self.object.han_un()
    }

    /// 제3자가 볼 때 / 자신이 '나 봐' 할 때의 설명. 파이썬 objs/player.getDesc(myself).
    /// 방파별호, 머리말, 꼬리말, 투명상태(호출처에서 필터), active_skills의 방어상태머리말.
    pub fn get_desc_for_look(&self, myself: bool) -> String {
        let act_str = if self.get_hp() <= 0 {
            "쓰러져 있습니다."
        } else {
            match self.act {
                ActState::Stand => "서 있습니다.",
                ActState::Rest => "운기조식을 하고 있습니다.",
                ActState::Fight => "목숨을 건 사투를 벌이고 있습니다.",
                ActState::Death => "쓰러져 있습니다.",
                ActState::Regeneration => "운기조식을 하고 있습니다.",
                ActState::Move => "서 있습니다.",
            }
        };
        let mut msg = String::new();
        if !myself {
            // 방파별호: \x1b[1m【%s】\x1b[0m
            let s = self.object.getString("방파별호");
            if !s.is_empty() {
                msg = format!("\x1b[1m【{}】\x1b[0m", s);
            }
            // 방어상태머리말: active_skills에 대해 skill.json의 방어상태머리말 이어붙임
            for a in &self.active_skills {
                let h = get_skill_defense_head(&a.name);
                if !h.is_empty() {
                    msg.push_str(&h);
                    msg.push(' ');
                }
            }
        }
        // 머리말
        let m = self.object.getString("머리말");
        if !m.is_empty() {
            msg.push_str(&m);
            msg.push(' ');
        }
        if myself {
            msg.push_str("당신이 ");
        } else {
            msg.push_str(&self.han_iga());
            msg.push(' ');
        }
        // 꼬리말
        let t = self.object.getString("꼬리말");
        if !t.is_empty() {
            msg.push_str(&t);
            msg.push(' ');
        }
        msg.push_str(&act_str);
        msg
    }

    /// Delegates to Object::checkAttr
    pub fn check_attr(&self, key: &str, attr: &str) -> bool {
        self.object.checkAttr(key, attr)
    }

    /// Delegates to Object::setAttr
    pub fn set_attr(&mut self, key: &str, attr: &str) {
        self.object.setAttr(key, attr);
    }

    /// Delegates to Object::delAttr
    pub fn del_attr(&mut self, key: &str, attr: &str) {
        self.object.delAttr(key, attr);
    }

    /// Get mutable reference to the inner Object
    pub fn object_mut(&mut self) -> &mut Object {
        &mut self.object
    }

    /// Get reference to the inner Object
    pub fn object_ref(&self) -> &Object {
        &self.object
    }

    // ==================== Secret Skills (비전) ====================

    /// Get the currently set secret skill (비전설정)
    /// This skill reduces damage from matching mobs by 50%
    pub fn get_vision_setting(&self) -> String {
        self.object.getString("비전설정")
    }

    /// Set the secret skill for training (비전설정)
    pub fn set_vision_setting(&mut self, skill_name: &str) {
        self.object.set("비전설정", skill_name);
    }

    /// Check if player has a specific secret skill learned (비전이름)
    pub fn has_secret_skill(&self, skill_name: &str) -> bool {
        let skills = self.object.getString("비전이름");
        if skills.is_empty() {
            return false;
        }
        skills.split(',').any(|s| s.trim() == skill_name)
    }

    /// Get list of learned secret skills
    pub fn get_secret_skills(&self) -> Vec<String> {
        let skills = self.object.getString("비전이름");
        if skills.is_empty() {
            Vec::new()
        } else {
            skills
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
    }

    /// Get currently training secret skill (비전수련)
    /// Returns (skill_name, progress_level)
    pub fn get_vision_training(&self) -> (String, i64) {
        let training = self.object.getString("비전수련");
        if training.is_empty() {
            (String::new(), 0)
        } else {
            let parts: Vec<&str> = training.split_whitespace().collect();
            let skill_name = parts.get(0).unwrap_or(&"").to_string();
            let progress = if parts.len() >= 2 {
                parts[1].parse().unwrap_or(0)
            } else {
                0
            };
            (skill_name, progress)
        }
    }

    /// Set training secret skill (비전수련설정)
    pub fn set_vision_training(&mut self, skill_name: &str, progress: i64) {
        let value = if progress > 0 {
            format!("{} {}", skill_name, progress)
        } else {
            skill_name.to_string()
        };
        self.object.set("비전수련", value);
    }

    /// Clear training secret skill (비전수련삭제)
    pub fn clear_vision_training(&mut self) {
        self.object.set("비전수련", "");
    }

    /// Add a learned secret skill to 비전이름
    pub fn add_secret_skill(&mut self, skill_name: &str) {
        let mut skills = self.get_secret_skills();
        if !skills.contains(&skill_name.to_string()) {
            skills.push(skill_name.to_string());
            let skills_str = skills.join(",");
            self.object.set("비전이름", skills_str);
        }
    }

    /// Check vision training progress - called during combat
    /// Python: checkVision(skill) - increases progress if skill matches training
    /// Returns (completed, message)
    pub fn check_vision_training(&mut self, skill_name: &str) -> (bool, String) {
        let (training_skill, progress) = self.get_vision_training();

        if training_skill.is_empty() {
            return (false, String::new());
        }

        if training_skill != skill_name {
            return (false, String::new());
        }

        // Training progress: ~1% chance per hit to complete
        // In Python: if randint(0, 99) > 1: progress += 1
        use rand::Rng;
        let roll = rand::thread_rng().gen_range(0..100);

        if roll > 1 {
            let new_progress = progress + 1;

            // Check if completed (threshold seems to be around 10-15 in Python)
            if new_progress >= 10 {
                // Training complete!
                self.clear_vision_training();
                self.add_secret_skill(skill_name);

                let msg = format!(
                    "\r\n\x1b[1m당신이 『\x1b[32m{}\x1b[37m』의 무공 구결을 깨우치기 시작합니다. \'ΔΨΞλΟ~\'\x1b[0;37m\r\n",
                    skill_name
                );
                return (true, msg);
            } else {
                self.set_vision_training(skill_name, new_progress);
            }
        }

        (false, String::new())
    }

    /// Get damage reduction against a mob skill based on vision setting
    /// Returns (multiplier, description) - e.g., (0.5, "damage halved")
    pub fn get_vision_damage_modifier(&self, mob_skill_name: &str) -> (f64, String) {
        let vision = self.get_vision_setting();
        if vision.is_empty() {
            return (1.0, String::new());
        }

        // Check if mob skill matches vision setting
        // 비전설정 can be:
        // - "비전무공" - matches any "비전XXX" skill
        // - Specific skill name like "강룡십팔장" - matches that exact skill
        if vision == "비전무공" {
            if mob_skill_name.starts_with("비전") {
                return (0.5, format!("비전({}) 때문에 피해가 절반입니다.", vision));
            }
        } else if vision == mob_skill_name {
            return (0.5, format!("비전({}) 때문에 피해가 절반입니다.", vision));
        }

        (1.0, String::new())
    }

    // ==================== Death & Reborn ====================

    /// Handle player death (Python: die() in objs/player.py:821)
    /// Sets death state, resets stats, drops items, enters coma
    pub fn die(&mut self) {
        self.act = ActState::Death;
        self._str = 0;
        self._dex = 0;
        self._arm = 0;
        // Note: autoMoveList would be cleared here if implemented

        self.unwear_all();
        self.drop_all_items_death();

        // Send death messages (matching Python output exactly)
        self.send_line("\r\n\x1b[1;37m당신이 쓰러집니다. '쿠웅~~ 철퍼덕~~'\x1b[0;37m");
        self.send_line("당신은 정신이 혼미합니다.");

        self.clear_targets_death();
        self.clear_skills();

        // Recalculate modifiers from skills (Python: for s in self.skills)
        let skill_list = self.get_string("무공이름");
        if !skill_list.is_empty() {
            for skill_name in skill_list.split(',') {
                let skill_name = skill_name.trim();
                if !skill_name.is_empty() {
                    // In full implementation, would load skill data and add modifiers
                    // For now, this is simplified
                }
            }
        }

        // Enter coma state (input_to(self.coma) in Python)
        // The actual coma handler is implemented at the network level
    }

    /// Process death progression through reborn (Python: doDeath() in objs/player.py:2248)
    /// Returns messages to send to player
    pub fn do_death(&mut self) -> Vec<String> {
        let mut messages = Vec::new();

        match self.step_death {
            0 => {
                messages.push("\r\n기혈이 거꾸로 돌며 정신이 혼미해 집니다.".to_string());
                self.step_death = 1;
            }
            1 => {
                messages.push("\r\n누군가가 당신 주위를 어슬렁 거립니다.".to_string());
                self.step_death = 2;
            }
            2 => {
                messages.push("\r\n장의사가 당신을 어깨에 메고 낑낑대며 걸어갑니다.".to_string());
                self.step_death = 3;
            }
            3 => {
                // Python: enterRoom('낙양성:7', '사망', '사망')
                // For now, just send message - actual room move handled by caller
                messages.push("\r\n장의사가 당신을 어깨에 메고 낑낑대며 걸어갑니다.".to_string());
                self.step_death = 4;
            }
            4 => {
                messages
                    .push("\r\n코끝을 찌르는 향냄새에 정신을 차려보니 장의사 내부다.".to_string());
                messages.push("당신은 죽었다가 다시 살아났습니다.".to_string());

                // Reset to alive state
                self.act = ActState::Stand;
                self.step_death = 0;

                // Full heal
                let max_hp = self.get_int("최대체력");
                self.set("체력", max_hp);
                let max_mp = self.get_int("최대내공");
                self.set("내공", max_mp);
            }
            _ => {
                self.step_death = 0;
            }
        }

        messages
    }

    /// Check if player is in coma/death state
    pub fn is_in_coma(&self) -> bool {
        self.act == ActState::Death
    }

    /// Handle coma input (blocks all commands except blank/empty)
    /// Returns true if input should be blocked
    pub fn coma_check(&self, input: &str) -> bool {
        if self.is_in_coma() && !input.trim().is_empty() {
            return true;
        }
        false
    }

    /// Drop all items on death (Python: dropAllItem())
    pub fn drop_all_items_death(&mut self) {
        // Clear all items from inventory
        // In full implementation, items would be moved to the room
        self.object.objs.clear();
        self.object.inv_stack.clear();
    }

    /// Clear all combat targets (separate method for death handling)
    pub fn clear_targets_death(&mut self) {
        self.targets.clear();
    }

    /// Get death progress step (0-4)
    pub fn get_death_step(&self) -> i32 {
        self.step_death
    }

    /// Set death progress step
    pub fn set_death_step(&mut self, step: i32) {
        self.step_death = step.clamp(0, 4);
    }

    /// Record time of death
    pub fn record_time_of_death(&mut self) {
        self.time_of_death = Some(std::time::Instant::now());
    }

    /// Get seconds since death
    pub fn get_seconds_since_death(&self) -> Option<u64> {
        self.time_of_death.map(|t| t.elapsed().as_secs())
    }
}

/// SendLine trait for sending messages to a body
pub trait SendLine {
    /// Sends a line to the target
    fn send_line(&self, line: &str);

    /// Writes raw data to the target
    fn write(&self, data: &[u8]);
}

impl SendLine for Body {
    fn send_line(&self, line: &str) {
        // Default implementation does nothing
        let _ = line;
    }

    fn write(&self, _data: &[u8]) {
        // Default implementation does nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_new() {
        let body = Body::new();
        assert_eq!(body.act, ActState::Stand);
        assert_eq!(body.tick, 0);
        assert!(body.skill.is_none());
        assert!(body.targets.is_empty());
    }

    #[test]
    fn test_act_state_from_i32() {
        assert_eq!(ActState::from_i32(0), ActState::Stand);
        assert_eq!(ActState::from_i32(1), ActState::Fight);
        assert_eq!(ActState::from_i32(2), ActState::Death);
        assert_eq!(ActState::from_i32(3), ActState::Regeneration);
        assert_eq!(ActState::from_i32(4), ActState::Rest);
        assert_eq!(ActState::from_i32(5), ActState::Move);
        assert_eq!(ActState::from_i32(999), ActState::Stand);
    }

    #[test]
    fn test_act_state_to_i32() {
        assert_eq!(ActState::Stand.to_i32(), 0);
        assert_eq!(ActState::Fight.to_i32(), 1);
        assert_eq!(ActState::Death.to_i32(), 2);
        assert_eq!(ActState::Regeneration.to_i32(), 3);
        assert_eq!(ActState::Rest.to_i32(), 4);
        assert_eq!(ActState::Move.to_i32(), 5);
    }

    #[test]
    fn test_skill_level_from_name() {
        assert_eq!(SkillLevel::from_name("초급"), Some(SkillLevel::Primary));
        assert_eq!(
            SkillLevel::from_name("중급"),
            Some(SkillLevel::Intermediate)
        );
        assert_eq!(SkillLevel::from_name("상급"), Some(SkillLevel::Advanced));
        assert_eq!(SkillLevel::from_name("고급"), Some(SkillLevel::High));
        assert_eq!(SkillLevel::from_name("특급"), Some(SkillLevel::Special));
        assert_eq!(SkillLevel::from_name("절정"), Some(SkillLevel::Peak));
        assert_eq!(
            SkillLevel::from_name("초절정"),
            Some(SkillLevel::Transcendent)
        );
        assert_eq!(SkillLevel::from_name("없음"), None);
    }

    #[test]
    fn test_skill_level_name() {
        assert_eq!(SkillLevel::Primary.name(), "초급");
        assert_eq!(SkillLevel::Intermediate.name(), "중급");
        assert_eq!(SkillLevel::Advanced.name(), "상급");
        assert_eq!(SkillLevel::High.name(), "고급");
        assert_eq!(SkillLevel::Special.name(), "특급");
        assert_eq!(SkillLevel::Peak.name(), "절정");
        assert_eq!(SkillLevel::Transcendent.name(), "초절정");
    }

    #[test]
    fn test_init_body() {
        let mut body = Body::new();
        body.init_body();

        assert_eq!(body.get_int("레벨"), 1);
        assert_eq!(body.get_int("체력"), 450);
        assert_eq!(body.get_int("최고체력"), 450);
        assert_eq!(body.get_int("힘"), 15);
        assert_eq!(body.get_int("맷집"), 15);
        assert_eq!(body.get_int("민첩성"), 0);
        assert_eq!(body.get_int("은전"), 100000);
        assert_eq!(body.get_int("내공"), 18);
        assert_eq!(body.get_int("최고내공"), 18);
        assert_eq!(body.get_int("나이"), 18);
    }

    #[test]
    fn test_get_str() {
        let mut body = Body::new();
        body.init_body();
        assert_eq!(body.get_str(), 15);

        body._str = 5;
        assert_eq!(body.get_str(), 20);

        body._str = -20; // Should not go below 0
        assert_eq!(body.get_str(), 0);
    }

    #[test]
    fn test_get_dex() {
        let mut body = Body::new();
        body.init_body();
        assert_eq!(body.get_dex(), 0);

        body._dex = 10;
        assert_eq!(body.get_dex(), 10);

        body._dex = -20; // Should not go below 0
        assert_eq!(body.get_dex(), 0);
    }

    #[test]
    fn test_get_hp() {
        let mut body = Body::new();
        body.init_body();
        assert_eq!(body.get_hp(), 450);
    }

    #[test]
    fn test_get_max_hp() {
        let mut body = Body::new();
        body.init_body();
        // Max HP = base + arm * 30 = 450 + 15 * 30 = 900
        assert_eq!(body.get_max_hp(), 900);
    }

    #[test]
    fn test_get_mp() {
        let mut body = Body::new();
        body.init_body();
        assert_eq!(body.get_mp(), 18);

        body._mp = 50; // 50% bonus
        assert_eq!(body.get_mp(), 27); // 18 + (18 * 50 / 100) = 27
    }

    #[test]
    fn test_get_max_mp() {
        let mut body = Body::new();
        body.init_body();
        assert_eq!(body.get_max_mp(), 18);

        body._maxmp = 10;
        assert_eq!(body.get_max_mp(), 28); // 18 + 10
    }

    #[test]
    fn test_is_movable() {
        let mut body = Body::new();
        assert!(body.is_movable());

        body.act = ActState::Fight;
        assert!(!body.is_movable());

        body.act = ActState::Rest;
        assert!(!body.is_movable());

        body.act = ActState::Stand;
        assert!(body.is_movable());
    }

    #[test]
    fn test_get_total_exp() {
        let mut body = Body::new();
        body.init_body(); // Level 1

        // ((1 * 1) / 3 + 30) * (1 + 4) = (0 + 30) * 5 = 150
        assert_eq!(body.get_total_exp(), 150);
    }

    #[test]
    fn test_add_exp_no_level_up() {
        let mut body = Body::new();
        body.init_body(); // Level 1, needs 150 exp

        assert!(!body.add_exp(100)); // Not enough to level up
        assert_eq!(body.get_int("레벨"), 1);
        assert_eq!(body.get_int("현재경험치"), 100);
    }

    #[test]
    fn test_add_exp_level_up() {
        let mut body = Body::new();
        body.init_body(); // Level 1, needs 150 exp

        assert!(body.add_exp(150)); // Level up!
        assert_eq!(body.get_int("레벨"), 2);
        assert_eq!(body.get_int("현재경험치"), 0);
        // Check that max HP increased
        assert!(body.get_int("최고체력") > 450);
    }

    #[test]
    fn test_minus_hp_survive() {
        let mut body = Body::new();
        body.init_body();

        assert!(!body.minus_hp(100)); // Survived
        assert_eq!(body.get_hp(), 350);
    }

    #[test]
    fn test_minus_hp_die() {
        let mut body = Body::new();
        body.init_body();

        assert!(body.minus_hp(500)); // Died
        assert_eq!(body.get_hp(), 0);
    }

    #[test]
    fn test_minus_mp() {
        let mut body = Body::new();
        body.init_body();

        body.minus_mp(10);
        assert_eq!(body.get_int("내공"), 8);

        body.minus_mp(20); // Should go to 0, not negative
        assert_eq!(body.get_int("내공"), 0);
    }

    #[test]
    fn test_get_hp_string() {
        let mut body = Body::new();
        body.init_body();

        // After init_body(), HP is 450, max_hp is 900 (450 + 15*30 armor bonus)
        // So ratio is 50% -> yellow bar
        let s = body.get_hp_string();
        assert!(s.contains("\x1b[33m")); // Yellow bar at 50%

        // Set to 100% HP
        body.object.set("체력", 900);
        let s = body.get_hp_string();
        assert!(s.contains("\x1b[32m")); // Green bar at 100%

        // Set to 10% HP
        body.object.set("체력", 90);
        let s = body.get_hp_string();
        assert!(s.contains("\x1b[31m")); // Red bar at 10%
    }

    #[test]
    fn test_skill_training() {
        let mut body = Body::new();

        body.set_skill_training("test_skill", 1, 0);
        let training = body.get_skill_training("test_skill");
        assert_eq!(training, Some(SkillTraining::new(1, 0)));

        // Skill up with enough experience
        let leveled_up = body.skill_up(Some("test_skill"), 10);
        assert!(!leveled_up); // Not enough exp yet (1/10)

        for _ in 0..8 {
            body.skill_up(Some("test_skill"), 10);
        }
        // Now we have 9 exp, still not enough
        let leveled_up = body.skill_up(Some("test_skill"), 10);
        assert!(leveled_up); // Should level up now (10/10)

        let training = body.get_skill_training("test_skill");
        assert_eq!(training, Some(SkillTraining::new(2, 0)));
    }

    #[test]
    fn test_get_skill() {
        let mut body = Body::new();

        assert!(body.get_skill("기공격").is_some());
        assert_eq!(body.skill, Some("기공격".to_string()));

        // Last skill should be cached
        assert_eq!(body.last_skill, Some("기공격".to_string()));
    }

    #[test]
    fn test_stop_skill() {
        let mut body = Body::new();
        body.skill = Some("기공격".to_string());

        body.stop_skill();
        assert!(body.skill.is_none());
        assert_eq!(body.last_skill, Some("기공격".to_string()));
    }

    #[test]
    fn test_clear_skills() {
        let mut body = Body::new();
        body.skill = Some("기공격".to_string());
        body.active_skills
            .push(ActiveSkill::new("방어무공".to_string(), 10));

        body.clear_skills();
        assert!(body.skill.is_none());
        assert!(body.active_skills.is_empty());
    }

    #[test]
    fn test_unwear_all() {
        let mut body = Body::new();
        body._str = 10;
        body.attpower = 100;
        body.armor = 50;

        body.unwear_all();
        assert_eq!(body._str, 0);
        assert_eq!(body.attpower, 0);
        assert_eq!(body.armor, 0);
    }

    #[test]
    fn test_get_item_weight() {
        let mut body = Body::new();

        // Create a test item
        let item = Arc::new(Mutex::new(Object::new()));
        item.lock().unwrap().set("이름", "검");
        item.lock().unwrap().set("무게", 50);
        body.object.objs.push(item);

        assert_eq!(body.get_item_weight(), 50);
    }

    #[test]
    fn test_get_item_count() {
        let mut body = Body::new();

        // Create test items
        for i in 0..3 {
            let item = Arc::new(Mutex::new(Object::new()));
            let name = format!("아이템{}", i);
            item.lock().unwrap().set("이름", name.as_str());
            body.object.objs.push(item);
        }

        assert_eq!(body.get_item_count(), 3);
    }

    #[test]
    fn test_get_inven_item_count() {
        let mut body = Body::new();

        // Create a used item (not counted)
        let item1 = Arc::new(Mutex::new(Object::new()));
        item1.lock().unwrap().set("이름", "장착한아이템");
        item1.lock().unwrap().set("inUse", 1);
        body.object.objs.push(item1);

        // Create an unused item (counted)
        let item2 = Arc::new(Mutex::new(Object::new()));
        item2.lock().unwrap().set("이름", "인벤아이템");
        item2.lock().unwrap().set("inUse", 0);
        body.object.objs.push(item2);

        // Create a hidden item (not counted)
        let item3 = Arc::new(Mutex::new(Object::new()));
        item3.lock().unwrap().set("이름", "숨겨진아이템");
        item3.lock().unwrap().set("아이템속성", "출력안함");
        body.object.objs.push(item3);

        assert_eq!(body.get_inven_item_count(), 1);
    }

    #[test]
    fn test_add_arm() {
        let mut body = Body::new();
        body.init_body(); // 맷집 = 15

        // Add armor experience
        body.add_arm(100); // Threshold is (15 - 10) * 20 = 100

        // Should level up
        assert_eq!(body.get_int("맷집"), 16);
        assert_eq!(body.get_int("맷집경험치"), 0);
    }

    #[test]
    fn test_add_str() {
        let mut body = Body::new();
        body.init_body(); // 힘 = 15

        // Add strength experience
        let leveled_up = body.add_str(100, true); // Threshold is (15 - 10) * 20 = 100

        assert!(leveled_up);
        assert_eq!(body.get_int("힘"), 16);
        assert_eq!(body.get_int("힘경험치"), 0);
    }

    #[test]
    fn test_add_dex() {
        let mut body = Body::new();
        body.init_body(); // 민첩성 = 0

        // Add dexterity experience
        let leveled_up = body.add_dex(32, 999); // Threshold is (0 + 4) * 8 = 32

        assert!(leveled_up);
        assert_eq!(body.get_int("민첩성"), 1);
        assert_eq!(body.get_int("민첩성경험치"), 0);
    }
}
