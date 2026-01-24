//! Information commands for MUD engine
//!
//! Handles commands that show player information: 소지품, 봐, 점수, 도움말

use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::Body;
use crate::object::Value;
use std::sync::Arc;

/// Shows the player's inventory (소지품)
///
/// Based on cmds/소지품.py
fn inventory_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let separator = "─────────────────";
    let header = "\x1b[0m\x1b[44m\x1b[1m\x1b[37m  ◁     소     지     품     ▷  \x1b[0m\x1b[37m\x1b[40m";
    let footer = "\x1b[0m\x1b[47m\x1b[30m▶ 은전 : {:>20} 개 \x1b[0m\x1b[37m\x1b[40m";

    let mut output = vec![
        "━━━━━━━━━━━━━━━━━".to_string(),
        header.to_string(),
        separator.to_string(),
    ];

    // Count items by name (excluding in-use items)
    let mut item_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for obj in player.objs() {
        if let Ok(obj) = obj.lock() {
            // Skip items in use
            if obj.get("inUse") == Value::Int(1) {
                continue;
            }

            // Skip items with "출력안함" attribute
            if obj.checkAttr("아이템속성", "출력안함") {
                continue;
            }

            let name = obj.getName();
            *item_counts.entry(name).or_insert(0) += 1;
        }
    }

    if item_counts.is_empty() {
        output.push("\x1b[36m☞ 아무것도 없습니다.\x1b[37m".to_string());
    } else {
        for (name, count) in item_counts {
            if count == 1 {
                output.push(format!("\x1b[36m{}\x1b[37m", name));
            } else {
                output.push(format!("\x1b[36m{}  \x1b[36m{}개\x1b[37m", name, count));
            }
        }
    }

    output.push(separator.to_string());

    // Show money
    let money = player.get_int("은전");
    output.push(format!("\x1b[0m\x1b[47m\x1b[30m▶ 은전 : {:>20} 개 \x1b[0m\x1b[37m\x1b[40m", money));

    // Show gold if present
    let gold = player.get_int("금전");
    if gold > 0 {
        output.push(format!("\x1b[0m\x1b[43m\x1b[30m▶ 금전 : {:>20} 개 \x1b[0m\x1b[37m\x1b[40m", gold));
    }

    output.push(format!("{}{}", separator, "\x1b[0;37m"));

    CommandResult::Output(output.join("\r\n"))
}

/// Shows player stats/score (능력치/점수)
///
/// Based on cmds/능력치.py and cmds/상태보기.py
fn score_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let name = player.get_name();
    let level = player.get_int("레벨");
    let age = player.get_int("나이");

    // HP/MP
    let hp = player.get_hp();
    let max_hp = player.get_max_hp();
    let mp = player.get_mp();
    let max_mp = player.get_max_mp();

    // Stats
    let str_stat = player.get_str();
    let con = player.get_int("맷집");
    let dex = player.get_dex();

    // Experience
    let exp = player.get_int("현재경험치");

    // Money
    let money = player.get_int("은전");

    let mut output = vec![
        "┏━━━━━━━━━━━━━━━━━━━━━━━━━┑".to_string(),
        format!("│\x1b[0m\x1b[44m\x1b[1m\x1b[37m ▷▶▷▶▷ {:10}의 현재 상태     ◁◀◁◀◁ \x1b[0m\x1b[40m\x1b[37m│", name),
        "┝━━━━━━━━━━━━┯━━━━━━━━━━━━┥".to_string(),
        format!("│ [레  벨]       [{:>5}] │ [나  이]          {:>4} │", level, age),
        format!("│ [체  력]     {:>6}/{:<6} │ [내  공]     {:>6}/{:<6} │", hp, max_hp, mp, max_mp),
        format!("│ [  힘  ]       {:>6} │ [맷  집]       {:>6} │", str_stat, con),
        format!("│ [민  첩]       {:>6} │ [현  경]       {:>6} │", dex, exp),
        "├────────────┴────────────┤".to_string(),
        format!("│\x1b[0m\x1b[47m\x1b[30m [은  전]                    {:>20} \x1b[0m\x1b[40m\x1b[37m│", money),
        "┕━━━━━━━━━━━━━━━━━━━━━━━━━┙".to_string(),
    ];

    CommandResult::Output(output.join("\r\n"))
}

