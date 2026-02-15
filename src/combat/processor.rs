//! PvM (Player vs Mob) Combat Processor
//!
//! Handles combat rounds between players and mobs including:
//! - Attack damage calculation
//! - Mob AI and counter-attacks
//! - Combat state management
//! - Death and rewards

use crate::hangul;
use crate::player::{ActState, Body};
use crate::world::{MobInstance, RawMobData, WorldState};
use rand::Rng;

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
    /// Player to attacker messages
    pub player_messages: Vec<String>,
    /// Room messages (to others in room)
    pub room_messages: Vec<String>,
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
            player_messages: Vec::new(),
            room_messages: Vec::new(),
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
    let mob_defense = mob_data.arm; // mob.getArm() + mob.getArmor()

    // 기본 데미지: (c1 + c2) - 방어력
    let mut damage = (c1 + c2) - mob_defense;

    // 최소 데미지 1 보장
    if damage < 1 {
        damage = 1;
    }

    // Python: ±20% 랜덤 변동 (0.8~1.2배)
    // randint(0, s1 - 1) + c1 where c1 = m * 0.80, c2 = m * 1.20
    let variation = rand::thread_rng().gen_range(-20..=20);
    damage = (damage * (100 + variation)) / 100;

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
    let mob_str = mob_data.strength;
    // 몹 공격력: 현재는 strength를 기본 공격력으로 사용
    // Python: mob.attpower는 사용아이템에서 계산됨
    let mob_weapon = mob_data.strength; // 기본 공격력

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

    // ±20% 랜덤 변동
    let variation = rand::thread_rng().gen_range(-20..=20);
    damage = (damage * (100 + variation)) / 100;

    // 최소 데미지 1 보장 (랜덤 후)
    if damage < 1 {
        damage = 1;
    }

    damage
}

/// Check if player hits the mob
///
/// Python formula (from objs/body.py:getAttackChance):
/// CHANCE = 100
/// bonus = self.getHit() * MAIN_CONFIG['명중확률']  # 0.2
/// bonus -= mob.getMiss() * MAIN_CONFIG['회피확률']  # 0.2
/// return CHANCE - (((mob['레벨']-self['레벨'])+90)//3) + bonus
pub fn check_hit(player: &Body, mob_data: &RawMobData) -> bool {
    const HIT_RATE: f64 = 0.2;  // MAIN_CONFIG['명중확률']
    const DODGE_RATE: f64 = 0.2;  // MAIN_CONFIG['회피확률']

    let player_level = player.get_int("레벨");
    let mob_level = mob_data.level;
    let player_hit = player.get_hit();
    let mob_miss = mob_data.agility; // mob.getMiss() uses agility

    // Python 공식
    let base_chance = 100;
    let level_modifier = ((mob_level - player_level) + 90) / 3;
    let bonus = (player_hit as f64 * HIT_RATE) - (mob_miss as f64 * DODGE_RATE);

    let hit_chance = base_chance - level_modifier + bonus as i64;

    // Clamp to 5-95%
    let hit_chance = hit_chance.clamp(5, 95);

    rand::thread_rng().gen_range(0..100) < hit_chance
}

/// Process player attack on mob
pub fn process_player_attack(
    player: &mut Body,
    mob_instance: &MobInstance,
    mob_data: &RawMobData,
) -> CombatRound {
    let mut round = CombatRound::new();

    let mob = mob_instance.clone();
    if !mob.alive {
        round
            .player_messages
            .push("☞ 이미 죽은 몬스터입니다.".to_string());
        round.combat_ended = true;
        return round;
    }

    // Set player to combat state
    player.act = ActState::Fight;

    // Check hit
    if !check_hit(player, mob_data) {
        let particle = hangul::han_obj(&mob.name);
        let msg = format!("{} {} 공격했지만 빗나갔습니다!", mob.name, particle);
        round.player_messages.push(msg.clone());
        round.room_messages.push(format!(
            "{} {} 공격했지만 빗나갔습니다.",
            player.get_name(),
            mob.name
        ));
        return round;
    }

    // Calculate damage
    let damage = calculate_player_damage(player, mob_data);
    round.damage_dealt = damage;

    // Format attack message
    let particle = hangul::han_obj(&mob.name);
    let attack_msgs = get_attack_message(player, &mob.name, damage, particle);
    round.player_messages.push(attack_msgs.0);
    round.room_messages.push(attack_msgs.1);

    // Check if mob died
    let new_hp = mob.hp - damage;
    if new_hp <= 0 {
        round.target_died = true;
        round.combat_ended = true;

        // Calculate rewards
        let exp = calculate_exp_reward(mob_data, player.get_int("레벨"));
        let gold = calculate_gold_reward(mob_data);

        // Death message
        let death_msg = format!("{} {} 쓰러뜨렸습니다!", mob.name, particle);
        round.player_messages.push(death_msg.clone());
        round.room_messages.push(death_msg);

        let reward_msg = format!("☞ 경험치 {} 획득! {} gold 획득!", exp, gold);
        round.player_messages.push(reward_msg);

        // Apply rewards
        player.add_exp(exp);
        // Add gold directly through object
        let current_gold = player.get_int("은전");
        player.set("은전", current_gold + gold);

        // Reset player state
        player.act = ActState::Stand;
    } else {
        // Mob counter-attacks
        let mob_damage = calculate_mob_damage(mob_data, player);
        round.damage_taken = mob_damage;

        player.set("체력", player.get_hp() - mob_damage);

        let mob_particle = hangul::han_obj(&mob.name);
        let counter_msg = format!(
            "{} {} {}의 피해를 입혔습니다!",
            mob.name, mob_particle, mob_damage
        );
        round.player_messages.push(counter_msg.clone());
        round.room_messages.push(format!(
            "{} {} {}의 피해를 입혔습니다.",
            mob.name,
            player.get_name(),
            mob_damage
        ));

        // Check if player died
        if player.get_hp() <= 0 {
            round.player_died = true;
            round.combat_ended = true;
            player.act = ActState::Death;

            let death_msg = format!("☞ {} 당했습니다...", mob.name);
            round.player_messages.push(death_msg);
            round.room_messages.push(format!(
                "{} {} 쓰러졌습니다.",
                player.get_name(),
                hangul::han_obj(&player.get_name())
            ));
        }
    }

    round
}

