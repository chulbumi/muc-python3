//! PvM (Player vs Mob) Combat Processor
//!
//! Handles combat rounds between players and mobs including:
//! - Attack damage calculation
//! - Mob AI and counter-attacks
//! - Combat state management
//! - Death and rewards

use crate::player::{ActState, Body};
use crate::world::skill::Skill;
use crate::world::{MobInstance, RawMobData, WorldState};
use rand::Rng;
use serde_json::Value as JsonValue;

fn mob_equipment_stats(mob: &RawMobData) -> (i64, i64) {
    mob.use_items
        .iter()
        .fold((0, 0), |(attack, armor), (key, _, _, _)| {
            let path = std::path::Path::new("data/item").join(format!("{key}.json"));
            let Some(info) = std::fs::read_to_string(path)
                .ok()
                .and_then(|source| serde_json::from_str::<JsonValue>(&source).ok())
                .and_then(|root| root.get("아이템정보").cloned())
            else {
                return (attack, armor);
            };
            let item_attack = info
                .get("공격력")
                .or_else(|| info.get("타격"))
                .and_then(JsonValue::as_i64)
                .unwrap_or(0);
            let item_armor = info.get("방어력").and_then(JsonValue::as_i64).unwrap_or(0);
            (attack + item_attack, armor + item_armor)
        })
}

/// Combat action result
#[derive(Debug, Clone)]
pub enum CombatAction {
    /// Hit target with damage
    Hit { target: String, damage: i64 },
    /// Missed target
    Miss { target: String },
    /// Target killed
    Kill { target: String, exp: i64, gold: i64 },
    /// Player killed
    PlayerDeath,
    /// Error message
    Error(String),
}

/// Combat round result
#[derive(Debug, Clone)]
pub struct CombatRound {
    /// State-only presentation events rendered by the heartbeat Rhai command.
    pub presentation_events: Vec<JsonValue>,
    /// Total damage dealt
    pub damage_dealt: i64,
    /// Total damage taken
    pub damage_taken: i64,
    /// Whether combat ended
    pub combat_ended: bool,
    /// Whether player died
    pub player_died: bool,
    /// Whether target died
    pub target_died: bool,
}

impl Default for CombatRound {
    fn default() -> Self {
        Self::new()
    }
}

impl CombatRound {
    pub fn new() -> Self {
        Self {
            presentation_events: Vec::new(),
            damage_dealt: 0,
            damage_taken: 0,
            combat_ended: false,
            player_died: false,
            target_died: false,
        }
    }
}

/// Calculate player damage to mob
///
/// Python formula (from objs/body.py:getAttackPoint):
/// c1 = self.getStr() * 2           # 힘 × 2
/// c1 += self.getMaxMp() // 5       # 플레이어: 최대내공 / 5
/// c2 = self.getAttPower() - ss     # 무기공격력 - 숙련도차이
/// m1 = (c1 + c2) - (mob.getArm() + mob.getArmor())
/// Then apply ±20% random variation (0.8~1.2x)
pub fn calculate_player_damage(player: &Body, mob_data: &RawMobData) -> i64 {
    calculate_player_damage_with_arm(player, mob_data, mob_data.arm)
}

fn calculate_player_damage_with_arm(player: &Body, mob_data: &RawMobData, mob_arm: i64) -> i64 {
    let player_str = player.get_str();
    let max_mp = player.get_max_mp();
    let weapon_power = player.attpower as i64;

    // Python 공식: c1 = 힘 × 2 + 최대내공 / 5
    let c1 = (player_str * 2) + (max_mp / 5);

    // 숙련도 시스템: ss = s1(무기기량) - s2(숙련도)
    // Python: s1 = getInt(item['기량']), s2 = getInt(self['%d 숙련도' % weaponType])
    // c2 = getAttPower() - ss
    let ss = player.get_mastery_diff();
    let c2 = weapon_power - ss;

    // 방어력 계산: 몹 맷집 (arm = 맷집)
    // Python: mob.getArm() returns 맷집, mob.getArmor() returns equipment defense
    // 현재는 맷집만 사용 (장비 방어력은 추후 구현)
    let (_, equipment_armor) = mob_equipment_stats(mob_data);
    let mob_defense = mob_arm + equipment_armor;

    // 기본 데미지: (c1 + c2) - 방어력
    let mut damage = (c1 + c2) - mob_defense;

    // 최소 데미지 1 보장
    if damage < 1 {
        damage = 1;
    }

    // Python chooses every integer in [int(m*0.80), int(m*1.20)] with equal
    // probability.  Multiplying by a random percentage is not equivalent for
    // larger values because it skips most integers in that interval.
    let min_damage = damage * 80 / 100;
    let max_damage = damage * 120 / 100;
    damage = rand::thread_rng().gen_range(min_damage..=max_damage);

    // 최소 데미지 1 보장 (랜덤 후)
    if damage < 1 {
        damage = 1;
    }

    // Python 코드에는 데미지 캡이 없지만,
    // 너무 큰 데미지를 방지하기 위해 밸런스 조정 (필요시 제거)
    // let max_damage = (mob_data.max_hp * 50) / 100; // 최대 50%
    // if damage > max_damage {
    //     damage = max_damage;
    // }

    damage
}

