//! Combat commands for MUD engine
//!
//! Handles combat-related commands: 쳐 (attack), 도망 (flee)

use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::{ActState, Body};
use crate::object::Value;
use std::sync::Arc;
use rand::Rng;

/// Attacks a target (쳐)
///
/// Based on combat in Python - attacks initiate combat with mobs
fn attack_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 공격할 대상을 지정해주세요.".to_string());
    }

    let target = args[0];

    // If already fighting, continue attacking current target
    if player.act == ActState::Fight {
        // Already in combat
        return CommandResult::Combat;
    }

    // Check if target exists in room
    // In Python: obj = ob.env.findObjName(line)
    // For now, we'll initiate combat

    CommandResult::Combat
}

/// Flees from combat (도망)
///
/// Based on cmds/도망.py
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
    // In Python: c1 = mob['레벨'] * (mob.getDex() + 1) - ob['레벨'] * (ob.getDex() + 1)
    // c1 = 100 - c1, min 10
    let player_level = player.get_int("레벨");
    let player_dex = player.get_dex();

    // Assume mob level/dex for calculation
    let mob_level = player_level.max(1); // Would get from actual mob
    let mob_dex = player_dex;

    let mut c1 = mob_level * (mob_dex + 1) - player_level * (player_dex + 1);
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

    // Find a random exit to flee through
    // In Python: room, dir = ob.env.getRandomExit()
    // For now, return success - the movement system will handle the actual flee

    CommandResult::Ok
}

/// Uses a skill (시전)
///
/// Based on the skill system in Python
fn cast_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 시전 [스킬명]".to_string());
    }

    let skill_name = args[0];

    // Check if player knows the skill
    // In Python: check against player.skillList

    CommandResult::Output(format!("{} 시전!", skill_name))
}

/// Looks at combat status (전투상태)
fn combat_status_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    if player.act != ActState::Fight {
        return CommandResult::Output("☞ 현재 전투중이 아닙니다.".to_string());
    }

    let name = player.get_name();
    let hp = player.get_hp();
    let max_hp = player.get_max_hp();

    let output = format!(
        "★ {}의 전투 상태 ★\r\n체력: {}/{}",
        name, hp, max_hp
    );

    CommandResult::Output(output)
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
        description: "대상을 공격합니다.".to_string(),
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
        usage: "시전 [스킬명]".to_string(),
    });

    // 전투상태 (Combat status)
    registry.register(crate::command::registry::CommandInfo {
        name: "전투상태".to_string(),
        aliases: vec!["전상".to_string(), "combat".to_string()],
        handler: Arc::new(combat_status_command),
        level: 0,
        description: "현재 전투 상태를 보여줍니다.".to_string(),
        usage: "전투상태".to_string(),
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
    }

    #[test]
    fn test_attack_command_no_target() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("쳐").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("대상"));
        }
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
    fn test_attack_command_while_fighting() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        player.act = ActState::Fight;

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

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("무림인"));
        }
    }

    #[test]
    fn test_flee_command_success() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        player.act = ActState::Fight;

        let cmd = registry.get("도망").unwrap();

        // First attempt sets cooldown
        let _result = (cmd.handler)(&mut player, &[]);
        // Result could be Ok or Error depending on RNG

        // Second attempt should fail due to cooldown
        let result2 = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result2, CommandResult::Error(_)));
    }

    #[test]
    fn test_flee_command_cooldown() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        player.act = ActState::Fight;

        let cmd = registry.get("도망").unwrap();

        // Manually set cooldown
        player.temp_mut().insert("_runaway".to_string(), Value::Int(1));

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("잡혔어요"));
        }
    }

    #[test]
    fn test_cast_command_no_args() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("시전").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_cast_command_with_skill() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("시전").unwrap();

        let result = (cmd.handler)(&mut player, &["화염구"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("화염구"));
            assert!(msg.contains("시전"));
        }
    }

    #[test]
    fn test_combat_status_not_fighting() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("전투상태").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("전투중이 아닙니다"));
        }
    }

    #[test]
    fn test_combat_status_fighting() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        let mut player = create_test_player();
        player.act = ActState::Fight;

        let cmd = registry.get("전투상태").unwrap();
        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("전투 상태"));
            assert!(msg.contains("체력"));
        }
    }
}