/// Shows help (도움말)
///
/// Based on cmds/도움말.py
fn help_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        // Show general help
        let output = vec![
            "=".repeat(50),
            "                    도  움  말".to_string(),
            "=".repeat(50),
            "".to_string(),
            "[기본 명령어]".to_string(),
            "  말 <내용>        - 주변 사람들에게 말합니다".to_string(),
            "  외쳐 <내용>      - 모든 사람들에게 외칩니다".to_string(),
            "  봐 <대상>        - 대상을 자세히 봅니다".to_string(),
            "  소지품           - 소지하고 있는 아이템을 봅니다".to_string(),
            "  능력치           - 자신의 능력치를 봅니다".to_string(),
            "".to_string(),
            "[이동 명령어]".to_string(),
            "  북/남/동/서      - 해당 방향으로 이동합니다".to_string(),
            "  위/아래          - 위/아래로 이동합니다".to_string(),
            "".to_string(),
            "[전투 명령어]".to_string(),
            "  쳐 <대상>        - 대상을 공격합니다".to_string(),
            "  도망             - 전투에서 도망칩니다".to_string(),
            "".to_string(),
            "상세 도움말: 도움말 <명령어>".to_string(),
            "=".repeat(50),
        ];
        CommandResult::Output(output.join("\r\n"))
    } else {
        // Show help for specific command
        let topic = args[0];
        let help_text = match topic.as_ref() {
            "말" | "say" => "말 <내용>\n  같은 방에 있는 사람들에게 말합니다.",
            "외쳐" | "shout" => "외쳐 <내용>\n  서버에 있는 모든 사람들에게 외칩니다.",
            "소지품" | "인벤토리" => "소지품\n  소지하고 있는 아이템 목록을 보여줍니다.",
            "능력치" | "점수" => "능력치\n  자신의 능력치와 상태를 보여줍니다.",
            "도망" | "flee" => "도망\n  전투 중에 도망칩니다.",
            _ => "☞ 해당 도움말이 없어요. ^^",
        };
        CommandResult::Output(help_text.to_string())
    }
}

/// Registers all info commands
pub fn register_info_commands(registry: &mut CommandRegistry) {
    // 소지품 (Inventory)
    registry.register(crate::command::registry::CommandInfo {
        name: "소지품".to_string(),
        aliases: vec!["소".to_string(), "소지".to_string(), "인벤토리".to_string(), "inventory".to_string()],
        handler: Arc::new(inventory_command),
        level: 0,
        description: "소지하고 있는 아이템을 보여줍니다.".to_string(),
        usage: "소지품".to_string(),
    });

    // 봐/보/look: 봐.rhai 스크립트로 처리. built_in_aliases에 보→봐, look→봐.

    // 능력치/점수 (Score)
    registry.register(crate::command::registry::CommandInfo {
        name: "능력치".to_string(),
        aliases: vec!["점수".to_string(), "score".to_string(), "stat".to_string()],
        handler: Arc::new(score_command),
        level: 0,
        description: "자신의 능력치를 보여줍니다.".to_string(),
        usage: "능력치".to_string(),
    });

    // 도움말 (Help)
    registry.register(crate::command::registry::CommandInfo {
        name: "도움말".to_string(),
        aliases: vec!["도움".to_string(), "help".to_string(), "?".to_string()],
        handler: Arc::new(help_command),
        level: 0,
        description: "도움말을 보여줍니다.".to_string(),
        usage: "도움말 [주제]".to_string(),
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
        player.set("나이", 25i64);
        player.set("체력", 80i64);
        player.set("최대체력", 100i64);
        player.set("내공", 50i64);
        player.set("최대내공", 100i64);
        player.set("힘", 15i64);
        player.set("맷집", 12i64);
        player.set("민첩", 14i64);
        player.set("현재경험치", 1000i64);
        player.set("은전", 5000i64);
        player
    }

    #[test]
    fn test_register_info_commands() {
        let mut registry = CommandRegistry::new();
        register_info_commands(&mut registry);

        assert!(registry.contains("소지품"));
        // 봐: 봐.rhai 스크립트로 처리
        assert!(registry.contains("능력치"));
        assert!(registry.contains("도움말"));
    }

    #[test]
    fn test_inventory_command_empty() {
        let mut registry = CommandRegistry::new();
        register_info_commands(&mut registry);

        let cmd = registry.get("소지품").unwrap();

        let result = (cmd.handler)(&mut Body::new(), &[]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("품"));
            assert!(msg.contains("아무것도 없습니다"));
        }
    }

    #[test]
    fn test_score_command() {
        let mut registry = CommandRegistry::new();
        register_info_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("능력치").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("테스터"));
            assert!(msg.contains("[레"));
            // Check for the spaced versions in the output format
            assert!(msg.contains("체") && msg.contains("력"));
            assert!(msg.contains("내") && msg.contains("공"));
        }
    }

    #[test]
    fn test_help_command_no_args() {
        let mut registry = CommandRegistry::new();
        register_info_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("도움말").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("도움말"));
            assert!(msg.contains("명령어"));
        }
    }

    #[test]
    fn test_help_command_with_topic() {
        let mut registry = CommandRegistry::new();
        register_info_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("도움말").unwrap();

        let result = (cmd.handler)(&mut player, &["말"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("말"));
        }
    }

    #[test]
    fn test_help_command_unknown_topic() {
        let mut registry = CommandRegistry::new();
        register_info_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("도움말").unwrap();

        let result = (cmd.handler)(&mut player, &["없는명령"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("해당 도움말이 없어요"));
        }
    }
}
