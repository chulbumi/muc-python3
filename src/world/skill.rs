//! Skill (무공) module for MUD combat system
//!
//! This module provides skill loading and management functionality.
//! Skills are loaded from JSON file in data/config/skill.json.

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

/// 무공 타입 (Mugong/Skill Type)
#[derive(Debug, Clone, PartialEq)]
pub enum SkillType {
    /// 전투무공 (Combat skill)
    Combat,
    /// 방어무공 (Defense skill)
    Defense,
    /// 내공 (Internal energy skill)
    Internal,
    /// 기타 (Other)
    Other,
}

impl SkillType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "전투" => SkillType::Combat,
            "방어" => SkillType::Defense,
            "내공" => SkillType::Internal,
            _ => SkillType::Other,
        }
    }
}

/// 공격 패턴 액션 타입 (Attack Pattern Action Type)
#[derive(Debug, Clone, PartialEq)]
pub enum PatternAction {
    /// 초식 (Opening move - visual effect)
    Opening,
    /// 공격 (Attack - deals damage)
    Attack,
    /// 대기 (Wait - no action)
    Wait,
}

impl PatternAction {
    pub fn from_str(s: &str) -> Self {
        match s {
            "초식" => PatternAction::Opening,
            "공격" => PatternAction::Attack,
            "대기" => PatternAction::Wait,
            _ => PatternAction::Wait,
        }
    }
}

/// 공격 패턴 요소 (Attack Pattern Element)
#[derive(Debug, Clone)]
pub struct PatternElement {
    /// 액션 타입 (초식/공격/대기)
    pub action: PatternAction,
    /// 메시지 템플릿 (with placeholders like [공], [방], [무])
    pub message: String,
}

/// 스킬 데이터 (Skill Data)
#[derive(Debug, Clone)]
pub struct Skill {
    /// 스킬 이름
    pub name: String,
    /// 스킬 타입
    pub skill_type: SkillType,
    /// 공격 패턴 (턴 -> 패턴 요소 목록)
    pub pattern: HashMap<i32, Vec<PatternElement>>,
    /// 최대 턴 수
    pub max_turn: usize,
    /// 무공 시전 메시지 (mugong script)
    pub mugong_script: String,
    /// 실패 메시지
    pub fail_message: String,
    /// 방어상태머리말 (Defense head - shown when mob is in combat)
    pub defense_head: String,
    /// 타격률 (Hit rate)
    pub hit_rate: f64,
    /// 확률 (Probability)
    pub probability: i64,
    /// 확률증가 (Probability increase per level)
    pub prob_increase: i64,
    /// 계열 (Category - for anti-type checking)
    pub category: String,
    /// 계열금지 (Anti-type - which category is denied)
    pub deny: String,
    /// 전체공격 여부 (All attack - hits all enemies)
    pub all_attack: bool,
    /// 속성 보너스 (Strength exp bonus)
    pub bonus: i64,
    /// 내공 소모 (MP cost)
    pub mp_cost: i64,
    /// 체력 소모 (HP cost)
    pub hp_cost: i64,
    /// 체력 요구 (HP requirement)
    pub hp_requirement: i64,
    /// 힘 증가 (_str)
    pub str_bonus: i64,
    /// 민첩성 증가 (_dex)
    pub dex_bonus: i64,
    /// 맷집 증가 (_arm)
    pub arm_bonus: i64,
    /// 내공 증가/감소 (_mp, percentage)
    pub mp_bonus: i64,
    /// 최고내공 증가/감소 (_maxmp, percentage)
    pub max_mp_bonus: i64,
    /// 체력 증가/감소 (_hp, percentage)
    pub hp_bonus: i64,
    /// 최고체력 증가/감소 (_maxhp, percentage)
    pub max_hp_bonus: i64,
    /// 방어시간 (Base defense duration)
    pub defense_time: i64,
    /// 방어시간증가치 (Defense time increase per level)
    pub defense_time_increase: i64,
    /// 상대무공 (Counter skill name)
    pub against_skill: Option<String>,
    /// 자기금지 (Cannot use on self)
    pub deny_self: bool,
    /// 타인금지 (Cannot use on others)
    pub deny_others: bool,
    // === State fields (reset on init()) ===
    /// Last executed turn (resets to 0 when cycle completes)
    pub end: i32,
    /// Current turn counter (increments each getScript call)
    pub curturn: i32,
    /// Current step (number of patterns to execute this call)
    pub step: i32,
}

