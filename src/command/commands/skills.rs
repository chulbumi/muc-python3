//! Skill (무공/기술) commands for MUD engine
//!
//! Handles displaying player's learned skills and training status.

use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::Body;
use std::sync::Arc;

/// Skill level types matching Python Body.skillLvType
pub const SKILL_LEVEL_TYPES: &[&str] = &[
    "초급", "중급", "상급", "고급", "특급", "절정", "초절정", "회복", "방어", "기타"
];

/// Skill level values matching Python Body.skillLv
pub fn get_skill_level_value(level_name: &str) -> i64 {
    match level_name {
        "초급" => 1,
        "중급" => 2,
        "상급" => 3,
        "고급" => 4,
        "특급" => 5,
        "절정" => 6,
        "초절정" => 7,
        _ => 1,
    }
}

/// Fill string with spaces to specified width (Python lib.func.fillSpace)
pub fn fill_space(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        return text.to_string();
    }
    format!("{}{}", text, " ".repeat(width - text_len))
}

/// Show player's skills (무공/기술)
///
/// Based on cmds/무공.py
fn mugong_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let _name = player.get_name();

    let mut output = format!(
        "\r\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n\
        \x1b[0m\x1b[47m\x1b[30m{:━<71}\x1b[0m\x1b[40m\x1b[37m\r\n\
        ───────────────────────────────────────\r\n",
        format!("◁ 당신의 무공 ▷")
    );

    // Get learned skills
    let skill_list = player.get_string("무공이름");
    let skills: Vec<&str> = if skill_list.is_empty() {
        Vec::new()
    } else {
        skill_list.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect()
    };

    if skills.is_empty() {
        output += "☞ 깨우친 무공이 없습니다.\r\n";
    } else {
        // Group skills by level type
        // Would need to load from MAIN_CONFIG - for now just list them
        output += "\x1b[1m\x1b[40m\x1b[32m▷ 기술 목록\x1b[0m\x1b[40m\x1b[37m\r\n";

        // Get skill map for levels
        let skill_training = player.get_string("무공숙련도");
        let skill_levels: std::collections::HashMap<String, i64> = parse_skill_training(&skill_training);

        let mut count = 0;
        for skill_name in &skills {
            let level = skill_levels.get(*skill_name).copied().unwrap_or(1);
            let buf = format!("{}({}성)", skill_name, level);
            output += &format!(" ◇ {} ", fill_space(&buf, 20));
            count += 1;
            if count % 3 == 0 {
                output += "\r\n";
            }
        }
        if count % 3 != 0 {
            output += "\r\n";
        }
    }

    output += "───────────────────────────────────────\r\n";

    // Secret skills (비전)
    output += "\x1b[1m\x1b[40m\x1b[32m▷ 비전\x1b[0m\x1b[40m\x1b[37m\r\n";

    let secret_training = player.get_string("비전수련");
    let secret_learned = player.get_string("비전이름");

    if secret_training.is_empty() && secret_learned.is_empty() {
        output += "☞ 오의를 깨우친 무공이 없습니다.\r\n";
    } else {
        let mut count = 0;

        // Currently training
        if !secret_training.is_empty() {
            output += &format!("\x1b[1m\x1b[33m{}\x1b[0m\x1b[40m\x1b[37m(수련중)\r\n", secret_training);
            count += 1;
        }

        // Fully learned secret skills
        if !secret_learned.is_empty() {
            let secrets: Vec<&str> = secret_learned.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            for secret in secrets {
                output += &format!(" ◇ {} ", fill_space(secret, 20));
                count += 1;
                if count % 3 == 0 {
                    output += "\r\n";
                }
            }
        }

        if count % 3 != 0 {
            output += "\r\n";
        }
    }

    output += "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n";

    CommandResult::Output(output)
}