/// Calculate mob damage to player
///
/// Uses same formula as player damage (from objs/body.py:getAttackPoint):
/// c1 = mob_str × 2
/// c2 = mob_weapon_power
/// damage = (c1 + c2) - player_defense
/// Then apply ±20% random variation
pub fn calculate_mob_damage(mob_data: &RawMobData, player: &Body) -> i64 {
    calculate_mob_damage_with_strength(mob_data, mob_data.strength, player)
}

fn calculate_mob_damage_with_strength(
    mob_data: &RawMobData,
    mob_strength: i64,
    player: &Body,
) -> i64 {
    let mob_str = mob_strength;
    // Python Mob.init() sums 공격력 from every configured 사용아이템.
    // An unarmed mob has attack power zero; strength is not used twice.
    let (mob_weapon, _) = mob_equipment_stats(mob_data);

    // Python 공식: c1 = 힘 × 2
    let c1 = mob_str * 2;

    // c2 = 무기공격력
    let c2 = mob_weapon;

    // 플레이어 방어력: armor + arm
    let player_defense = (player.armor as i64) + (player.get_arm());

    // 기본 데미지
    let mut damage = (c1 + c2) - player_defense;

    // 최소 데미지 1 보장
    if damage < 1 {
        damage = 1;
    }

    let min_damage = damage * 80 / 100;
    let max_damage = damage * 120 / 100;
    damage = rand::thread_rng().gen_range(min_damage..=max_damage);

    // 최소 데미지 1 보장 (랜덤 후)
    if damage < 1 {
        damage = 1;
    }

    damage
}

/// Python `Mob.getSkillPoint`: runtime strength/difficulty, ordinary attack
/// variance, skill hit-rate amplification, then the mob's luck/critical roll.
pub(crate) fn calculate_mob_skill_damage(
    mob: &MobInstance,
    mob_data: &RawMobData,
    player: &Body,
    skill: &crate::world::skill::Skill,
    critical_roll: i64,
) -> i64 {
    let strength = (mob.strength + mob.str_modifier).max(0);
    let base = calculate_mob_damage_with_strength(mob_data, strength, player);
    let rate = if skill.hit_rate <= 0.0 {
        0.1
    } else {
        skill.hit_rate
    };
    let amplified = (base as f64 + base as f64 * rate) as i64;
    let chance = mob_data.luck as f64 * crate::script::get_murim_config_float("운확률");
    let multiplier = if chance > critical_roll as f64 {
        (mob_data.critical as f64 * crate::script::get_murim_config_float("필살배수")).max(1.0)
    } else {
        1.0
    };
    (amplified as f64 * multiplier) as i64
}

/// Check if player hits the mob
///
/// Python formula (from objs/body.py:getAttackChance):
/// CHANCE = 100
/// bonus = self.getHit() * MAIN_CONFIG['명중확률']  # 0.2
/// bonus -= mob.getMiss() * MAIN_CONFIG['회피확률']  # 0.2
/// return CHANCE - (((mob['레벨']-self['레벨'])+90)//3) + bonus
pub fn calculate_attack_chance(player: &Body, mob_data: &RawMobData) -> f64 {
    calculate_attack_chance_with_level(player, mob_data, mob_data.level)
}

