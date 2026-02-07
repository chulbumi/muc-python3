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

    // c2 = 무기공격력 (숙련도 시스템은 나중에 구현)
    let c2 = weapon_power;

    // 방어력 계산: 몹 맷집 (strength = 맷집)
    // Python: mob.attpower (무기공격력) + mob.armor (방어구)
    // 현재는 strength만 사용 (몹 데이터에 별도 필드 없음)
    let mob_defense = mob_data.strength; // mob.getArm() + mob.getArmor()

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
    let mob_str = mob_data.strength as i64;
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
pub fn check_hit(player: &Body, mob_data: &RawMobData) -> bool {
    let player_dex = player.get_dex();
    let player_level = player.get_int("레벨");
    let mob_agility = mob_data.agility;
    let mob_level = mob_data.level;

    // Hit chance calculation
    let hit_chance =
        50 + (player_dex / 2) + (player_level * 2) - (mob_agility / 3) - (mob_level * 2);

    // Clamp to 5-95%
    let hit_chance = hit_chance.max(5).min(95);

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
    let attack_msgs = get_attack_message(player, &mob.name, damage, &particle);
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
fn get_attack_message(
    player: &Body,
    target_name: &str,
    damage: i64,
    particle: &str,
) -> (String, String) {
    let player_name = player.get_name();

    // Simple attack message
    let to_player = format!(
        "{} {} {}의 피해를 입혔습니다!",
        target_name, particle, damage
    );

    let to_room = format!(
        "{} {} {}의 피해를 입혔습니다.",
        player_name, target_name, damage
    );

    (to_player, to_room)
}

/// Calculate EXP reward from mob
fn calculate_exp_reward(mob_data: &RawMobData, player_level: i64) -> i64 {
    let base_exp = mob_data.level * 10;

    // Level difference adjustment
    let level_diff = player_level - mob_data.level;
    let adjusted_exp = if level_diff > 10 {
        base_exp / 4 // Much lower level mob
    } else if level_diff > 5 {
        base_exp / 2
    } else if level_diff < -5 {
        base_exp * 2 // Higher level mob bonus
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
    (base_gold * (100 + variation)) / 100.max(1)
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
        player.attpower = 20;

        let mut mob_data = RawMobData::new();
        mob_data.name = "테스트몹".to_string();
        mob_data.level = 8;
        mob_data.strength = 50; // 맷집 (방어력 - combined with inner_power)
        mob_data.inner_power = 20; // 내공 (defense contribution)
        mob_data.agility = 20;
        mob_data.max_hp = 100;
        mob_data.hp = 100;

        // Python 공식: (50*2 + 100/5 + 20) - (50) = 100 + 20 + 20 - 50 = 90
        // ±20%: 72 ~ 108
        let damage = calculate_player_damage(&player, &mob_data);
        assert!(damage > 0);
        // 최소 72, 최대 108 범위 내에 있어야 함 (랜덤이므로 넓게 체크)
        assert!(damage >= 50 && damage <= 130);
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
        assert!(damage >= 50 && damage <= 100);
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