impl Skill {
    /// Parse skill from JSON value
    pub fn from_json(name: String, json: &JsonValue) -> Result<Self, String> {
        // 기본 스킬 생성
        let mut skill = Skill {
            name: name.clone(),
            skill_type: SkillType::Other,
            pattern: HashMap::new(),
            max_turn: 0,
            mugong_script: String::new(),
            fail_message: String::new(),
            defense_head: String::new(),
            hit_rate: 0.0,
            probability: 100,
            prob_increase: 90,
            category: String::new(),
            deny: String::new(),
            all_attack: false,
            bonus: 1,
            mp_cost: 0,
            hp_cost: 0,
            hp_requirement: 0,
            str_bonus: 0,
            dex_bonus: 0,
            arm_bonus: 0,
            mp_bonus: 0,
            max_mp_bonus: 0,
            hp_bonus: 0,
            max_hp_bonus: 0,
            defense_time: 0,
            defense_time_increase: 0,
            against_skill: None,
            deny_self: false,
            deny_others: false,
            // State fields
            end: 0,
            curturn: 0,
            step: 0,
        };

        // 종류 (Type)
        if let Some(typ) = json.get("종류").and_then(|v| v.as_str()) {
            skill.skill_type = SkillType::from_str(typ);
        }

        // 무공스크립 (Mugong script)
        skill.mugong_script = json.get("무공스크립")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 실패 (Fail message)
        skill.fail_message = json.get("실패")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 방어상태머리말 (Defense head)
        skill.defense_head = json.get("방어상태머리말")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 타격률 (Hit rate)
        if let Some(rate) = json.get("타격률") {
            if let Some(f) = rate.as_f64() {
                skill.hit_rate = f;
            } else if let Some(i) = rate.as_i64() {
                skill.hit_rate = i as f64;
            }
        }

        // 확률 (Probability)
        skill.probability = json.get("확률")
            .and_then(|v| v.as_i64())
            .unwrap_or(100);

        // 확률증가 (Probability increase)
        skill.prob_increase = json.get("확률증가")
            .and_then(|v| v.as_i64())
            .unwrap_or(90);

        // 계열 (Category)
        skill.category = json.get("계열")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 공격 패턴 파싱 (Attack pattern)
        if let Some(attack) = json.get("공격") {
            skill.parse_attack_patterns(attack)?;
        }

        // 속성 파싱 (Attributes)
        if let Some(attrs) = json.get("속성") {
            skill.parse_attributes(attrs)?;
        }

        // 방어능력 파싱 (Defense abilities)
        if let Some(def) = json.get("방어능력") {
            skill.parse_defense_abilities(def)?;
        }

        // 방어시간 (Defense time)
        skill.defense_time = json.get("방어시간")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 방어시간증가치 (Defense time increase)
        skill.defense_time_increase = json.get("방어시간증가치")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(skill)
    }

    /// Parse attack patterns from JSON
    fn parse_attack_patterns(&mut self, attack: &JsonValue) -> Result<(), String> {
        let attack_lines: Vec<&str> = if let Some(s) = attack.as_str() {
            vec![s]
        } else if let Some(arr) = attack.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect()
        } else {
            return Ok(());
        };

        for line in attack_lines {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() < 2 {
                continue;
            }

            let turn: i32 = parts[0].parse().unwrap_or(1);
            let action_type = PatternAction::from_str(parts[1]);
            let message = if parts.len() >= 3 {
                parts[2].to_string()
            } else {
                String::new()
            };

            let element = PatternElement {
                action: action_type,
                message,
            };

            self.pattern.entry(turn).or_insert_with(Vec::new).push(element);
        }

