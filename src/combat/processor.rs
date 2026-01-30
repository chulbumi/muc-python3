//! PvM (Player vs Mob) Combat Processor
//!
//! Handles combat rounds between players and mobs including:
//! - Attack damage calculation
//! - Mob AI and counter-attacks
//! - Combat state management
//! - Death and rewards

use crate::player::{ActState, Body};
use crate::world::{WorldState, MobInstance, RawMobData};
use crate::hangul;
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
pub fn calculate_player_damage(player: &Body, mob_data: &RawMobData) -> i64 {
    let player_str = player.get_str();
    let player_dex = player.get_dex();
    let player_level = player.get_int("레벨");
    let weapon_power = player.attpower;

    // Base damage
    let mut damage = (player_str / 2) + (weapon_power as i64) + (player_level / 2);

    // Player attack bonus from equipment
    let weapon_bonus = player.get_int("무기공격력");
    damage += weapon_bonus;

    // Defense calculation
    let mob_defense = (mob_data.strength / 3) + (mob_data.agility / 5);
    damage = (damage * 100) / (100 + mob_defense);

    // Dexterity bonus to hit chance and damage
    let dex_bonus = player_dex / 10;
    damage += dex_bonus;

    // Random variation ±20%
    let variation = rand::thread_rng().gen_range(-20..=20);
    damage = (damage * (100 + variation)) / 100;

    // Minimum damage
    if damage < 1 {
        damage = 1;
    }

    // Max damage cap (based on mob max HP)
    let max_damage = (mob_data.max_hp * 30) / 100; // Max 30% of mob HP per hit
    if damage > max_damage {
        damage = max_damage;
    }

    damage
}

/// Calculate mob damage to player
pub fn calculate_mob_damage(mob_data: &RawMobData, player: &Body) -> i64 {
    let mob_str = mob_data.strength;
    let mob_level = mob_data.level;
    let player_armor = player.armor;
    let player_arm = player.get_arm();
    let player_dex = player.get_dex();

    // Base damage
    let mut damage = (mob_str / 2) + (mob_level * 2);

    // Defense calculation
    let defense = player_armor + (player_arm as i32) + (player_dex / 10) as i32;
    damage = (damage * 100) / (100 + defense as i64);

    // Random variation ±15%
    let variation = rand::thread_rng().gen_range(-15..=15);
    damage = (damage * (100 + variation)) / 100;

    // Minimum damage
    if damage < 1 {
        damage = 1;
    }

    // Max damage cap (based on player max HP)
    let max_hp = player.get_max_hp();
    let max_damage = (max_hp * 25) / 100; // Max 25% of player HP per hit
    if damage > max_damage {
        damage = max_damage;
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
    let hit_chance = 50 + (player_dex / 2) + (player_level * 2) - (mob_agility / 3) - (mob_level * 2);

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
        round.player_messages.push("☞ 이미 죽은 몬스터입니다.".to_string());
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
        round.room_messages.push(format!("{} {} 공격했지만 빗나갔습니다.", player.get_name(), mob.name));
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
        let counter_msg = format!("{} {} {}의 피해를 입혔습니다!",
            mob.name, mob_particle, mob_damage);
        round.player_messages.push(counter_msg.clone());
        round.room_messages.push(format!("{} {} {}의 피해를 입혔습니다.",
            mob.name, player.get_name(), mob_damage));

        // Check if player died
        if player.get_hp() <= 0 {
            round.player_died = true;
            round.combat_ended = true;
            player.act = ActState::Death;

            let death_msg = format!("☞ {} 당했습니다...", mob.name);
            round.player_messages.push(death_msg);
            round.room_messages.push(format!("{} {} 쓰러졌습니다.", player.get_name(), hangul::han_obj(&player.get_name())));
        }
    }

    round
}

/// Get attack message (based on weapon type)
fn get_attack_message(player: &Body, target_name: &str, damage: i64, particle: &str) -> (String, String) {
    let player_name = player.get_name();

    // Simple attack message
    let to_player = format!("{} {} {}의 피해를 입혔습니다!",
        target_name, particle, damage);

    let to_room = format!("{} {} {}의 피해를 입혔습니다.",
        player_name, target_name, damage);

    (to_player, to_room)
}

/// Calculate EXP reward from mob
fn calculate_exp_reward(mob_data: &RawMobData, player_level: i64) -> i64 {
    let base_exp = mob_data.level * 10;

    // Level difference adjustment
    let level_diff = player_level - mob_data.level;
    let adjusted_exp = if level_diff > 10 {
        base_exp / 4  // Much lower level mob
    } else if level_diff > 5 {
        base_exp / 2
    } else if level_diff < -5 {
        base_exp * 2  // Higher level mob bonus
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

    .max(1)
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
        let mob_instances = world.mob_cache.get_mobs_in_room(&player_pos.zone, &player_pos.room);
        for mob in mob_instances {
            if mob.name == target_name {
                if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                    return Some((mob.clone(), mob_data.clone()));
                }
            }
        }
    }

    // Then try partial match
    let mob_instances = world.mob_cache.get_mobs_in_room(&player_pos.zone, &player_pos.room);
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
        player.attpower = 20;

        let mut mob_data = RawMobData::new();
        mob_data.name = "테스트몹".to_string();
        mob_data.level = 8;
        mob_data.strength = 40;
        mob_data.agility = 20;
        mob_data.max_hp = 100;
        mob_data.hp = 100;

        let damage = calculate_player_damage(&player, &mob_data);
        assert!(damage > 0);
        assert!(damage <= 30); // Max 30% of 100
    }

    #[test]
    fn test_calculate_mob_damage() {
        let mut player = Body::new();
        player.set("레벨", 10i64);
        player.set("맷집", 40i64);
        player.set("민첩", 30i64);
        player.armor = 10;
        player.set("최대체력", 100i64);

        let mut mob_data = RawMobData::new();
        mob_data.level = 8;
        mob_data.strength = 40;

        let damage = calculate_mob_damage(&mob_data, &player);
        assert!(damage > 0);
        assert!(damage <= 25); // Max 25% of player max HP
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
