//! Combat commands for MUD engine
//!
//! Handles combat-related commands: 쳐 (attack), 도망 (flee), 시전 (cast)
//! Supports PvP (Player vs Player) and PvM (Player vs Mob) combat.

use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::{ActState, Body};
use crate::object::Value;
use crate::world::WorldState;
use crate::combat;
use crate::hangul;
use std::sync::Arc;
use rand::Rng;

/// PvP combat result
#[derive(Debug, Clone, PartialEq)]
pub enum PvPResult {
    /// Attack initiated successfully
    Success,
    /// PvP not allowed in this area
    SafeZone,
    /// Target is too high/low level
    LevelRestriction,
    /// Target has invalid state
    InvalidTarget,
    /// Attacker is in invalid state
    InvalidAttacker,
}

/// PvP configuration
#[derive(Debug, Clone)]
pub struct PvPConfig {
    /// Minimum level difference for PvP (e.g., 10 means can't attack if level diff > 10)
    pub max_level_diff: i64,
    /// PvP enabled in safe zones
    pub safe_zone_pvp: bool,
    /// PvP death penalty (percentage of exp lost, 0-100)
    pub death_penalty_pct: i64,
    /// PvP reward (exp gained on kill)
    pub kill_exp_reward: i64,
}

impl Default for PvPConfig {
    fn default() -> Self {
        Self {
            max_level_diff: 20,      // Allow PvP within 20 levels
            safe_zone_pvp: false,    // No PvP in safe zones
            death_penalty_pct: 5,    // 5% exp loss on death
            kill_exp_reward: 100,    // 100 exp for PvP kill
        }
    }
}

/// Check if PvP is allowed between two players
pub fn check_pvp_allowed(
    attacker: &Body,
    target: &Body,
    room_attrs: &[(String, String)],
    config: &PvPConfig,
) -> PvPResult {
    // Check attacker state
    if attacker.act != ActState::Stand && attacker.act != ActState::Fight {
        return PvPResult::InvalidAttacker;
    }

    // Check target state
    if target.act == ActState::Death {
        return PvPResult::InvalidTarget;
    }

    // Check for combat-forbidden attributes
    for (attr, _value) in room_attrs {
        if attr == "전투금지" || attr == "사용자전투금지" {
            if !config.safe_zone_pvp {
                return PvPResult::SafeZone;
            }
        }
    }

    // Check level difference
    let attacker_level = attacker.get_int("레벨");
    let target_level = target.get_int("레벨");
    let level_diff = (attacker_level - target_level).abs();

    if level_diff > config.max_level_diff {
        return PvPResult::LevelRestriction;
    }

    PvPResult::Success
}

/// Calculate PvP damage between players
pub fn calculate_pvp_damage(attacker: &Body, target: &Body) -> i64 {
    let attacker_str = attacker.get_str();
    let attacker_weapon = attacker.get_int("무기공격력");
    let attacker_level = attacker.get_int("레벨");

    let target_arm = target.get_arm();
    let target_dex = target.get_dex();

    // Base damage calculation
    let mut damage = (attacker_str / 2) + attacker_weapon + (attacker_level / 3);

    // Defense reduction
    let defense = target_arm + (target_dex / 10);
    damage = (damage * 100) / (100 + defense);

    // Add some randomness
    let variation = rand::thread_rng().gen_range(-10..=10);
    damage += variation;

    // Ensure minimum damage
    if damage < 1 {
        damage = 1;
    }

    // Cap maximum damage based on target max HP
    let max_damage = target.get_max_hp() / 4; // Max 25% of max HP per hit
    if damage > max_damage {
        damage = max_damage;
    }

    damage
}

/// Format PvP combat message
pub fn format_pvp_message(
    attacker_name: &str,
    target_name: &str,
    damage: i64,
    attacker_msg: &str,
    target_msg: &str,
    room_msg: &str,
) -> (String, String, String) {
    let particle = hangul::han_obj(target_name);

    let to_attacker = attacker_msg
        .replace("[공]", attacker_name)
        .replace("[방]", target_name)
        .replace("[대상]", target_name)
        .replace("[ particle]", &particle);

    let to_target = target_msg
        .replace("[공]", attacker_name)
        .replace("[방]", target_name)
        .replace("[대상]", target_name)
        .replace("[damage]", &damage.to_string());

    let to_room = room_msg
        .replace("[공]", attacker_name)
        .replace("[방]", target_name)
        .replace("[damage]", &damage.to_string());

    (to_attacker, to_target, to_room)
}