fn calculate_attack_chance_with_level(player: &Body, mob_data: &RawMobData, mob_level: i64) -> f64 {
    const HIT_RATE: f64 = 0.2; // MAIN_CONFIG['명중확률']
    const DODGE_RATE: f64 = 0.2; // MAIN_CONFIG['회피확률']

    let player_level = player.get_int("레벨");
    // data/config/murim.json `최대사냥레벨차이`; Python returns -1 before
    // applying any hit/evasion bonuses at this boundary.
    if mob_level - player_level >= 400 {
        return -1.0;
    }
    let player_hit = player.get_hit();
    let mob_miss = mob_data.miss;

    // Python 공식
    let base_chance = 100.0;
    // Python 2 integer `//` floors negative values.  `div_euclid` preserves
    // that behavior, unlike Rust's truncating `/`.
    let level_modifier = ((mob_level - player_level) + 90).div_euclid(3) as f64;
    let bonus = (player_hit as f64 * HIT_RATE) - (mob_miss as f64 * DODGE_RATE);

    base_chance - level_modifier + bonus
}

pub fn check_hit(player: &Body, mob_data: &RawMobData) -> bool {
    let hit_chance = calculate_attack_chance(player, mob_data);

    // Python uses randint(0, 100), whose upper bound is inclusive.  This is
    // observably different at both 0 and 100 and must not be modeled as
    // Rust's half-open 0..100 range.
    hit_chance >= rand::thread_rng().gen_range(0..=100) as f64
}

fn check_hit_instance(player: &Body, mob: &MobInstance, mob_data: &RawMobData) -> bool {
    let hit_chance = calculate_attack_chance_with_level(player, mob_data, mob.level);
    hit_chance >= rand::thread_rng().gen_range(0..=100) as f64
}

pub(crate) fn apply_player_attack_training(player: &mut Body, hit: bool, round: &mut CombatRound) {
    if let Some(skill) =
        player.check_item_skill_with_roller(&mut || rand::thread_rng().gen_range(0..=99))
    {
        round.presentation_events.push(serde_json::json!({
            "kind": "item_skill_learned", "skill": skill,
        }));
    }
    let stat_up = if hit {
        player.add_str(1, true).then_some("strength_up")
    } else {
        let max_dex = crate::script::get_murim_config_int("민첩성최고수치");
        player.add_dex(1, max_dex).then_some("dexterity_up")
    };
    if let Some(kind) = stat_up {
        round
            .presentation_events
            .push(serde_json::json!({ "kind": kind }));
    }
    if player.weapon_skill_up(1) {
        round
            .presentation_events
            .push(serde_json::json!({ "kind": "mastery_up" }));
    }
}

/// Resolve one player strike without advancing the mob's independent clock.
pub fn process_player_strike(
    player: &mut Body,
    mob_instance: &MobInstance,
    mob_data: &RawMobData,
) -> CombatRound {
    let mut round = CombatRound::new();

    let mob = mob_instance.clone();
    if !mob.alive {
        round.presentation_events.push(serde_json::json!({
            "kind": "combat_error",
            "code": "already_dead",
        }));
        round.combat_ended = true;
        return round;
    }

    // Set player to combat state
    player.act = ActState::Fight;

    // Check hit
    if !check_hit_instance(player, &mob, mob_data) {
        round.presentation_events.push(serde_json::json!({
            "kind": "player_miss",
            "mob": mob.name,
            "player": player.get_name(),
            "weapon_type": player.get_fight_script_type(),
        }));
        apply_player_attack_training(player, false, &mut round);
        return round;
    }

    // Calculate damage
    let effective_arm = (mob.arm + mob.arm_modifier).max(0);
    let damage = calculate_player_damage_with_arm(player, mob_data, effective_arm);
    round.damage_dealt = damage;

    round.presentation_events.push(serde_json::json!({
        "kind": "player_attack",
        "mob": mob.name,
        "player": player.get_name(),
        "weapon": player.get_weapon_name(),
        "weapon_type": player.get_fight_script_type(),
        "damage": damage,
    }));
    apply_player_attack_training(player, true, &mut round);

    // Check if mob died
    let new_hp = mob.hp - damage;
    if new_hp <= 0 {
        round.target_died = true;
        round.combat_ended = true;

        // Reset player state
        player.act = ActState::Stand;
    }

    round
}

