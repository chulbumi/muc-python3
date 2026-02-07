//! 비전 (Secret Skill) commands for MUD engine
//!
//! Handles secret skill (비전) system - learning and training legendary skills.

use crate::command::registry::CommandRegistry;
use crate::command::CommandResult;
use crate::player::Body;
use std::sync::Arc;

/// Show or set 비전 (secret skill)
///
/// Based on cmds/비전.py
/// - No args: Show current 비전설정
/// - With args: Set 비전설정 to train (must be learned first)
fn vision_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        // Show current vision setting
        let vision = player.get_vision_setting();
        if vision.is_empty() {
            return CommandResult::Output("☞ 비전 : 없음".to_string());
        } else {
            return CommandResult::Output(format!("☞ 비전 : [\x1b[1;37m{}\x1b[0;37m]", vision));
        }
    }

    let skill_name = args[0];

    // Check if player has learned this secret skill
    if !player.has_secret_skill(skill_name) {
        return CommandResult::Error("☞ 당신은 그런 비전을 배운적이 없습니다.".to_string());
    }

    // Set vision setting
    player.set_vision_setting(skill_name);

    CommandResult::Output("☞ 비전을 지정하였습니다.".to_string())
}

/// Remove 비전설정 (vision setting)
///
/// Based on cmds/비전삭제.py
fn delete_vision_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let vision = player.get_vision_setting();
    if vision.is_empty() {
        return CommandResult::Error("☞ 지정된 비전이 없습니다.".to_string());
    }

    player.set_vision_setting("");

    CommandResult::Output("☞ 지정된 비전을 삭제합니다.".to_string())
}

/// Show learned secret skills
fn show_learned_visions_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let skills = player.get_secret_skills();

    if skills.is_empty() {
        return CommandResult::Output("☞ 배운 비전이 없습니다.".to_string());
    }

    let mut output = "☞ 배운 비전:\r\n".to_string();
    for (i, skill) in skills.iter().enumerate() {
        output += &format!("  {}. {}\r\n", i + 1, skill);
    }

    CommandResult::Output(output)
}

/// Show training secret skill status
fn show_training_vision_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let (training_skill, progress) = player.get_vision_training();

    if training_skill.is_empty() {
        return CommandResult::Output("☞ 수련 중인 비전이 없습니다.".to_string());
    }

    CommandResult::Output(format!(
        "☞ 수련 중인 비전: {} (진도: {}{})",
        training_skill,
        "█".repeat(progress as usize),
        if progress >= 10 { " 완료!" } else { "" }
    ))
}

