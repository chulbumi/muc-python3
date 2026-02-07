//! Communication commands for MUD engine
//!
//! Handles player communication: 말 (say), 외쳐 (shout)

use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::Body;
use crate::hangul;
use std::sync::Arc;

/// 말 메시지 내 {밝},{빨} 등 ANSI 치환. 파이썬 cmds/말.py ANSI()
fn ansi_replace(msg: &str) -> String {
    const PAIRS: &[(&str, &str)] = &[
        ("{밝}", "\x1b[1m"), ("{어}", "\x1b[0m"), ("{검}", "\x1b[30m"), ("{빨}", "\x1b[31m"),
        ("{초}", "\x1b[32m"), ("{노}", "\x1b[33m"), ("{파}", "\x1b[34m"), ("{자}", "\x1b[35m"),
        ("{하}", "\x1b[36m"), ("{흰}", "\x1b[37m"),
        ("{배검}", "\x1b[40m"), ("{배빨}", "\x1b[41m"), ("{배초}", "\x1b[42m"), ("{배노}", "\x1b[43m"),
        ("{배파}", "\x1b[44m"), ("{배자}", "\x1b[45m"), ("{배하}", "\x1b[46m"), ("{배흰}", "\x1b[47m"),
    ];
    let mut s = msg.to_string();
    for (k, v) in PAIRS {
        s = s.replace(k, v);
    }
    s
}

/// Says something to the room (말)
///
/// Based on cmds/말.py: ANSI 치환 {밝},{빨} 등, 당신/방 메시지
fn say_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("\r\nSay What???".to_string());
    }

    let raw = args.join(" ");
    let m = ansi_replace(&raw);
    let name = player.get_name();

    // 발언자: "당신이 말합니다 : '...' \x1b[0;40;37m". 같은 방: "이름이/가 말합니다 : '...'" (파이썬 둘 다 ANSI 적용)
    let iga = hangul::han_iga(&name);
    let to_self = format!("당신이 말합니다 : '{}\x1b[0;40;37m'", m);
    let to_room = format!("{}\x1b[33m{}\x1b[37m 말합니다 : '{}'", name, iga, m);

    CommandResult::SayToRoom(to_self, to_room)
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

    let personality = player.get_string("성격");

    // Determine shout type based on personality
    let _shout_type = if personality == "선인" {
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

    // 이름은 포함, 조사(이/가)만 생략: 멍멍이(외침타입) : 메시지. 게임 접속 전체에 broadcast.
    let name = player.get_name();
    let msg = format!("{}({}) : {}", name, shout_type, message);

    CommandResult::Shout(msg)
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

/// 전음: 특정 대상에게 귓속말. 파이썬 cmds/전음.py
/// 사용법: [대상] [내용] 전음(/). 발신/수신 전음거부 체크, 대상 자신이면 "중얼 중얼".
fn tell_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::Error("☞ 사용법: [대상] [내용] 전음(/)".to_string());
    }
    let config = player.get_string("설정상태");
    if config.contains("전음거부 1") {
        return CommandResult::Error("☞ 전음 거부중이에요. ^^".to_string());
    }
    let target_name = args[0].to_string();
    let message = args[1..].join(" ");
    if target_name == player.get_name() {
        return CommandResult::Output("중얼 중얼 거립니다.".to_string());
    }
    CommandResult::Tell(target_name, message)
}

/// Emotes an action (표현)
///
/// Based on cmds/표현.py: 본인 "당신이 {line}", 방 "이름{이/가} {line}"
fn emote_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 표현 [동작] 또는 ' [동작]".to_string());
    }

    let name = player.get_name();
    let iga = hangul::han_iga(&name);
    let action = args.join(" ");
    let to_self = format!("당신이 {}", action);
    let to_room = format!("{}{} {}", name, iga, action);

    CommandResult::SayToRoom(to_self, to_room)
}

/// Registers all communication commands
pub fn register_communication_commands(registry: &mut CommandRegistry) {
    // 말 (Say). 파이썬 cmd.json: 말, .
    registry.register(crate::command::registry::CommandInfo {
        name: "말".to_string(),
        aliases: vec!["say".to_string(), "'".to_string(), ".".to_string()],
        handler: Arc::new(say_command),
        level: 0,
        description: "주변 사람들에게 말합니다.".to_string(),
        usage: "말 [내용]".to_string(),
    });

    // 외쳐: Rhai 전환 (cmds/외쳐.rhai)

    // 속삭여 (Whisper)
    registry.register(crate::command::registry::CommandInfo {
        name: "속삭여".to_string(),
        aliases: vec!["속".to_string(), "whisper".to_string()],
        handler: Arc::new(whisper_command),
        level: 0,
        description: "특정 대상에게 귓속말을 합니다.".to_string(),
        usage: "속삭여 [대상] [내용]".to_string(),
    });

    // 전음: Rhai 전환 (cmds/전음.rhai)

    // 표현: Rhai 전환 (cmds/표현.rhai)
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
        // 외쳐, 전음, 표현: Rhai 전환 (register_script_commands에서 등록)
        assert!(registry.contains("속삭여"));
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
        assert!(matches!(result, CommandResult::SayToRoom(_, _)));

        if let CommandResult::SayToRoom(to_self, to_room) = result {
            assert!(to_self.contains("당신이 말합니다"));
            assert!(to_self.contains("안녕하세요"));
            assert!(to_room.contains("말합니다"));
            assert!(to_room.contains("안녕하세요"));
        }
    }

    // 외쳐: Rhai 전환 (cmds/외쳐.rhai). Rust handler 테스트 제거.

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

    // 전음: Rhai 전환 (cmds/전음.rhai). 표현: Rhai 전환 (cmds/표현.rhai). Rust handler 테스트 제거.
}