        // Calculate max_turn
        self.max_turn = self.pattern.keys().cloned().max().unwrap_or(0) as usize;
        Ok(())
    }

    /// Parse attributes from JSON
    fn parse_attributes(&mut self, attrs: &JsonValue) -> Result<(), String> {
        let attr_list: Vec<&str> = if let Some(s) = attrs.as_str() {
            vec![s]
        } else if let Some(arr) = attrs.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect()
        } else {
            return Ok(());
        };

        for attr in attr_list {
            if attr.contains("힘경험치증가") {
                let parts: Vec<&str> = attr.split_whitespace().collect();
                if parts.len() >= 2 {
                    self.bonus = parts[1].parse().unwrap_or(1);
                }
            } else if attr.contains("내공소모") {
                let parts: Vec<&str> = attr.split_whitespace().collect();
                if parts.len() >= 2 {
                    self.mp_cost = parts[1].parse().unwrap_or(0);
                }
            } else if attr.contains("체력소모") {
                let parts: Vec<&str> = attr.split_whitespace().collect();
                if parts.len() >= 2 {
                    self.hp_cost = parts[1].parse().unwrap_or(0);
                }
            } else if attr.contains("체력요구") {
                let parts: Vec<&str> = attr.split_whitespace().collect();
                if parts.len() >= 2 {
                    self.hp_requirement = parts[1].parse().unwrap_or(0);
                }
            } else if attr.contains("전체무공") {
                self.all_attack = true;
            } else if attr.contains("계열금지") {
                let parts: Vec<&str> = attr.split_whitespace().collect();
                if parts.len() >= 2 {
                    self.deny = parts[1].to_string();
                }
            } else if attr.contains("상대무공") {
                // Extract skill name after "상대무공"
                let skill_name = attr.replacen("상대무공", "", 1).trim().to_string();
                if !skill_name.is_empty() {
                    self.against_skill = Some(skill_name);
                }
            } else if attr.contains("자신금지") {
                self.deny_self = true;
            } else if attr.contains("타인금지") {
                self.deny_others = true;
            }
        }
        Ok(())
    }

    /// Parse defense abilities from JSON
    fn parse_defense_abilities(&mut self, def: &JsonValue) -> Result<(), String> {
        let def_list: Vec<&str> = if let Some(s) = def.as_str() {
            vec![s]
        } else if let Some(arr) = def.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect()
        } else {
            return Ok(());
        };

        for item in def_list {
            let parts: Vec<&str> = item.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let value_str = parts.get(1).unwrap_or(&"0");
            let (base, percent) = if value_str.contains('%') {
                (0, value_str.trim_end_matches('%').parse().unwrap_or(0))
            } else {
                (value_str.parse().unwrap_or(0), 0)
            };

            match parts[0] {
                "힘" => self.str_bonus = base,
                "민첩성" => self.dex_bonus = base,
                "맷집" => self.arm_bonus = base,
                "내공" => self.mp_bonus = percent,
                "최고내공" => self.max_mp_bonus = percent,
                "체력" => self.hp_bonus = percent,
                "최고체력" => self.max_hp_bonus = percent,
                _ => {}
            }
        }
        Ok(())
    }

    /// Initialize/reset skill state (call when starting to use a skill)
    /// Python: def init(self): self.start_time = 0; self.step = 0; self.end = 0; self.curturn = 0
    pub fn init(&mut self) {
        self.end = 0;
        self.curturn = 0;
        self.step = 0;
    }

    /// Get attack script for given dex (agility)
    /// Returns (scripts, more, remaining_dex)
    /// Python: getScript(dex) -> (script, more, dex)
    ///
    /// # Combat Flow
    /// 1. Each combat turn: `dex += getDex() + 700`
    /// 2. Call `get_script(dex)` to execute skill patterns
    /// 3. Skill consumes 700 dex per turn of pattern execution
    /// 4. After skill: if `more == false` or last action was '대기':
    ///    - Calculate normal attacks: `cnt = remaining_dex / 700`
    ///    - Keep remainder: `dex = remaining_dex % 700`
    ///    - Execute `cnt` normal attacks
    ///
    /// # Example
    /// - Skill has 7 turns of patterns
    /// - Character has 3000 dex accumulated
    /// - Skill executes: 3000 / 700 = 4 steps (turns 1-4 of patterns)
    /// - Remaining dex: 3000 - 2800 = 200
    /// - No normal attacks (200 < 700)
    /// - Next turn: dex += 700, now 900 total
    /// - Skill executes: 900 / 700 = 1 step (turn 5)
    /// - Remaining dex: 900 - 700 = 200
    /// - And so on...
    pub fn get_script(&mut self, dex: i64) -> (Vec<PatternElement>, bool, i64) {
        let mut more = true;
        let start = self.end + 1;  // Continue from where we left off
        self.curturn += 1;         // Increment turn counter

        // Calculate step based on dex (700 dex = 1 step)
        self.step = (dex / 700) as i32;
        if self.step as usize > self.max_turn - start as usize + 1 {
            self.step = (self.max_turn - start as usize + 1) as i32;
        }

        let mut remaining_dex = dex;
        if self.step as usize > self.max_turn {
            remaining_dex -= 700 * self.max_turn as i64;
        } else {
            remaining_dex -= 700 * self.step as i64;
        }

        self.end = start + self.step - 1;

        let mut scripts = Vec::new();

        for i in start..=self.end {
            if i > self.max_turn as i32 {
                break;
            }
            if let Some(patterns) = self.pattern.get(&i) {
                for p in patterns {
                    scripts.push(p.clone());
                }
            }
        }

        // Check if cycle is complete
        if self.end >= self.max_turn as i32 {
            self.end = 0;  // Reset for next cycle
            more = false;
        }

        (scripts, more, remaining_dex)
    }

    /// Check if skill is all-attack type
    pub fn is_all_attack(&self) -> bool {
        self.all_attack
    }

    /// Get anti-type (deny category)
    pub fn get_anti_type(&self) -> &str {
        &self.deny
    }
}