/// Attacks a target (쳐)
///
/// Supports both PvM (Player vs Mob) and PvP (Player vs Player)
fn attack_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 쳐 [대상]".to_string());
    }

    let target_name = args[0];

    // Check if player can attack (not dead)
    if player.act == ActState::Death {
        return CommandResult::Error("☞ 죽은 사람은 공격할 수 없습니다.".to_string());
    }

    // For now, use the combat processor if we can access world state
    // The full implementation would check world state for mobs
    // For now, initiate combat state
    player.act = ActState::Fight;
    player.temp_mut().insert("_attack_target".to_string(), Value::String(target_name.to_string()));

    CommandResult::Combat
}

/// Initiates PvP combat with a target player
///
/// Returns (success, error_message)
pub fn initiate_pvp(
    attacker: &mut Body,
    target_name: &str,
    world: &WorldState,
    config: &PvPConfig,
) -> (bool, String) {
    // Get attacker position
    let attacker_pos = match world.get_player_position(&attacker.get_name()) {
        Some(pos) => pos,
        None => return (false, "☞ 당신의 위치를 찾을 수 없습니다.".to_string()),
    };

    // Check if target exists in same room
    let players_in_room = world.get_players_in_room(&attacker_pos.zone, &attacker_pos.room);

    // Find the target
    let target_found = players_in_room.iter()
        .find(|name| *name == target_name || name.contains(target_name));

    let target_name = match target_found {
        Some(name) => name.clone(),
        None => return (false, "☞ 그런 상대가 없습니다.".to_string()),
    };

    // Check if attacking self
    if target_name == attacker.get_name() {
        return (false, "☞ 자신을 공격할 수 없습니다.".to_string());
    }

    // Get room attributes for PvP check
    let _room_attrs = world.room_attrs
        .get(&format!("{}:{}", attacker_pos.zone, attacker_pos.room))
        .map(|attrs| attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>())
        .unwrap_or_default();

    // For full implementation, would get target Body and check PvP rules
    // For now, initiate combat

    attacker.act = ActState::Fight;
    // Store target name in temp attributes for tracking
    attacker.temp_mut().insert("_pvp_target".to_string(), crate::object::Value::String(target_name.clone()));

    let msg = format!("{} {} 공격을 시작합니다!", target_name, hangul::han_obj(&target_name));
    (true, msg)
}

/// Flees from combat (도망)
///
/// Works for both PvM and PvP combat
fn flee_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    // Check if player is in combat
    if player.act != ActState::Fight {
        return CommandResult::Error("☞ 무림인은 아무때나 도망가는것이 아니라네".to_string());
    }

    // Check for flee cooldown (_runaway temp attribute)
    if player.temp().get("_runaway") == Some(&Value::Int(1)) {
        return CommandResult::Error("☞ 도망 갈려다 잡혔어요. '흑흑~~ T_T'".to_string());
    }

    // Set cooldown
    player.temp_mut().insert("_runaway".to_string(), Value::Int(1));

    // Calculate flee chance
    let player_level = player.get_int("레벨");
    let player_dex = player.get_dex();

    // Assume average mob/enemy level and dex for calculation
    let enemy_level = player_level.max(1);
    let enemy_dex = player_dex;

    let mut c1 = enemy_level * (enemy_dex + 1) - player_level * (player_dex + 1);
    if c1 < 1 {
        c1 = 1;
    }
    c1 = 100 - c1;
    if c1 < 10 {
        c1 = 10;
    }

    // Random check
    let roll = rand::thread_rng().gen_range(0..100);

    if roll > c1 {
        return CommandResult::Error("☞ 도망 갈려다 잡혔어요. '흑흑~~ T_T'".to_string());
    }

    // Clear combat state on successful flee
    player.act = ActState::Stand;
    player.clear_all_targets();

    CommandResult::Output("☞ 성공적으로 도망쳤습니다!".to_string())
}

/// Uses a skill (시전)
///
/// Supports both PvM and PvP skills
fn cast_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 시전 [스킬명] ([대상])".to_string());
    }

    let skill_name = args[0];

    // Check if player knows the skill
    // In full implementation, check against player.skillList
    // For now, just acknowledge

    CommandResult::Output(format!("{} 무공을 펼칩니다!", skill_name))
}