/// Resolve one ordinary mob strike without advancing the player's clock.
pub fn process_mob_strike(
    player: &mut Body,
    mob: &MobInstance,
    mob_data: &RawMobData,
) -> CombatRound {
    let mut round = CombatRound::new();
    let chance = 100.0 - ((player.get_int("레벨") - mob.level + 90).div_euclid(3)) as f64
        + mob_data.hit as f64 * 0.2
        - player.get_miss() as f64 * 0.2;
    if chance < rand::thread_rng().gen_range(0..=100) as f64 {
        round.presentation_events.push(serde_json::json!({
            "kind": "mob_normal_miss", "mob": mob.name,
            "player": player.get_name(), "weapon_type": mob_data.combat_script,
        }));
        return round;
    }
    let effective_strength = (mob.strength + mob.str_modifier).max(0);
    let damage = calculate_mob_damage_with_strength(mob_data, effective_strength, player);
    round.damage_taken = damage;
    let lethal = player.minus_hp(damage);
    round.presentation_events.push(serde_json::json!({
        "kind": "mob_normal_attack", "mob": mob.name,
        "player": player.get_name(), "damage": damage,
        "weapon_type": mob_data.combat_script,
    }));
    if player.add_anger() {
        round
            .presentation_events
            .push(serde_json::json!({ "kind": "anger_100" }));
    }
    if lethal {
        round.player_died = true;
        round.combat_ended = true;
        player.act = ActState::Death;
        player.unwear_all();
        player.clear_targets_death();
        player.clear_skills();
        player.set_death_step(0);
        round
            .presentation_events
            .push(serde_json::json!({ "kind": "player_death" }));
    }
    round
}

/// Compatibility one-exchange API. The heartbeat uses the two independent
/// strike functions because Python maintains separate dexterity clocks.
pub fn process_player_attack(
    player: &mut Body,
    mob: &MobInstance,
    mob_data: &RawMobData,
) -> CombatRound {
    let mut round = process_player_strike(player, mob, mob_data);
    if !round.target_died && !round.combat_ended {
        let counter = process_mob_strike(player, mob, mob_data);
        round.damage_taken = counter.damage_taken;
        round.player_died = counter.player_died;
        round.combat_ended = counter.combat_ended;
        round
            .presentation_events
            .extend(counter.presentation_events);
    }
    round
}

/// Calculate EXP reward from mob
///
/// Based on Python objs/mob.py die() and objs/body.py addExp()
/// Base exp scales with mob level and adjusts for player level difference
fn base_exp_reward(mob_level: i64, player_level: i64) -> i64 {
    // Python Mob.getExpGold.  Use i128 intermediates because Python integers
    // do not overflow before the final MAX_INT clamp.
    let level = mob_level as i128;
    let target = player_level as i128;
    let a = level * level / 3 + 30;
    let b = (a * (level - target)).div_euclid(100);
    (a + b).clamp(1, i32::MAX as i128) as i64
}

pub(crate) fn calculate_exp_reward_for_level(mob_level: i64, player_level: i64) -> i64 {
    let mut exp = base_exp_reward(mob_level, player_level);
    let variance = rand::thread_rng().gen_range(0..=9);
    if rand::thread_rng().gen_range(0..=1) == 0 {
        exp = exp.saturating_add(variance);
    } else {
        exp = exp.saturating_sub(variance);
    }
    exp.clamp(1, i32::MAX as i64)
}

/// Calculate gold reward from mob
pub(crate) fn calculate_gold_reward_for_level(mob_level: i64, carried_gold: i64) -> i64 {
    let variance = rand::thread_rng().gen_range(0..=4);
    let mut gold = (mob_level as i128) + 14;
    if rand::thread_rng().gen_range(0..=1) == 0 {
        gold += variance as i128;
    } else {
        gold -= variance as i128;
    }
    gold += carried_gold as i128;
    gold.clamp(1, i32::MAX as i128) as i64
}

/// Start combat with a mob
pub fn start_combat(
    player: &mut Body,
    mob_instance: &MobInstance,
    mob_data: &RawMobData,
) -> CombatRound {
    process_player_attack(player, mob_instance, mob_data)
}

/// Find mob in room by name
/// Returns (mob_instance, mob_data)
pub fn find_mob_in_room(
    player_name: &str,
    target_name: &str,
    world: &WorldState,
) -> Option<(MobInstance, RawMobData)> {
    let player_pos = world.get_player_position(player_name)?;

    // First try exact match using WorldState's mob_cache
    {
        let mob_instances = world
            .mob_cache
            .get_mobs_in_room(&player_pos.zone, &player_pos.room);
        for mob in mob_instances {
            if mob.name == target_name {
                if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                    return Some((mob.clone(), mob_data.clone()));
                }
            }
        }
    }

    // Then try partial match
    let mob_instances = world
        .mob_cache
        .get_mobs_in_room(&player_pos.zone, &player_pos.room);
    for mob in mob_instances {
        if mob.name.contains(target_name) {
            if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                return Some((mob.clone(), mob_data.clone()));
            }
        }
    }

    None
}

