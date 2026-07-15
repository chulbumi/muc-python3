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
    pub level: i64,
    /// Skill experience points
    pub exp: u32,
}

impl SkillTraining {
    pub fn new(level: i64, exp: u32) -> Self {
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
    /// HP percentage bonus/penalty
    pub hp_bonus: i32,
    /// Max HP bonus/penalty
    pub max_hp_bonus: i32,
    /// Category blocked by this effect (`Skill.getAntiType()` in Python)
    pub anti_type: String,
    pub category: String,
    pub recovery_percent: i64,
    pub recovery_script: String,
    pub release_script: String,
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
            hp_bonus: 0,
            max_hp_bonus: 0,
            anti_type: String::new(),
            category: String::new(),
            recovery_percent: 0,
            recovery_script: String::new(),
            release_script: String::new(),
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
#[derive(Debug, Clone)]
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
    /// `무공이름`/`무공숙련도` 객체 속성을 런타임 맵으로 복원했는지 여부.
    pub skill_state_loaded: bool,
    /// Item skill training map
    pub item_skill_map: HashMap<String, u32>,
    /// Python itemSkillMap dict insertion order for save compatibility.
    pub item_skill_order: Vec<String>,
    /// Skill cooldown tracking (skill_name -> last_cast_timestamp)
    pub skill_cooldowns: HashMap<String, i64>,
    /// 범용 스크립트: 아이템확인에서 설정, 옵션출력/옵션확인 등에서 사용. Complete 시 클리어.
    pub script_temp_item: Option<Arc<Mutex<Object>>>,
    /// 도착 쪽지. 키 "메모:발신자이름", 값 MemoRecord. load/save 시 JSON 루트의 "메모:xxx"와 연동.
    pub memos: HashMap<String, MemoRecord>,
    /// Python `Player.talkHistory`: 현재 접속의 전음 대화 기록 (저장하지 않음)
    pub talk_history: Vec<String>,
    /// Death progression step (0-4, for doDeath() progression)
    pub step_death: i32,
    /// Time of death (for regen timing)
    pub time_of_death: Option<std::time::Instant>,
    /// Corpse duration (seconds before becoming corpse)
    pub corpse_duration: u64,
    /// Regeneration duration (seconds before regen)
    pub regen_duration: u64,
    /// Current difficulty level (0 = base zone, 1-7 = difficulty zones)
    pub difficulty: u8,
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
            skill_state_loaded: false,
            item_skill_map: HashMap::new(),
            item_skill_order: Vec::new(),
            skill_cooldowns: HashMap::new(),
            script_temp_item: None,
            memos: HashMap::new(),
            talk_history: Vec::new(),
            step_death: 0,
            time_of_death: None,
            corpse_duration: 60, // Default 60 seconds
            regen_duration: 300, // Default 5 minutes
            difficulty: 0,       // Base zone by default
        }
    }

    /// Creates a Body from an existing Object
    pub fn from_object(object: Object) -> Self {
        let mut body = Self::new();
        body.object = object;
        body
    }

    // ==================== Difficulty Methods ====================

    /// Get current difficulty level
    pub fn get_difficulty(&self) -> u8 {
        self.difficulty
    }

    /// Set difficulty level
    pub fn set_difficulty(&mut self, difficulty: u8) {
        self.difficulty = difficulty.min(7); // Cap at 7
    }

    /// Check if player can enter a difficulty zone
    pub fn can_enter_difficulty(&self, target_difficulty: u8) -> bool {
        use crate::world::DifficultyConfig;
        let min_level = DifficultyConfig::min_level_for_difficulty(target_difficulty);
        self.object.getInt("레벨") >= min_level
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
            // Python `//` floors negative percentage modifiers; Rust `/`
            // truncates toward zero and differs whenever the product is not
            // divisible by 100.
            base + (base * self._mp as i64).div_euclid(100)
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

    // ==================== Weapon Mastery Methods ====================

    /// Gets weapon type (1-5) from equipped weapon
    /// Python: self.getWeapon()['무기종류']
    /// Returns 1 (기본) if no weapon equipped
    pub fn get_weapon_type(&self) -> i64 {
        // 무기종류: 1=도, 2=창, 3=검, 4=봉, 5=기타
        // 현재 장착한 무기에서 가져와야 함
        // 간단히 구현: weaponItem에서 무기종류 가져오기
        if let Some(weak) = &self.weapon_item {
            if let Some(arc) = weak.upgrade() {
                if let Ok(obj) = arc.lock() {
                    return obj.getInt("무기종류").max(1);
                }
            }
        }
        1 // 기본값: 주먹 = 1
    }

    /// Gets mastery level for a weapon type
    /// Python: getInt(self['%d 숙련도' % weapon_type])
    pub fn get_mastery(&self, weapon_type: i64) -> i64 {
        let key = format!("{} 숙련도", weapon_type);
        let base = self.object.getInt(&key);
        // 숙련도상승 버프 확인
        if !self.object.getString("숙련도상승").is_empty() {
            base + 2000
        } else {
            base
        }
    }

    /// Gets weapon skill level (기량) from equipped weapon
    /// Python: getInt(item['기량'])
    /// Returns 0 for 주먹 (fist)
    pub fn get_weapon_skill(&self) -> i64 {
        if let Some(weak) = &self.weapon_item {
            if let Some(arc) = weak.upgrade() {
                if let Ok(obj) = arc.lock() {
                    return obj.getInt("기량");
                }
            }
        }
        0 // 주먹은 기량 0
    }

    /// Calculates mastery difference for damage calculation
    /// Python: ss = s1 - s2 where s1 = weapon skill, s2 = mastery
    /// Returns max(0, weapon_skill - mastery)
    pub fn get_mastery_diff(&self) -> i64 {
        let weapon_type = self.get_weapon_type();
        let s1 = self.get_weapon_skill(); // 무기 기량
        let s2 = self.get_mastery(weapon_type); // 숙련도 (`숙련도상승` 포함)
        let ss = s1 - s2;
        ss.max(0)
    }

    /// Gets weapon display name for combat messages
    /// Python: makeFightScript() uses mstr = '[36m주먹[37m' or weapon.getNameA()
    pub fn get_weapon_name(&self) -> String {
        if let Some(weak) = &self.weapon_item {
            if let Some(arc) = weak.upgrade() {
                if let Ok(obj) = arc.lock() {
                    return obj.getNameA();
                }
            }
        }
        // Default: 주먹 with cyan color
        "\x1b[36m주먹\x1b[37m".to_string()
    }

    /// Gets fight script type from weapon
    /// Python: getWeaponFightType() returns getWeapon()['전투스크립']
    /// Used to select the combat message script (주먹, 검, 도, 등)
    pub fn get_fight_script_type(&self) -> String {
        if let Some(weak) = &self.weapon_item {
            if let Some(arc) = weak.upgrade() {
                if let Ok(obj) = arc.lock() {
                    let script_type = obj.getString("전투스크립");
                    if !script_type.is_empty() {
                        return script_type;
                    }
                }
            }
        }
        // Default: 주먹
        "주먹".to_string()
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

    /// Python `Body.loadSkillList()`와 `Body.loadSkillUp()`에 해당한다.
    ///
    /// Python JSON 배열은 Rust `Object::attr`에서 `|` 구분 문자열로 보존된다.
    /// 과거 Rust 저장본의 쉼표 구분 문자열도 읽되, 런타임 상태는 Python과
    /// 동일하게 `skill_list`/`skill_map`에 다시 구성한다.
    pub fn load_skill_state_from_attrs(&mut self) {
        fn split_entries(raw: &str) -> Vec<&str> {
            if raw.contains('|') {
                raw.split('|').collect()
            } else if raw.contains(',') {
                raw.split(',').collect()
            } else if raw.is_empty() {
                Vec::new()
            } else {
                vec![raw]
            }
        }

        self.skill_list.clear();
        let skill_names = self.get_string("무공이름");
        for name in split_entries(&skill_names) {
            let name = name.trim();
            if !name.is_empty() {
                self.skill_list.push(name.to_string());
            }
        }

        self.skill_map.clear();
        let training_entries = self.get_string("무공숙련도");
        for entry in split_entries(&training_entries) {
            let words: Vec<&str> = entry.split_whitespace().collect();
            if words.len() < 3 {
                continue;
            }
            let Ok(level) = words[1].parse::<i64>() else {
                continue;
            };
            let Ok(exp) = words[2].parse::<u32>() else {
                continue;
            };
            self.skill_map
                .insert(words[0].to_string(), SkillTraining::new(level, exp));
        }
        self.item_skill_map.clear();
        self.item_skill_order.clear();
        let item_training = self.get_string("무공이름수련리스트");
        for entry in split_entries(&item_training) {
            let words = entry.split_whitespace().collect::<Vec<_>>();
            if words.len() < 2 {
                continue;
            }
            let Some(value) = words.last().and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };
            let name = words[..words.len() - 1].join(" ");
            if !self.item_skill_order.contains(&name) {
                self.item_skill_order.push(name.clone());
            }
            self.item_skill_map.insert(name, value);
        }
        self.skill_state_loaded = true;
    }

    /// Python 저장 직전의 `buildSkillList()`/`buildSkillUp()`과 같이 런타임
    /// 무공 상태를 객체 속성 해시맵으로 되돌린다.
    pub fn sync_skill_state_to_attrs(&mut self) {
        self.object.set("무공이름", self.skill_list.join("|"));

        // 기존 배열 순서를 가능한 한 유지하고, 새 항목은 skill_list 순서 뒤에
        // 이름순으로 붙인다. 숙련도 조회 자체는 이름 키 기반이다.
        let previous = self.get_string("무공숙련도");
        let mut ordered_names = Vec::new();
        for entry in previous.split(['|', ',']) {
            let Some(name) = entry.split_whitespace().next() else {
                continue;
            };
            if self.skill_map.contains_key(name)
                && !ordered_names.iter().any(|existing| existing == name)
            {
                ordered_names.push(name.to_string());
            }
        }
        for name in &self.skill_list {
            if self.skill_map.contains_key(name)
                && !ordered_names.iter().any(|existing| existing == name)
            {
                ordered_names.push(name.clone());
            }
        }
        let mut remaining: Vec<_> = self
            .skill_map
            .keys()
            .filter(|name| !ordered_names.iter().any(|existing| existing == *name))
            .cloned()
            .collect();
        remaining.sort();
        ordered_names.extend(remaining);

        let training = ordered_names
            .into_iter()
            .filter_map(|name| {
                self.skill_map
                    .get(&name)
                    .map(|value| format!("{} {} {}", name, value.level, value.exp))
            })
            .collect::<Vec<_>>()
            .join("|");
        self.object.set("무공숙련도", training);
        let mut item_names = self.item_skill_order.clone();
        let mut remaining = self
            .item_skill_map
            .keys()
            .filter(|name| !item_names.contains(name))
            .cloned()
            .collect::<Vec<_>>();
        remaining.sort();
        item_names.extend(remaining);
        let item_training = item_names
            .into_iter()
            .filter_map(|name| {
                self.item_skill_map
                    .get(&name)
                    .map(|value| format!("{name} {value}"))
            })
            .collect::<Vec<_>>()
            .join("|");
        self.object.set("무공이름수련리스트", item_training);
        self.skill_state_loaded = true;
    }

    /// Python `Body.loadSkills()`: restore active defense effects from the
    /// `방어무공시전` object attribute and re-apply their modifiers.
    pub fn load_active_skills_from_attrs(&mut self) {
        // Repeated loads must not stack the same temporary modifiers.
        for effect in self.active_skills.drain(..) {
            self._str -= effect.str_bonus;
            self._dex -= effect.dex_bonus;
            self._arm -= effect.arm_bonus;
            self._mp -= effect.mp_bonus;
            self._maxmp -= effect.max_mp_bonus;
            self._hp -= effect.hp_bonus;
            self._maxhp -= effect.max_hp_bonus;
        }

        let raw = self.get_string("방어무공시전");
        if raw.is_empty() {
            return;
        }
        for entry in raw.split(['|', ',']) {
            let words = entry.split_whitespace().collect::<Vec<_>>();
            if words.len() < 2 {
                continue;
            }
            let Ok(remaining) = words[1].parse::<i32>() else {
                continue;
            };
            let Some(skill) = crate::world::get_skill(words[0]) else {
                continue;
            };
            let mut effect = ActiveSkill::new(skill.name, remaining);
            effect.str_bonus = skill.str_bonus as i32;
            effect.dex_bonus = skill.dex_bonus as i32;
            effect.arm_bonus = skill.arm_bonus as i32;
            effect.mp_bonus = skill.mp_bonus as i32;
            effect.max_mp_bonus = skill.max_mp_bonus as i32;
            effect.hp_bonus = skill.hp_bonus as i32;
            effect.max_hp_bonus = skill.max_hp_bonus as i32;
            effect.anti_type = skill.deny;
            effect.category = skill.category;
            effect.recovery_percent = skill.recovery_percent;
            effect.recovery_script = skill.recovery_script;
            effect.release_script = skill.release_script;
            self._str += effect.str_bonus;
            self._dex += effect.dex_bonus;
            self._arm += effect.arm_bonus;
            self._mp += effect.mp_bonus;
            self._maxmp += effect.max_mp_bonus;
            self._hp += effect.hp_bonus;
            self._maxhp += effect.max_hp_bonus;
            self.active_skills.push(effect);
        }
    }

    /// Python `Body.buildSkills()`: keep active effect state in the object
    /// property hashmap so Rhai and Python-compatible JSON can access it.
    pub fn sync_active_skills_to_attrs(&mut self) {
        let value = self
            .active_skills
            .iter()
            .map(|effect| format!("{} {}", effect.name, effect.start_time))
            .collect::<Vec<_>>()
            .join("|");
        self.object.set("방어무공시전", value);
    }

    /// 관리자 무공제거용: 같은 이름의 첫 활성 방어 효과를 해제한다.
    /// Python은 목록에서만 제거해 보너스를 남기는 오류가 있었으므로 Rust는
    /// 효과 수치도 함께 되돌리고 저장 속성을 동기화한다.
    pub fn remove_active_skill_by_name(&mut self, name: &str) -> bool {
        let Some(index) = self
            .active_skills
            .iter()
            .position(|skill| skill.name == name)
        else {
            return false;
        };
        let effect = self.active_skills.remove(index);
        self._str -= effect.str_bonus;
        self._dex -= effect.dex_bonus;
        self._arm -= effect.arm_bonus;
        self._mp -= effect.mp_bonus;
        self._maxmp -= effect.max_mp_bonus;
        self._hp -= effect.hp_bonus;
        self._maxhp -= effect.max_hp_bonus;
        self.sync_active_skills_to_attrs();
        true
    }

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
    pub fn set_skill_training(&mut self, skill_name: &str, level: i64, exp: u32) {
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
        if self.get_int("전직") < 2 {
            let level = self.get_int("레벨");
            if level >= 2_000 || level % 10 == 0 {
                self.set("특성치", self.get_int("특성치") + 1);
            }
        }
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
        self.skill_map.clear();
        self.skill_list.clear();
        self.skill_state_loaded = true;
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
    pub fn add_arm(&mut self, arm: i32) -> bool {
        let mut exp = self.object.getInt("맷집경험치");
        exp += arm as i64;
        self.object.set("맷집경험치", exp);

        let armor_val = self.object.getInt("맷집");
        let threshold = (armor_val - 10) * 20;

        if exp >= threshold && threshold > 0 {
            self.object.set("맷집경험치", 0);
            self.object.set("맷집", armor_val + 1);
            return true;
        }
        false
    }

    /// Adds strength experience
    pub fn add_str(&mut self, str_val: i32, check: bool) -> bool {
        let mut exp = self.object.getInt("힘경험치");
        exp += str_val as i64;
        self.object.set("힘경험치", exp);

        if check {
            let str_stat = self.object.getInt("힘");
            let threshold = (str_stat - 10) * 20;

            if exp >= threshold {
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

    /// Python `Body.weaponSkillUp`: every attempted player attack advances
    /// the mastery experience of the currently equipped weapon type.
    pub fn weapon_skill_up(&mut self, amount: i64) -> bool {
        let weapon_type = self.get_weapon_type();
        let mastery_key = format!("{weapon_type} 숙련도");
        let exp_key = format!("{weapon_type} 숙련도경험치");
        let mastery = self.get_int(&mastery_key);
        let experience = self.get_int(&exp_key).saturating_add(amount);
        self.set(&exp_key, experience);
        if experience >= (mastery + 5).saturating_mul(7) {
            self.set(&mastery_key, mastery + 1);
            self.set(&exp_key, 0_i64);
            true
        } else {
            false
        }
    }

    /// Python `Body.addAnger`, called after every successful ordinary mob hit.
    /// Returns true only for the transition that emits the 100-point message.
    pub fn add_anger(&mut self) -> bool {
        let anger = self.get_int("분노");
        if anger >= 600 {
            return false;
        }
        let next = anger + 1;
        self.set("분노", next);
        next == 100
    }

    /// Python `Body.checkItemSkill` for the currently equipped weapon.
    /// The caller supplies the inclusive 0..99 roll so combat tests can lock
    /// the probability boundary.  Returns the newly discovered skill name.
    pub fn check_item_skill_with_roller(
        &mut self,
        roll: &mut impl FnMut() -> i64,
    ) -> Option<String> {
        let weapon = self.weapon_item.as_ref()?.upgrade()?;
        let (weapon_name, declarations, consume_after_learning, armor, attack) = {
            let weapon = weapon.lock().ok()?;
            let raw = weapon.getString("무공이름");
            if raw.is_empty() {
                return None;
            }
            (
                weapon.getString("이름"),
                raw.split('|').map(str::to_string).collect::<Vec<_>>(),
                weapon.checkAttr("아이템속성", "무공배운후소멸"),
                weapon.getInt("방어력"),
                weapon.getInt("공격력"),
            )
        };
        if !self.item_skill_map.contains_key(&weapon_name) {
            self.item_skill_order.push(weapon_name.clone());
        }
        let counter = self.item_skill_map.entry(weapon_name.clone()).or_insert(0);
        *counter = counter.saturating_add(1);
        let count = *counter;
        for declaration in declarations {
            let words = declaration.split_whitespace().collect::<Vec<_>>();
            if words.len() < 5 || self.skill_list.iter().any(|name| name == words[0]) {
                continue;
            }
            let tendency = words[1];
            let personality = self.get_string("성격");
            if tendency != "정사"
                && personality != tendency
                && personality != "기인"
                && personality != "선인"
            {
                continue;
            }
            let required = words[2].parse::<u32>().unwrap_or(0);
            let interval = words[3].parse::<u32>().unwrap_or(0);
            let chance = words[4].parse::<i64>().unwrap_or(0);
            if count < required || interval == 0 || count % interval != 0 {
                continue;
            }
            if count < 2_500_000 && roll() > chance {
                continue;
            }
            let skill_name = words[0].to_string();
            self.skill_list.push(skill_name.clone());
            self.item_skill_map.insert(weapon_name.clone(), 0);
            self.sync_skill_state_to_attrs();
            if consume_after_learning {
                if let Ok(mut item) = weapon.lock() {
                    item.set("inUse", 0_i64);
                }
                self.armor = self.armor.saturating_sub(armor as i32);
                self.attpower = self.attpower.saturating_sub(attack as i32);
                self.object.remove(&weapon);
                self.weapon_item = None;
            }
            return Some(skill_name);
        }
        None
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
                item.attr.remove("inUse");
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
        msg.push_str(act_str);
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
        self.get_secret_skills()
            .iter()
            .any(|learned| learned == skill_name)
    }

    /// Get list of learned secret skills
    pub fn get_secret_skills(&self) -> Vec<String> {
        let skills = self.object.getString("비전이름");
        if skills.is_empty() {
            Vec::new()
        } else {
            skills
                // JSON 배열은 로드 시 `|`로 보존된다. 쉼표는 기존 Rust
                // 저장값을 읽기 위한 레거시 호환 구분자다.
                .split(['|', ','])
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
            let skill_name = parts.first().copied().unwrap_or("").to_string();
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
            let skills_str = skills.join("|");
            self.object.set("비전이름", skills_str);
        }
    }

    /// Check vision training progress - called during combat
    /// Python: checkVision(skill) - increases progress if skill matches training
    /// Returns whether training completed; presentation belongs to Rhai.
    pub fn check_vision_training(&mut self, skill_name: &str) -> bool {
        self.check_vision_training_with_roll(skill_name, &mut || {
            rand::Rng::gen_range(&mut rand::thread_rng(), 0..=99)
        })
    }

    pub fn check_vision_training_with_roll(
        &mut self,
        skill_name: &str,
        roll: &mut impl FnMut() -> i64,
    ) -> bool {
        let (training_skill, progress) = self.get_vision_training();

        if training_skill.is_empty() {
            return false;
        }

        if training_skill != skill_name {
            return false;
        }

        // Python completes immediately on 0 or 1; every other roll merely
        // increments the persisted progress counter without a threshold.
        if roll() > 1 {
            self.set_vision_training(skill_name, progress + 1);
            return false;
        }
        self.clear_vision_training();
        self.add_secret_skill(skill_name);
        true
    }

    /// Get damage reduction against a mob skill based on vision setting
    pub fn get_vision_damage_modifier(&self, mob_skill_name: &str) -> f64 {
        let vision = self.get_vision_setting();
        if vision.is_empty() {
            return 1.0;
        }

        let configured = vision.replace("비전", "");
        let poison = mob_skill_name
            .strip_prefix('독')
            .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()));
        if mob_skill_name == configured || poison {
            return 0.5;
        }

        1.0
    }

    // ==================== Death & Reborn ====================

    /// Advance Python `Player.doDeath()` state without owning presentation.
    /// Returns `(processed_step, insured_items)` for the Rhai renderer.
    pub fn advance_death(&mut self) -> (i32, i64) {
        let step = self.step_death;
        let insured = self.get_int("_death_insured_items");
        match step {
            9 => {
                self.act = ActState::Rest;
                self.step_death = 0;
                self.set("체력", (self.get_int("최고체력") as f64 * 0.33) as i64);
            }
            _ => {
                if (0..9).contains(&step) {
                    self.step_death += 1;
                } else {
                    self.step_death = 0;
                }
            }
        }
        (step, insured)
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
        self.step_death = step.clamp(0, 9);
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
    fn death_progression_matches_python_ten_heartbeat_sequence() {
        let mut body = Body::new();
        body.act = ActState::Death;
        body.set("최고체력", 900_i64);
        body.set("보험료", 0_i64);

        let mut steps = Vec::new();
        for _ in 0..10 {
            steps.push(body.advance_death());
        }

        assert_eq!(
            steps.iter().map(|step| step.0).collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
        assert_eq!(steps[8].1, 0);
        assert_eq!(body.act, ActState::Rest);
        assert_eq!(body.get_hp(), 297);
        assert_eq!(body.get_death_step(), 0);
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
    fn get_mp_uses_python_floor_division_for_negative_percent_effects() {
        let mut body = Body::new();
        body.set("내공", 1i64);
        body._mp = -40;

        assert_eq!(body.get_mp(), 0);
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
    fn test_load_skill_state_from_python_array_attributes() {
        let mut body = Body::new();
        body.set("무공이름", "지르기|강룡십팔장");
        body.set("무공숙련도", "지르기 3 17|강룡십팔장 9 42");

        body.load_skill_state_from_attrs();

        assert_eq!(body.skill_list, vec!["지르기", "강룡십팔장"]);
        assert_eq!(
            body.skill_map.get("지르기"),
            Some(&SkillTraining::new(3, 17))
        );
        assert_eq!(
            body.skill_map.get("강룡십팔장"),
            Some(&SkillTraining::new(9, 42))
        );
    }

    #[test]
    fn test_sync_skill_state_to_python_array_attributes() {
        let mut body = Body::new();
        body.skill_list = vec!["지르기".to_string(), "강룡십팔장".to_string()];
        body.skill_map
            .insert("지르기".to_string(), SkillTraining::new(3, 17));
        body.skill_map
            .insert("강룡십팔장".to_string(), SkillTraining::new(9, 42));

        body.sync_skill_state_to_attrs();

        assert_eq!(body.get_string("무공이름"), "지르기|강룡십팔장");
        assert_eq!(body.get_string("무공숙련도"), "지르기 3 17|강룡십팔장 9 42");
    }

    #[test]
    fn secret_skill_names_use_python_array_storage() {
        let mut body = Body::new();
        body.set("비전이름", "강룡십팔장비전|무극검비전");

        assert!(body.has_secret_skill("강룡십팔장비전"));
        assert!(body.has_secret_skill("무극검비전"));
        assert!(!body.has_secret_skill("강룡십팔장"));
        assert_eq!(
            body.get_secret_skills(),
            vec!["강룡십팔장비전", "무극검비전"]
        );

        body.add_secret_skill("천마검비전");
        assert_eq!(
            body.get_string("비전이름"),
            "강룡십팔장비전|무극검비전|천마검비전"
        );
    }

    #[test]
    fn secret_skill_names_still_read_legacy_comma_storage() {
        let mut body = Body::new();
        body.set("비전이름", "강룡십팔장비전, 무극검비전");

        assert!(body.has_secret_skill("강룡십팔장비전"));
        assert!(body.has_secret_skill("무극검비전"));
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

    #[test]
    fn item_weapon_skill_learning_uses_python_counter_roll_and_consumption() {
        let mut body = Body::new();
        body.set("성격", "정파");
        let weapon = Arc::new(Mutex::new(Object::new()));
        {
            let mut weapon_data = weapon.lock().unwrap();
            weapon_data.set("이름", "비급검");
            weapon_data.set("무공이름", "비검술 정파 1 1 25");
            weapon_data.set("아이템속성", "무공배운후소멸");
            weapon_data.set("inUse", 1_i64);
            weapon_data.set("공격력", 7_i64);
        }
        body.attpower = 7;
        body.object.objs.push(weapon.clone());
        body.weapon_item = Some(Arc::downgrade(&weapon));

        assert_eq!(body.check_item_skill_with_roller(&mut || 26), None);
        assert_eq!(body.item_skill_map["비급검"], 1);
        assert_eq!(
            body.check_item_skill_with_roller(&mut || 25),
            Some("비검술".to_string())
        );
        assert!(body.skill_list.iter().any(|skill| skill == "비검술"));
        assert!(body.weapon_item.is_none());
        assert!(body.object.objs.is_empty());
        assert_eq!(body.attpower, 0);
    }

    #[test]
    fn anger_caps_at_six_hundred_and_only_signals_one_hundred() {
        let mut body = Body::new();
        body.set("분노", 99_i64);
        assert!(body.add_anger());
        assert!(!body.add_anger());
        body.set("분노", 600_i64);
        assert!(!body.add_anger());
        assert_eq!(body.get_int("분노"), 600);
    }

    #[test]
    fn vision_training_completes_only_on_python_zero_or_one_roll() {
        let mut body = Body::new();
        body.set("비전수련", "독문무공 9");
        assert!(!body.check_vision_training_with_roll("독문무공", &mut || 2));
        assert_eq!(body.get_string("비전수련"), "독문무공 10");
        assert!(body.check_vision_training_with_roll("독문무공", &mut || 1));
        assert!(body.get_string("비전수련").is_empty());
        assert!(body.get_secret_skills().contains(&"독문무공".to_string()));
    }

    #[test]
    fn vision_setting_halves_configured_and_numeric_poison_skills() {
        let mut body = Body::new();
        body.set("비전설정", "비전독문무공");
        assert_eq!(body.get_vision_damage_modifier("독문무공"), 0.5);
        assert_eq!(body.get_vision_damage_modifier("독12"), 0.5);
        assert_eq!(body.get_vision_damage_modifier("강룡십팔장"), 1.0);
    }

    #[test]
    fn item_skill_training_round_trips_in_python_insertion_order() {
        let mut body = Body::new();
        body.set("무공이름수련리스트", "첫 무기 12|둘째 7");
        body.load_skill_state_from_attrs();
        assert_eq!(body.item_skill_order, vec!["첫 무기", "둘째"]);
        assert_eq!(body.item_skill_map["첫 무기"], 12);
        body.item_skill_map.insert("셋째".to_string(), 3);
        body.item_skill_order.push("셋째".to_string());
        body.sync_skill_state_to_attrs();
        assert_eq!(
            body.get_string("무공이름수련리스트"),
            "첫 무기 12|둘째 7|셋째 3"
        );
    }

    #[test]
    fn level_up_awards_python_trait_points_before_second_job() {
        let mut body = Body::new();
        body.set("레벨", 10_i64);
        body.set("최고체력", 100_i64);
        body.set("맷집", 10_i64);
        body.set("전직", 0_i64);
        body.level_up();
        assert_eq!(body.get_int("특성치"), 1);

        body.set("레벨", 2_001_i64);
        body.set("전직", 2_i64);
        body.level_up();
        assert_eq!(body.get_int("특성치"), 1);
    }
}