/// Get attack message (based on weapon type)
/// Python: objs/body.py getAttackScript(), makeFightScript()
fn get_attack_message(
    player: &Body,
    target_name: &str,
    damage: i64,
    _particle: &str,
) -> (String, String) {
    use crate::hangul::{post_position_all, strip_ansi};
    use rand::Rng;

    let player_name = player.get_name();
    let weapon_name = player.get_weapon_name();
    let fight_type = player.get_fight_script_type();

    // Combat scripts by weapon type (matching Python SCRIPT data)
    let scripts: &[( &str, &[&str] )] = &[
        ("주먹", &[
            "[공](이/가) [방](을/를) [무]으로 강타했습니다",
            "[공](이/가) [방]에게 [무]으로 정권을 날렸습니다",
            "[공](이/가) [방](을/를) 향해 [무]을(를) 휘둘렀습니다",
        ]),
        ("검", &[
            "[공](이/가) [방](을/를) [무]으로 베었습니다",
            "[공](이/가) [방]에게 [무]으로 예리한 일격을 가했습니다",
            "[공](이/가) [무]을(를) 휘둘러 [방](을/를) 공격했습니다",
        ]),
        ("도", &[
            "[공](이/가) [방](을/를) [무]으로 내리쳤습니다",
            "[공](이/가) [방]에게 [무]으로 강력한 타격을 입혔습니다",
            "[공](이/가) [무]으로 [방](을/를) 베어 넘겼습니다",
        ]),
        ("창", &[
            "[공](이/가) [방](을/를) [무]으로 찔렀습니다",
            "[공](이/가) [방]에게 [무]으로 뚫고 들어갔습니다",
            "[공](이/가) [무]을(를) 휘둘러 [방](을/를) 후렸습니다",
        ]),
        ("봉", &[
            "[공](이/가) [방](을/를) [무]으로 내리쳤습니다",
            "[공](이/가) [방]에게 [무]으로 후려쳤습니다",
            "[공](이/가) [무]으로 [방](을/를) 강타했습니다",
        ]),
    ];

    // Find matching script type (default to 주먹)
    let default_scripts: &[&str] = &[
        "[공](이/가) [방](을/를) 공격했습니다",
    ];
    let script_set = scripts
        .iter()
        .find(|(t, _)| fight_type.starts_with(t))
        .map(|(_, s)| *s)
        .unwrap_or(default_scripts);

    // Pick random script
    let script = if script_set.is_empty() {
        "[공](이/가) [방](을/를) 공격했습니다".to_string()
    } else {
        let idx = rand::thread_rng().gen_range(0..script_set.len());
        script_set[idx].to_string()
    };

    // For player message: [공] -> "당신", [방] -> target_name
    let mut to_player = script.replace("[공]", "당신");
    to_player = to_player.replace("[방]", target_name);
    to_player = to_player.replace("[무]", &weapon_name);
    // Apply Korean particles
    to_player = post_position_all(&to_player);
    // Add damage
    to_player = format!("{} ({} 피해)", to_player, damage);

    // For room message: [공] -> player_name, [방] -> target_name
    let player_name_clean = strip_ansi(&player_name);
    let mut to_room = script.replace("[공]", &player_name_clean);
    to_room = to_room.replace("[방]", target_name);
    to_room = to_room.replace("[무]", &weapon_name);
    // Apply Korean particles
    to_room = post_position_all(&to_room);
    // Add damage
    to_room = format!("{} ({} 피해)", to_room, damage);

    (to_player, to_room)
}