/// Skill damage result
#[derive(Debug, Clone)]
pub struct SkillDamageResult {
    /// Base damage before modifiers
    pub base_damage: i64,
    /// Final damage after all modifiers
    pub final_damage: i64,
    /// Whether the skill hit
    pub hit: bool,
}

fn calculate_skill_result_with_rolls(
    player: &Body,
    skill: &Skill,
    skill_level: i32,
    mob_data: &RawMobData,
    mob: &MobInstance,
    attack_roll: i64,
    damage_roll: i64,
    critical_roll: i64,
) -> (bool, i64) {
    if mob.level - player.get_int("레벨") >= 400 {
        return (false, 0);
    }
    let mastery_chance = match skill_level {
        11 => 10.0,
        12 => 20.0,
        _ => 0.0,
    };
    let chance = skill.probability as f64 + skill_level as f64 * 4.0
        - ((mob.level - player.get_int("레벨") + 90).div_euclid(3)) as f64
        + player.get_hit() as f64 * 0.2
        - mob_data.miss as f64 * 0.2
        + mastery_chance;
    if chance < attack_roll as f64 {
        return (false, 0);
    }

    let c1 = player.get_str() * 2 + player.get_max_mp().div_euclid(5);
    let c2 = player.attpower as i64 - player.get_mastery_diff();
    let (_, equipment_armor) = mob_equipment_stats(mob_data);
    let base = (c1 + c2 - (mob.arm + mob.arm_modifier).max(0) - equipment_armor).max(1);
    let min_damage = base * 80 / 100;
    let max_damage = base * 120 / 100;
    let normal_damage = damage_roll.clamp(min_damage, max_damage).max(1);
    let hit_rate = if skill.hit_rate <= 0.0 {
        0.1
    } else {
        skill.hit_rate
    };
    let mut damage = (normal_damage as f64 + normal_damage as f64 * hit_rate) as i64;
    let (critical_chance_bonus, mastery_damage) = match skill_level {
        11 => (10.0, 1.3),
        12 => (20.0, 1.5),
        _ => (0.0, 1.0),
    };
    let critical_chance = player.get_critical_chance() as f64 * 0.2 + critical_chance_bonus;
    let critical_multiplier = if critical_chance > critical_roll as f64 {
        (player.get_critical() as f64 * 0.015).max(1.0)
    } else {
        1.0
    };
    damage = (damage as f64 * critical_multiplier * mastery_damage) as i64;
    (true, damage.max(1))
}

pub fn calculate_skill_damage_against(
    player: &Body,
    skill: &Skill,
    skill_level: i32,
    mob_data: &RawMobData,
    mob: &MobInstance,
    _target_name: &str,
) -> SkillDamageResult {
    let c1 = player.get_str() * 2 + player.get_max_mp().div_euclid(5);
    let c2 = player.attpower as i64 - player.get_mastery_diff();
    let (_, equipment_armor) = mob_equipment_stats(mob_data);
    let base = (c1 + c2 - (mob.arm + mob.arm_modifier).max(0) - equipment_armor).max(1);
    let min_damage = base * 80 / 100;
    let max_damage = base * 120 / 100;
    let damage_roll = rand::thread_rng().gen_range(min_damage..=max_damage);
    let (hit, final_damage) = calculate_skill_result_with_rolls(
        player,
        skill,
        skill_level,
        mob_data,
        mob,
        rand::thread_rng().gen_range(0..=100),
        damage_roll,
        rand::thread_rng().gen_range(0..=100),
    );
    SkillDamageResult {
        base_damage: base,
        final_damage,
        hit,
    }
}