/// Parse skill training data from string format
/// Format: "skill1 level1 exp1,skill2 level2 exp2"
fn parse_skill_training(data: &str) -> std::collections::HashMap<String, i64> {
    let mut map = std::collections::HashMap::new();

    if data.is_empty() {
        return map;
    }

    for entry in data.split(',') {
        let parts: Vec<&str> = entry.split_whitespace().collect();
        if parts.len() >= 2 {
            let skill_name = parts[0];
            if let Ok(level) = parts[1].parse::<i64>() {
                map.insert(skill_name.to_string(), level);
            }
        }
    }

    map
}

/// Show another player's skills (admin only)
fn inspect_mugong_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 기술 [대상]".to_string());
    }

    // Check if player is admin
    let admin_level = player.get_int("관리자등급");
    if admin_level < 1000 {
        return CommandResult::Error("☞ 관리자만 사용할 수 있습니다.".to_string());
    }

    // Would need to find target player in world
    // For now, return not implemented
    CommandResult::Error(format!("☞ {}의 무공을 조회합니다 (구현 예정)", args[0]))
}

/// Registers all skill commands
pub fn register_skill_commands(registry: &mut CommandRegistry) {
    // 무공 (Mugong/Skills)
    registry.register(crate::command::registry::CommandInfo {
        name: "무공".to_string(),
        aliases: vec!["기술".to_string(), "skills".to_string(), "skill".to_string()],
        handler: Arc::new(mugong_command),
        level: 0,
        description: "습득한 무공/기술 목록을 보여줍니다.".to_string(),
        usage: "무공 ([대상])".to_string(),
    });

    // 기술보기 (Inspect skills - admin)
    registry.register(crate::command::registry::CommandInfo {
        name: "기술보기".to_string(),
        aliases: vec!["inspect_skills".to_string()],
        handler: Arc::new(inspect_mugong_command),
        level: 1000,
        description: "다른 플레이어의 기술을 확인합니다.".to_string(),
        usage: "기술보기 [대상]".to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandRegistry;

    #[test]
    fn test_fill_space() {
        assert_eq!(fill_space("test", 10), "test      ");
        assert_eq!(fill_space("한글", 10), "한글        ");
        assert_eq!(fill_space("verylongname", 5), "verylongname");
    }

    #[test]
    fn test_get_skill_level_value() {
        assert_eq!(get_skill_level_value("초급"), 1);
        assert_eq!(get_skill_level_value("중급"), 2);
        assert_eq!(get_skill_level_value("상급"), 3);
        assert_eq!(get_skill_level_value("절정"), 6);
        assert_eq!(get_skill_level_value("초절정"), 7);
        assert_eq!(get_skill_level_value("unknown"), 1);
    }

    #[test]
    fn test_parse_skill_training() {
        let data = "검법 3 500,검술 5 1000";
        let map = parse_skill_training(data);

        assert_eq!(map.get("검법"), Some(&3));
        assert_eq!(map.get("검술"), Some(&5));
        assert_eq!(map.get("없는기술"), None);
    }

    #[test]
    fn test_register_skill_commands() {
        let mut registry = CommandRegistry::new();
        register_skill_commands(&mut registry);

        assert!(registry.contains("무공"));
        assert!(registry.contains("기술"));
        assert!(registry.contains("skills"));
    }

    #[test]
    fn test_mugong_command_no_args() {
        let mut registry = CommandRegistry::new();
        register_skill_commands(&mut registry);

        let mut player = Body::new();
        let cmd = registry.get("무공").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));
    }

    #[test]
    fn test_inspect_mugong_not_admin() {
        let mut registry = CommandRegistry::new();
        register_skill_commands(&mut registry);

        let mut player = Body::new();
        player.set("관리자등급", 100i64);

        let cmd = registry.get("기술보기").unwrap();

        let result = (cmd.handler)(&mut player, &["target"]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_inspect_mugong_no_args() {
        let mut registry = CommandRegistry::new();
        register_skill_commands(&mut registry);

        let mut player = Body::new();
        player.set("관리자등급", 1000i64);

        let cmd = registry.get("기술보기").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }
}