/// Looks at combat status (전투상태)
fn combat_status_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    if player.act != ActState::Fight {
        return CommandResult::Output("☞ 현재 전투중이 아닙니다.".to_string());
    }

    let name = player.get_name();
    let hp = player.get_hp();
    let max_hp = player.get_max_hp();
    let mp = player.get_mp();
    let max_mp = player.get_max_mp();

    // Get target from temp attributes
    let target_list = if let Some(crate::object::Value::String(target)) = player.temp().get("_pvp_target") {
        target.clone()
    } else {
        "없음".to_string()
    };

    let output = format!(
        "★ {}의 전투 상태 ★\r\n체력: {}/{}\r\n내공: {}/{}\r\n대상: {}",
        name, hp, max_hp, mp, max_mp, target_list
    );

    CommandResult::Output(output)
}

/// PvP duel command (결투)
///
/// Initiates a formal PvP duel with consent required
fn duel_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 결투 [상대방]".to_string());
    }

    let target_name = args[0];

    // Check if target is self
    if target_name == player.get_name() {
        return CommandResult::Error("☞ 자신과 결투할 수 없습니다.".to_string());
    }

    // In full implementation, this would:
    // 1. Check if target is in same room
    // 2. Send duel request to target
    // 3. Wait for target acceptance
    // 4. Start combat on both sides

    CommandResult::Output(format!("{}님에게 결투를 신청합니다.", target_name))
}

/// Accepts a duel request (결투수락)
fn accept_duel_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    // In full implementation, check for pending duel requests
    CommandResult::Output("☞ 대기 중인 결투 신청이 없습니다.".to_string())
}

/// Declines a duel request (결투거절)
fn decline_duel_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    // In full implementation, check for and decline pending duel requests
    CommandResult::Output("☞ 대기 중인 결투 신청이 없습니다.".to_string())
}