/// Calculate number of normal attacks from remaining dex
/// Returns (attack_count, remainder_dex)
/// Python: cnt = int(dex // 700); dex = dex % 700
///
/// This is called after get_script when `more == false` or last action was '대기'
///
/// # Example
/// ```rust
/// let (attacks, remainder) = calculate_normal_attacks(1500);
/// assert_eq!(attacks, 2);  // 2 normal attacks
/// assert_eq!(remainder, 100);  // 100 dex carries over
/// ```
pub fn calculate_normal_attacks(dex: i64) -> (i64, i64) {
    let count = dex / 700;
    let remainder = dex % 700;
    (count, remainder)
}

/// 글로벌 스킬 캐시
#[derive(Debug)]
pub struct SkillCache {
    /// 스킬 데이터 (스킬 이름 -> 스킬)
    skills: HashMap<String, Skill>,
}

impl SkillCache {
    /// Create a new skill cache
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Load skills from data/config/skill.json
    pub fn load_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let path = "data/config/skill.json";
        let content = std::fs::read_to_string(path)?;

        let json: JsonValue = serde_json::from_str(&content)?;

        if let Some(obj) = json.as_object() {
            for (name, skill_data) in obj {
                match Skill::from_json(name.clone(), skill_data) {
                    Ok(skill) => {
                        self.skills.insert(name.clone(), skill);
                    }
                    Err(e) => {
                        eprintln!("Failed to parse skill {}: {}", name, e);
                    }
                }
            }
        }

        tracing::info!("Loaded {} skills from {}", self.skills.len(), path);
        Ok(())
    }

    /// Get skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Get skill mutable by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Skill> {
        self.skills.get_mut(name)
    }

    /// Get all skill names
    pub fn skill_names(&self) -> Vec<&str> {
        self.skills.keys().map(|k| k.as_str()).collect()
    }

    /// Get number of skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

impl Default for SkillCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 글로벌 스킬 캐시 접근자
pub fn get_skill_cache() -> &'static RwLock<SkillCache> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<RwLock<SkillCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut cache = SkillCache::new();
        if let Err(e) = cache.load_all() {
            eprintln!("Failed to load skills: {}", e);
        }
        RwLock::new(cache)
    })
}

/// 스킬 이름으로 스킬 데이터 가져오기
pub fn get_skill(name: &str) -> Option<Skill> {
    get_skill_cache()
        .read()
        .ok()?
        .get(name)
        .cloned()
}

/// 스킬 이름으로 방어상태머리말 가져오기
/// This is already in data/mod.rs but keeping for convenience
pub fn get_skill_defense_head(name: &str) -> String {
    if let Ok(cache) = get_skill_cache().read() {
        cache.get(name)
            .map(|s| s.defense_head.clone())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_cache_new() {
        let cache = SkillCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_pattern_action_from_str() {
        assert_eq!(PatternAction::from_str("초식"), PatternAction::Opening);
        assert_eq!(PatternAction::from_str("공격"), PatternAction::Attack);
        assert_eq!(PatternAction::from_str("대기"), PatternAction::Wait);
        assert_eq!(PatternAction::from_str("unknown"), PatternAction::Wait);
    }

    #[test]
    fn test_skill_type_from_str() {
        assert_eq!(SkillType::from_str("전투"), SkillType::Combat);
        assert_eq!(SkillType::from_str("방어"), SkillType::Defense);
        assert_eq!(SkillType::from_str("내공"), SkillType::Internal);
        assert_eq!(SkillType::from_str("unknown"), SkillType::Other);
    }
}