/// Registers all 비전 commands
pub fn register_vision_commands(registry: &mut CommandRegistry) {
    // 비전 (Vision/Secret Skill)
    registry.register(crate::command::registry::CommandInfo {
        name: "비전".to_string(),
        aliases: vec!["secret".to_string(), "vision".to_string()],
        handler: Arc::new(vision_command),
        level: 0,
        description: "비전(오의)을 지정하거나 확인합니다.".to_string(),
        usage: "비전 ([비전이름])".to_string(),
    });

    // 비전삭제 (Delete Vision)
    registry.register(crate::command::registry::CommandInfo {
        name: "비전삭제".to_string(),
        aliases: vec!["비전삭제".to_string(), "delete_vision".to_string()],
        handler: Arc::new(delete_vision_command),
        level: 0,
        description: "지정된 비전을 삭제합니다.".to_string(),
        usage: "비전삭제".to_string(),
    });

    // 비전목록 (Show Learned Visions)
    registry.register(crate::command::registry::CommandInfo {
        name: "비전목록".to_string(),
        aliases: vec![
            "비전목록".to_string(),
            "secrets".to_string(),
            "learned_secrets".to_string(),
        ],
        handler: Arc::new(show_learned_visions_command),
        level: 0,
        description: "배운 비전 목록을 보여줍니다.".to_string(),
        usage: "비전목록".to_string(),
    });

    // 비전수련 (Show Training Vision)
    registry.register(crate::command::registry::CommandInfo {
        name: "비전수련".to_string(),
        aliases: vec!["비전수련".to_string(), "training_vision".to_string()],
        handler: Arc::new(show_training_vision_command),
        level: 0,
        description: "수련 중인 비전을 보여줍니다.".to_string(),
        usage: "비전수련".to_string(),
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
        player
    }

    #[test]
    fn test_register_vision_commands() {
        let mut registry = CommandRegistry::new();
        register_vision_commands(&mut registry);

        assert!(registry.contains("비전"));
        assert!(registry.contains("비전삭제"));
        assert!(registry.contains("비전목록"));
        assert!(registry.contains("비전수련"));
    }

    #[test]
    fn test_vision_command_no_vision() {
        let mut registry = CommandRegistry::new();
        register_vision_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("비전").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("없음"));
        }
    }

    #[test]
    fn test_vision_command_set_vision_not_learned() {
        let mut registry = CommandRegistry::new();
        register_vision_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("비전").unwrap();

        let result = (cmd.handler)(&mut player, &["강룡십팔장"]);
        assert!(matches!(result, CommandResult::Error(_)));

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("배운적이 없습니다"));
        }
    }

    #[test]
    fn test_vision_command_set_vision_learned() {
        let mut registry = CommandRegistry::new();
        register_vision_commands(&mut registry);

        let mut player = create_test_player();
        // Simulate having learned the skill
        player.set("비전이름", "강룡십팔장");

        let cmd = registry.get("비전").unwrap();

        let result = (cmd.handler)(&mut player, &["강룡십팔장"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("지정하였습니다"));
        }

        // Verify it was set
        assert_eq!(player.get_vision_setting(), "강룡십팔장");
    }

    #[test]
    fn test_delete_vision_command_no_vision() {
        let mut registry = CommandRegistry::new();
        register_vision_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("비전삭제").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("없습니다"));
        }
    }

    #[test]
    fn test_delete_vision_command_has_vision() {
        let mut registry = CommandRegistry::new();
        register_vision_commands(&mut registry);

        let mut player = create_test_player();
        player.set("비전설정", "강룡십팔장");

        let cmd = registry.get("비전삭제").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));

        // Verify it was cleared
        assert_eq!(player.get_vision_setting(), "");
    }

    #[test]
    fn test_has_secret_skill() {
        let mut player = create_test_player();
        assert!(!player.has_secret_skill("강룡십팔장"));

        player.set("비전이름", "강룡십팔장,비전검법");
        assert!(player.has_secret_skill("강룡십팔장"));
        assert!(player.has_secret_skill("비전검법"));
        assert!(!player.has_secret_skill("없는기술"));
    }

    #[test]
    fn test_add_secret_skill() {
        let mut player = create_test_player();
        assert!(!player.has_secret_skill("강룡십팔장"));

        player.add_secret_skill("강룡십팔장");
        assert!(player.has_secret_skill("강룡십팔장"));

        // Adding again shouldn't duplicate
        player.add_secret_skill("강룡십팔장");
        let skills = player.get_secret_skills();
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn test_vision_training() {
        let mut player = create_test_player();

        // No training initially
        let (skill, progress) = player.get_vision_training();
        assert!(skill.is_empty());
        assert_eq!(progress, 0);

        // Set training
        player.set_vision_training("비전무공", 5);
        let (skill, progress) = player.get_vision_training();
        assert_eq!(skill, "비전무공");
        assert_eq!(progress, 5);

        // Clear training
        player.clear_vision_training();
        let (skill, progress) = player.get_vision_training();
        assert!(skill.is_empty());
        assert_eq!(progress, 0);
    }

    #[test]
    fn test_get_vision_damage_modifier() {
        let mut player = create_test_player();

        // No vision set
        let (multiplier, _) = player.get_vision_damage_modifier("비전무공");
        assert_eq!(multiplier, 1.0);

        // Set vision to 비전무공
        player.set("비전설정", "비전무공");

        // Matching skill
        let (multiplier, desc) = player.get_vision_damage_modifier("비전무공");
        assert_eq!(multiplier, 0.5);
        assert!(desc.contains("비전"));
        assert!(desc.contains("피해가 절반"));

        // Non-matching skill
        let (multiplier, desc) = player.get_vision_damage_modifier("다른기술");
        assert_eq!(multiplier, 1.0);
        assert!(desc.is_empty());
    }
}