/// Registers all combat commands
pub fn register_combat_commands(registry: &mut CommandRegistry) {
    // 쳐 (Attack)
    registry.register(crate::command::registry::CommandInfo {
        name: "쳐".to_string(),
        aliases: vec!["공격".to_string(), "때려".to_string(), "attack".to_string(),
                       "kill".to_string(), "k".to_string()],
        handler: Arc::new(attack_command),
        level: 0,
        description: "대상을 공격합니다. PvP 지원.".to_string(),
        usage: "쳐 [대상]".to_string(),
    });

    // 도망 (Flee)
    registry.register(crate::command::registry::CommandInfo {
        name: "도망".to_string(),
        aliases: vec!["도".to_string(), "flee".to_string(), "run".to_string()],
        handler: Arc::new(flee_command),
        level: 0,
        description: "전투에서 도망칩니다.".to_string(),
        usage: "도망".to_string(),
    });

    // 시전 (Cast skill)
    registry.register(crate::command::registry::CommandInfo {
        name: "시전".to_string(),
        aliases: vec!["시".to_string(), "cast".to_string(), "skill".to_string()],
        handler: Arc::new(cast_command),
        level: 0,
        description: "무공/스킬을 시전합니다.".to_string(),
        usage: "시전 [스킬명] ([대상])".to_string(),
    });

    // 전투상태 (Combat status)
    registry.register(crate::command::registry::CommandInfo {
        name: "전투상태".to_string(),
        aliases: vec!["전상".to_string(), "combat".to_string(), "status".to_string()],
        handler: Arc::new(combat_status_command),
        level: 0,
        description: "현재 전투 상태를 보여줍니다.".to_string(),
        usage: "전투상태".to_string(),
    });

    // 결투 (Duel) - PvP
    registry.register(crate::command::registry::CommandInfo {
        name: "결투".to_string(),
        aliases: vec!["duel".to_string(), "pvp".to_string()],
        handler: Arc::new(duel_command),
        level: 0,
        description: "다른 플레이어에게 결투를 신청합니다.".to_string(),
        usage: "결투 [상대방]".to_string(),
    });

    // 결투수락 (Accept duel)
    registry.register(crate::command::registry::CommandInfo {
        name: "결투수락".to_string(),
        aliases: vec!["수락".to_string(), "accept".to_string(), "yes".to_string()],
        handler: Arc::new(accept_duel_command),
        level: 0,
        description: "결투 신청을 수락합니다.".to_string(),
        usage: "결투수락".to_string(),
    });

    // 결투거절 (Decline duel)
    registry.register(crate::command::registry::CommandInfo {
        name: "결투거절".to_string(),
        aliases: vec!["거절".to_string(), "decline".to_string(), "no".to_string()],
        handler: Arc::new(decline_duel_command),
        level: 0,
        description: "결투 신청을 거절합니다.".to_string(),
        usage: "결투거절".to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandRegistry;

    fn create_test_player() -> Body {
        let mut player = Body::new();
        player.set("이름", "테스터");
        player.set("레벨", 10i64);
        player.set("민첩", 15i64);
        player.set("체력", 80i64);
        player.set("최대체력", 100i64);
        player.act = ActState::Stand;
        player
    }

    #[test]
    fn test_register_combat_commands() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        assert!(registry.contains("쳐"));
        assert!(registry.contains("도망"));
        assert!(registry.contains("시전"));
        assert!(registry.contains("전투상태"));
        assert!(registry.contains("결투"));
    }

    #[test]
    fn test_pvp_config_default() {
        let config = PvPConfig::default();
        assert_eq!(config.max_level_diff, 20);
        assert_eq!(config.safe_zone_pvp, false);
        assert_eq!(config.death_penalty_pct, 5);
        assert_eq!(config.kill_exp_reward, 100);
    }

    #[test]
    fn test_calculate_pvp_damage() {
        let mut attacker = Body::new();
        attacker.set("이름", "공격자");
        attacker.set("레벨", 20i64);
        attacker.set("힘", 50i64);
        attacker.set("무기공격력", 30i64);

        let mut target = Body::new();
        target.set("이름", "방어자");
        target.set("레벨", 20i64);
        target.set("맷집", 40i64);
        target.set("민첩", 30i64);
        target.set("최대체력", 200i64);

        let damage = calculate_pvp_damage(&attacker, &target);
        assert!(damage > 0);
        assert!(damage <= 50); // Max 25% of 200
    }

    #[test]
    fn test_format_pvp_message() {
        let (to_attacker, to_target, to_room) = format_pvp_message(
            "철사",
            "영걸",
            25,
            "[공]이 [방][ particle] 후려칩니다!",
            "[공]이 당신에게 [damage]의 피해를 입혔습니다!",
            "[공]이 [방]을 공격하여 [damage]의 피해를 입혔습니다!",
        );

        assert!(to_attacker.contains("철사"));
        assert!(to_attacker.contains("영걸"));
        assert!(to_target.contains("25"));
        assert!(to_room.contains("25"));
    }

    #[test]
    fn test_attack_command_no_target() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("쳐").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_attack_command_with_target() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("쳐").unwrap();

        let result = (cmd.handler)(&mut player, &["몬스터"]);
        assert!(matches!(result, CommandResult::Combat));
    }

    #[test]
    fn test_flee_command_not_fighting() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("도망").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_flee_command_success() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        player.act = ActState::Fight;

        let cmd = registry.get("도망").unwrap();

        // Could succeed or fail based on RNG
        let _result = (cmd.handler)(&mut player, &[]);
    }

    #[test]
    fn test_combat_status_not_fighting() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("전투상태").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));
    }

    #[test]
    fn test_combat_status_fighting() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        player.act = ActState::Fight;
        player.temp_mut().insert("_pvp_target".to_string(), crate::object::Value::String("적".to_string()));

        let cmd = registry.get("전투상태").unwrap();
        let result = (cmd.handler)(&mut player, &[]);

        assert!(matches!(result, CommandResult::Output(_)));
    }

    #[test]
    fn test_duel_command_no_target() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("결투").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_duel_command_with_target() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("결투").unwrap();

        let result = (cmd.handler)(&mut player, &["상대방"]);
        assert!(matches!(result, CommandResult::Output(_)));
    }

    #[test]
    fn test_pvp_check_level_restriction() {
        let config = PvPConfig {
            max_level_diff: 10,
            ..Default::default()
        };

        let mut attacker = Body::new();
        attacker.set("레벨", 10i64);
        attacker.act = ActState::Stand;

        let mut target = Body::new();
        target.set("레벨", 30i64); // 20 level difference
        target.act = ActState::Stand;

        let room_attrs = vec![];
        let result = check_pvp_allowed(&attacker, &target, &room_attrs, &config);
        assert_eq!(result, PvPResult::LevelRestriction);
    }

    #[test]
    fn test_pvp_check_safe_zone() {
        let config = PvPConfig::default();

        let mut attacker = Body::new();
        attacker.set("레벨", 20i64);
        attacker.act = ActState::Stand;

        let mut target = Body::new();
        target.set("레벨", 20i64);
        target.act = ActState::Stand;

        let room_attrs = vec![
            ("전투금지".to_string(), "".to_string()),
        ];
        let result = check_pvp_allowed(&attacker, &target, &room_attrs, &config);
        assert_eq!(result, PvPResult::SafeZone);
    }
}
