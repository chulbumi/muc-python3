//! Communication commands for MUD engine
//!
//! Handles player communication: 말 (say), 외쳐 (shout)

use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::Body;
use crate::hangul;
use std::sync::Arc;

/// Says something to the room (말)
///
/// Based on cmds/말.py
fn say_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("\r\nSay What???".to_string());
    }

    let message = args.join(" ");
    let name = player.get_name();

    // In Python: format is "당신이 말합니다 : '{message}'"
    // and to room: "{name} 말합니다 : '{message}'"

    // Use Korean particle
    let iga = hangul::han_iga(&name);

    let self_msg = format!("당신이 말합니다 : '{}'", message);
    let room_msg = format!("{} 말합니다 : '{}'", format!("{}\x1b[33m{}\x1b[37m", name, iga), message);

    // Return both messages - the system will handle sending to room
    let output = format!("{}\r\n{}", self_msg, room_msg);

    CommandResult::Output(output)
}

/// Shouts to the entire server (외쳐)
///
/// Based on cmds/외쳐.py
fn shout_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: [내용] 외침(,)".to_string());
    }

    let message = args.join(" ");

    // Check length limit
    if message.len() > 160 {
        return CommandResult::Error("☞ 너무 길어요. ^^".to_string());
    }

    // Check if player is blocking shouts (외침거부)
    // In Python: ob.checkConfig('외침거부')
    let config = player.get_string("설정상태");
    if config.contains("외침거부 1") {
        return CommandResult::Error("☞ 외침거부중엔 외칠 수 없어요. ^^".to_string());
    }

    // Check if resting (운기조식)
    if player.act == crate::player::ActState::Rest {
        return CommandResult::Error("☞ 운기조식중에 외치게 되면 기가 흐트러집니다.".to_string());
    }

    let name = player.get_name();
    let personality = player.get_string("성격");

    // Determine shout type based on personality
    let shout_type = if personality == "선인" {
        "창룡후"
    } else if personality == "기인" {
        "사자후"
    } else {
        "외 침"
    };

    let admin_level = player.get_int("관리자등급");

    let shout_type = if admin_level >= 2000 {
        "\x1b[0;35m사자후\x1b[0;37m"
    } else if personality == "선인" {
        "\x1b[1;36m창룡후\x1b[0;37m"
    } else if personality == "기인" {
        "\x1b[1;32m사자후\x1b[0;37m"
    } else {
        "\x1b[32m외 침\x1b[37m"
    };

    let iga = hangul::han_iga(&name);

    let msg = format!("{}({}) : {}", format!("{}\x1b[33m{}\x1b[37m", name, iga), shout_type, message);

    CommandResult::Output(msg)
}

/// Whispers to a specific player (속삭여)
///
/// Not in original Python but common MUD feature
fn whisper_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::Error("☞ 사용법: 속삭여 [대상] [내용]".to_string());
    }

    let target = args[0];
    let message = args[1..].join(" ");
    let name = player.get_name();

    let output = format!("{} {}게 속삭입니다: '{}'", name, target, message);

    CommandResult::Output(output)
}

/// Emotes an action (표현)
///
/// Based on the emotion system in Python
fn emote_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 표현 [동작]".to_string());
    }

    let name = player.get_name();
    let action = args.join(" ");

    let output = format!("{} {}", name, action);

    CommandResult::Output(output)
}

/// Registers all communication commands
pub fn register_communication_commands(registry: &mut CommandRegistry) {
    // 말 (Say)
    registry.register(crate::command::registry::CommandInfo {
        name: "말".to_string(),
        aliases: vec!["say".to_string(), "'".to_string()],
        handler: Arc::new(say_command),
        level: 0,
        description: "주변 사람들에게 말합니다.".to_string(),
        usage: "말 [내용]".to_string(),
    });

    // 외쳐 (Shout)
    registry.register(crate::command::registry::CommandInfo {
        name: "외쳐".to_string(),
        aliases: vec!["외".to_string(), "외침".to_string(), "잡".to_string(),
                       "잡담".to_string(), ",".to_string(), "shout".to_string()],
        handler: Arc::new(shout_command),
        level: 0,
        description: "전체 서버에 외칩니다.".to_string(),
        usage: "외쳐 [내용]".to_string(),
    });

    // 속삭여 (Whisper)
    registry.register(crate::command::registry::CommandInfo {
        name: "속삭여".to_string(),
        aliases: vec!["속".to_string(), "whisper".to_string()],
        handler: Arc::new(whisper_command),
        level: 0,
        description: "특정 대상에게 귓속말을 합니다.".to_string(),
        usage: "속삭여 [대상] [내용]".to_string(),
    });

    // 표현 (Emote)
    registry.register(crate::command::registry::CommandInfo {
        name: "표현".to_string(),
        aliases: vec!["표".to_string(), "emote".to_string()],
        handler: Arc::new(emote_command),
        level: 0,
        description: "자유로운 동작을 표현합니다.".to_string(),
        usage: "표현 [동작]".to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandRegistry;

    fn create_test_player() -> Body {
        let mut player = Body::new();
        player.set("이름", "테스터");
        player.act = crate::player::ActState::Stand;
        player
    }

    #[test]
    fn test_register_communication_commands() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        assert!(registry.contains("말"));
        assert!(registry.contains("외쳐"));
        assert!(registry.contains("속삭여"));
        assert!(registry.contains("표현"));
    }

    #[test]
    fn test_say_command_no_args() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("말").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("Say What"));
        }
    }

    #[test]
    fn test_say_command_with_message() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("말").unwrap();

        let result = (cmd.handler)(&mut player, &["안녕하세요"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("안녕하세요"));
            assert!(msg.contains("말합니다"));
        }
    }

    #[test]
    fn test_shout_command_no_args() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("외쳐").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_shout_command_too_long() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("외쳐").unwrap();

        let long_message = "a".repeat(200);
        let result = (cmd.handler)(&mut player, &[&long_message]);
        assert!(matches!(result, CommandResult::Error(_)));

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("길어요"));
        }
    }

    #[test]
    fn test_shout_command_valid() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("외쳐").unwrap();

        let result = (cmd.handler)(&mut player, &["안녕하세요"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("안녕하세요"));
        }
    }

    #[test]
    fn test_shout_command_while_resting() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        player.act = crate::player::ActState::Rest;

        let cmd = registry.get("외쳐").unwrap();
        let result = (cmd.handler)(&mut player, &["안녕하세요"]);
        assert!(matches!(result, CommandResult::Error(_)));

        if let CommandResult::Error(msg) = result {
            assert!(msg.contains("운기조식"));
        }
    }

    #[test]
    fn test_whisper_command_no_args() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("속삭여").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_whisper_command_valid() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("속삭여").unwrap();

        let result = (cmd.handler)(&mut player, &["철수", "안녕"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("철수"));
            assert!(msg.contains("안녕"));
        }
    }

    #[test]
    fn test_emote_command_no_args() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("표현").unwrap();

        let result = (cmd.handler)(&mut player, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_emote_command_valid() {
        let mut registry = CommandRegistry::new();
        register_communication_commands(&mut registry);

        let mut player = create_test_player();
        let cmd = registry.get("표현").unwrap();

        let result = (cmd.handler)(&mut player, &["배고파요"]);
        assert!(matches!(result, CommandResult::Output(_)));

        if let CommandResult::Output(msg) = result {
            assert!(msg.contains("테스터"));
            assert!(msg.contains("배고파요"));
        }
    }
}