/// Calculate skill-based damage
///
/// Skill damage formula (based on Python objs/body.py:useMugong):
/// - Base damage from player's attack power
/// - Multiplied by skill's 타격률 (hit rate bonus)
/// - Modified by skill level (숙련도 보너스: 11=1.3x, 12=1.5x, etc.)
/// - Random variation ±20%
pub fn calculate_skill_damage(
    player: &Body,
    _skill_name: &str,
    skill_level: i32,
    skill_bonus: i64,
    _target_name: &str,
) -> SkillDamageResult {
    // Get player's base attack stats
    let player_str = player.get_str();
    let max_mp = player.get_max_mp();
    let weapon_power = player.attpower as i64;

    // Base attack power (same as normal attack)
    let base_attack = (player_str * 2) + (max_mp / 5) + weapon_power;

    // Skill damage multiplier based on level (each level adds 10%)
    let level_multiplier = 1.0 + (skill_level as f64 * 0.1);

    // Skill bonus multiplier (타격률)
    let skill_multiplier = if skill_bonus > 0 {
        skill_bonus as f64 / 100.0 + 1.0
    } else {
        1.0
    };

    // Python 숙련도 보너스 (11=초급 1.3x, 12=중급 1.5x, 13=상급 1.7x, etc.)
    let mastery_bonus = match skill_level {
        11 => 1.3,       // 초급
        12 => 1.5,       // 중급
        13 => 1.7,       // 상급
        14 => 2.0,       // 고급
        15 => 2.5,       // 특급
        16..=100 => 3.0, // 절정 이상
        _ => 1.0,
    };

    // Calculate base damage
    let base_damage =
        (base_attack as f64 * level_multiplier * skill_multiplier * mastery_bonus) as i64;

    // Random variation ±20%
    let variation = rand::thread_rng().gen_range(-20..=20);
    let final_damage = ((base_damage as f64 * (100.0 + variation as f64)) / 100.0) as i64;
    let final_damage = final_damage.max(1);

    // Hit check (skills have higher hit rate)
    let hit_chance = 80 + (skill_level * 2);
    let hit = rand::thread_rng().gen_range(0..100) < hit_chance.min(95);

    SkillDamageResult {
        base_damage,
        final_damage,
        hit,
    }
}

/// Skill effect types
#[derive(Debug, Clone, PartialEq)]
pub enum SkillEffectType {
    /// Heal HP
    HealHp,
    /// Heal MP
    HealMp,
    /// Boost strength temporarily
    BoostStr,
    /// Boost dexterity temporarily
    BoostDex,
    /// Boost defense temporarily
    BoostArm,
    /// No special effect (damage only)
    None,
}

/// Skill effect result
#[derive(Debug, Clone)]
pub struct SkillEffectResult {
    /// Type of effect applied
    pub effect_type: SkillEffectType,
    /// Amount of effect (heal amount, boost amount, etc.)
    pub amount: i64,
    /// Duration in seconds (for temporary effects)
    pub duration: i64,
    /// Message describing the effect
    pub message: String,
}