/// Calculate EXP reward from mob
///
/// Based on Python objs/mob.py die() and objs/body.py addExp()
/// Base exp scales with mob level and adjusts for player level difference
fn calculate_exp_reward(mob_data: &RawMobData, player_level: i64) -> i64 {
    // 기본 경험치: 몹 레벨 기반
    let mob_level = mob_data.level;
    let base_exp = if mob_level < 10 {
        mob_level * 5
    } else if mob_level < 50 {
        mob_level * 10
    } else if mob_level < 100 {
        mob_level * 15
    } else {
        mob_level * 20
    };

    // 레벨 차이에 따른 조정 (Python 스타일)
    let level_diff = player_level - mob_level;
    let adjusted_exp = if level_diff > 20 {
        base_exp / 4  // 너무 낮은 몹
    } else if level_diff > 10 {
        base_exp / 2
    } else if level_diff < -10 {
        base_exp * 2  // 높은 몹 보너스
    } else if level_diff < -5 {
        (base_exp * 150) / 100  // 1.5배
    } else {
        base_exp
    };

    adjusted_exp.max(1)
}

/// Calculate gold reward from mob
fn calculate_gold_reward(mob_data: &RawMobData) -> i64 {
    let base_gold = mob_data.level * 5;

    // Random variation ±50%
    let variation = rand::thread_rng().gen_range(-50..=50);
    (base_gold * (100 + variation)) / 100
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
    /// Message to show to the player
    pub player_message: String,
    /// Message to show to the room
    pub room_message: String,
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
    skill_name: &str,
    skill_level: i32,
    skill_bonus: i64,
    target_name: &str,
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
        11 => 1.3,   // 초급
        12 => 1.5,   // 중급
        13 => 1.7,   // 상급
        14 => 2.0,   // 고급
        15 => 2.5,   // 특급
        16..=100 => 3.0, // 절정 이상
        _ => 1.0,
    };

    // Calculate base damage
    let base_damage = (base_attack as f64 * level_multiplier * skill_multiplier * mastery_bonus) as i64;

    // Random variation ±20%
    let variation = rand::thread_rng().gen_range(-20..=20);
    let final_damage = ((base_damage as f64 * (100.0 + variation as f64)) / 100.0) as i64;
    let final_damage = final_damage.max(1);

    // Hit check (skills have higher hit rate)
    let hit_chance = 80 + (skill_level * 2);
    let hit = rand::thread_rng().gen_range(0..100) < hit_chance.min(95);

    // Generate messages
    let (player_message, room_message) = if hit {
        let to_player = format!(
            "{} 무공으로 {}에게 {}의 피해를 입혔습니다!",
            skill_name, target_name, final_damage
        );
        let to_room = format!(
            "{}이(가) {} 무공으로 {}에게 {}의 피해를 입혔습니다!",
            player.get_name(),
            skill_name,
            target_name,
            final_damage
        );
        (to_player, to_room)
    } else {
        let to_player = format!("{} 무공이 {}에게 빗나갔습니다!", skill_name, target_name);
        let to_room = format!(
            "{}이(가) {}에게 {} 무공을 시전했지만 빗나갔습니다!",
            player.get_name(),
            target_name,
            skill_name
        );
        (to_player, to_room)
    };

    SkillDamageResult {
        base_damage,
        final_damage,
        hit,
        player_message,
        room_message,
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
                format!("{} 무공으로 체력이 {} 회복되었습니다.", skill_name, hp_change)
            } else {
                format!("{} 무공의 부작용으로 체력이 {} 감소했습니다.", skill_name, hp_change.abs())
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
                format!("{} 무공으로 내공이 {} 회복되었습니다.", skill_name, mp_change)
            } else {
                format!("{} 무공의 부작용으로 내공이 {} 감소했습니다.", skill_name, mp_change.abs())
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

        // 공식: (40*2 + 40) - (10 + 40) = 80 + 40 - 50 = 70
        // ±20%: 56 ~ 84
        let damage = calculate_mob_damage(&mob_data, &player);
        assert!(damage > 0);
        assert!((50..=100).contains(&damage));
    }

    #[test]
    fn test_calculate_exp_reward() {
        let mut mob_data = RawMobData::new();
        mob_data.level = 10;

        // Same level
        let exp = calculate_exp_reward(&mob_data, 10);
        assert_eq!(exp, 100);

        // Much higher level player
        let exp_low = calculate_exp_reward(&mob_data, 25);
        assert!(exp_low < exp);

        // Lower level player
        let exp_high = calculate_exp_reward(&mob_data, 3);
        assert!(exp_high > exp);
    }
}