/// Apply skill effects to player
///
/// Based on skill attributes (hp_bonus, mp_bonus, etc.)
pub fn apply_skill_effects(
    player: &mut Body,
    skill_name: &str,
    hp_bonus: i64,
    mp_bonus: i64,
    str_bonus: i64,
    dex_bonus: i64,
    arm_bonus: i64,
) -> Vec<SkillEffectResult> {
    let mut effects = Vec::new();

    // HP bonus/restore (percentage of max HP)
    if hp_bonus != 0 {
        let max_hp = player.get_max_hp();
        let hp_change = if hp_bonus > 0 {
            (max_hp * hp_bonus) / 100
        } else {
            hp_bonus // Direct reduction for negative
        };
        let current_hp = player.get_hp();
        let new_hp = (current_hp + hp_change).clamp(0, max_hp);
        player.set("체력", new_hp);

        effects.push(SkillEffectResult {
            effect_type: if hp_bonus > 0 {
                SkillEffectType::HealHp
            } else {
                SkillEffectType::None
            },
            amount: hp_change,
            duration: 0,
            message: if hp_bonus > 0 {
                format!(
                    "{} 무공으로 체력이 {} 회복되었습니다.",
                    skill_name, hp_change
                )
            } else {
                format!(
                    "{} 무공의 부작용으로 체력이 {} 감소했습니다.",
                    skill_name,
                    hp_change.abs()
                )
            },
        });
    }

    // MP bonus/restore (percentage of max MP)
    if mp_bonus != 0 {
        let max_mp = player.get_max_mp();
        let mp_change = if mp_bonus > 0 {
            (max_mp * mp_bonus) / 100
        } else {
            mp_bonus
        };
        let current_mp = player.get_mp();
        let new_mp = (current_mp + mp_change).clamp(0, max_mp);
        player.set("내공", new_mp);

        effects.push(SkillEffectResult {
            effect_type: if mp_bonus > 0 {
                SkillEffectType::HealMp
            } else {
                SkillEffectType::None
            },
            amount: mp_change,
            duration: 0,
            message: if mp_bonus > 0 {
                format!(
                    "{} 무공으로 내공이 {} 회복되었습니다.",
                    skill_name, mp_change
                )
            } else {
                format!(
                    "{} 무공의 부작용으로 내공이 {} 감소했습니다.",
                    skill_name,
                    mp_change.abs()
                )
            },
        });
    }

    // Stat boosts (temporary - stored in modifier fields)
    if str_bonus != 0 {
        player._str += str_bonus as i32;
        effects.push(SkillEffectResult {
            effect_type: SkillEffectType::BoostStr,
            amount: str_bonus,
            duration: 0, // Permanent for now
            message: format!("{} 무공으로 힘이 {} 증가했습니다.", skill_name, str_bonus),
        });
    }

    if dex_bonus != 0 {
        player._dex += dex_bonus as i32;
        effects.push(SkillEffectResult {
            effect_type: SkillEffectType::BoostDex,
            amount: dex_bonus,
            duration: 0,
            message: format!("{} 무공으로 민첩이 {} 증가했습니다.", skill_name, dex_bonus),
        });
    }

    if arm_bonus != 0 {
        player._arm += arm_bonus as i32;
        effects.push(SkillEffectResult {
            effect_type: SkillEffectType::BoostArm,
            amount: arm_bonus,
            duration: 0,
            message: format!("{} 무공으로 맷집이 {} 증가했습니다.", skill_name, arm_bonus),
        });
    }

    effects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_player_attack_attempt_trains_python_stat_and_weapon_mastery() {
        let mut player = Body::new();
        player.set("힘", 11_i64);
        player.set("힘경험치", 19_i64);
        player.set("민첩성", 0_i64);
        player.set("민첩성경험치", 31_i64);
        player.set("1 숙련도", 0_i64);
        player.set("1 숙련도경험치", 33_i64);

        let mut hit_round = CombatRound::new();
        apply_player_attack_training(&mut player, true, &mut hit_round);
        assert_eq!(player.get_int("힘"), 12);
        assert_eq!(player.get_int("힘경험치"), 0);
        assert_eq!(player.get_int("1 숙련도"), 0);
        assert!(hit_round
            .presentation_events
            .iter()
            .any(|event| event["kind"] == "strength_up"));

        let mut miss_round = CombatRound::new();
        apply_player_attack_training(&mut player, false, &mut miss_round);
        assert_eq!(player.get_int("민첩성"), 1);
        assert_eq!(player.get_int("민첩성경험치"), 0);
        assert_eq!(player.get_int("1 숙련도"), 1);
        assert_eq!(player.get_int("1 숙련도경험치"), 0);
        assert!(miss_round
            .presentation_events
            .iter()
            .any(|event| event["kind"] == "dexterity_up"));
        assert!(miss_round
            .presentation_events
            .iter()
            .any(|event| event["kind"] == "mastery_up"));
    }

    #[test]
    fn mob_skill_damage_uses_runtime_strength_and_python_critical_fields() {
        let mut data = RawMobData::new();
        data.strength = 1;
        data.luck = 1_000;
        data.critical = 200;
        let mut mob = MobInstance::new("시험:무공몹".to_string(), "시험".to_string(), "1", &data);
        mob.strength = 100;
        let player = Body::new();
        let skill = crate::world::skill::Skill::from_json(
            "시험무공".to_string(),
            &serde_json::json!({ "타격률": 0 }),
        )
        .unwrap();

        let damage = calculate_mob_skill_damage(&mob, &data, &player, &skill, 100);
        // Runtime strength 100 gives at least 160 before 10% amplification,
        // and luck 1000 guarantees the 200 * 0.015 == 3x critical.
        assert!(damage >= 528, "runtime/critical damage was {damage}");
    }

    #[test]
    fn attack_chance_matches_python_fraction_and_uses_miss_not_agility() {
        let mut player = Body::new();
        player.set("레벨", 10_i64);
        player.set("명중", 3_i64);

        let mut mob = RawMobData::new();
        mob.level = 10;
        mob.agility = 9_999; // getMiss() is the separate 회피 attribute.
        mob.miss = 1;

        // 100 - ((10 - 10 + 90) // 3) + 3*0.2 - 1*0.2
        assert_eq!(calculate_attack_chance(&player, &mob), 70.4);
    }

    #[test]
    fn attack_chance_keeps_python_flooring_for_negative_level_term() {
        let mut player = Body::new();
        player.set("레벨", 101_i64);
        let mut mob = RawMobData::new();
        mob.level = 10;
        mob.miss = 0;

        // Python: ((10 - 101 + 90) // 3) == -1.
        assert_eq!(calculate_attack_chance(&player, &mob), 101.0);
    }

    #[test]
    fn attack_chance_rejects_python_max_hunting_level_boundary() {
        let mut player = Body::new();
        player.set("레벨", 100_i64);
        player.set("명중", 99_999_i64);
        let mut mob = RawMobData::new();
        mob.level = 500;

        assert_eq!(calculate_attack_chance(&player, &mob), -1.0);
    }

    #[test]
    fn test_calculate_player_damage() {
        let mut player = Body::new();
        player.set("레벨", 10i64);
        player.set("힘", 50i64);
        player.set("민첩", 30i64);
        player.set("최고내공", 100i64);
        player.set("1 숙련도", 10i64); // 무기 숙련도 추가
        player.attpower = 20;

        let mut mob_data = RawMobData::new();
        mob_data.name = "테스트몹".to_string();
        mob_data.level = 8;
        mob_data.arm = 50; // 맷집 (방어력)
        mob_data.strength = 40; // 힘 (공격력)
        mob_data.inner_power = 20; // 내공 (defense contribution)
        mob_data.agility = 20;
        mob_data.max_hp = 100;
        mob_data.hp = 100;

        // Python 공식 with 숙련도:
        // c1 = 50*2 + 100/5 = 100 + 20 = 120
        // ss = s1(무기기량=0) - s2(숙련도=10) = -10 → 0 (음수면 0)
        // c2 = 20 - 0 = 20
        // damage = (120 + 20) - 50 = 90
        // ±20%: 72 ~ 108
        let damage = calculate_player_damage(&player, &mob_data);
        assert!(damage > 0);
        // 최소 72, 최대 108 범위 내에 있어야 함 (랜덤이므로 넓게 체크)
        assert!((50..=130).contains(&damage));
    }

    #[test]
    fn test_calculate_mob_damage() {
        let mut player = Body::new();
        player.set("레벨", 10i64);
        player.set("맷집", 40i64);
        player.set("민첩", 30i64);
        player.armor = 10; // 방어구
        player.set("최대체력", 100i64);

        let mut mob_data = RawMobData::new();
        mob_data.level = 8;
        mob_data.strength = 40; // 힘
        mob_data.inner_power = 20; // 내공 (attack power contribution)

        // Python의 무장하지 않은 몹 attpower는 0이다:
        // (40*2 + 0) - (10 + 40) = 30, 범위 24..=36.
        let damage = calculate_mob_damage(&mob_data, &player);
        assert!((24..=36).contains(&damage));
    }

    #[test]
    fn mob_use_items_supply_python_attack_and_armor_totals() {
        let mut mob = RawMobData::new();
        mob.use_items.push(("160-5".to_string(), 1, 1, 1));
        // Python Mob.init ignores count/probability here and equips every
        // configured 사용아이템 once.  160-5 has 공격력 1000, 방어력 0.
        assert_eq!(mob_equipment_stats(&mob), (1000, 0));
    }

    #[test]
    fn skill_damage_uses_python_target_defense_hit_rate_and_mastery_bonus() {
        let mut player = Body::new();
        player.set("레벨", 10_i64);
        player.set("힘", 50_i64);
        player.set("최고내공", 100_i64);
        player.set("명중", 0_i64);
        player.set("운", 0_i64);
        player.set("필살", 0_i64);
        player.attpower = 20;
        let mut data = RawMobData::new();
        data.name = "표적".to_string();
        data.level = 10;
        data.arm = 50;
        data.miss = 0;
        let mob = MobInstance::new("시험:표적".to_string(), "시험".to_string(), "1", &data);
        let mut skill = Skill::from_json(
            "시험무공".to_string(),
            &serde_json::json!({
                "확률": 100,
                "타격률": 0.5,
                "공격": "1 공격 시험"
            }),
        )
        .unwrap();
        skill.probability = 100;

        // Normal roll 90; 90 + 50% = 135; level-11 mastery => 175.5 -> 175.
        assert_eq!(
            calculate_skill_result_with_rolls(&player, &skill, 11, &data, &mob, 100, 90, 100),
            (true, 175)
        );
    }

    #[test]
    fn test_calculate_exp_reward() {
        // Python: a=((10*10)//3)+30 == 63.
        assert_eq!(base_exp_reward(10, 10), 63);
        assert_eq!(base_exp_reward(10, 25), 53);
        assert_eq!(base_exp_reward(10, 3), 67);
        for _ in 0..100 {
            assert!((54..=72).contains(&calculate_exp_reward_for_level(10, 10)));
        }
    }
}
