//! 업데이트(인 업데이트) 명령
//!
//! JSON·설정 등을 게임 내에서 다시 로드. 관리자등급 1000 이상.
//! 파이썬 cmds/업데이트.py 참조. 현재 표현(emotion.json), 도우미(doumi.json) 지원.

use std::sync::Arc;
use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::Body;

fn update_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if player.get_int("관리자등급") < 1000 {
        return CommandResult::Error("☞ 무슨 말인지 모르겠어요. *^_^*".to_string());
    }
    let sub = args.first().copied().unwrap_or("").trim();
    if sub.is_empty() {
        return CommandResult::Output(
            "* 명령어, 무림별호, 도움말, 무공, 표현, 도우미, 메인설정, 스크립트 중에 선택하세요".to_string(),
        );
    }
    match sub {
        "표현" => match crate::emotion::reload_emotion_map() {
            Ok(()) => CommandResult::Output("* 표현이 업데이트 되었습니다.".to_string()),
            Err(e) => CommandResult::Error(format!("emotion.json 재로딩 실패: {}", e)),
        },
        "도우미" => match crate::network::client::reload_doumi_json() {
            Ok(()) => CommandResult::Output("* 도우미가 업데이트 되었습니다.".to_string()),
            Err(e) => CommandResult::Error(e),
        },
        _ => CommandResult::Output(
            "* 해당 항목은 아직 지원하지 않습니다. (표현, 도우미)".to_string(),
        ),
    }
}

/// 업데이트 명령 등록 (alias: 업)
pub fn register_update_commands(registry: &mut CommandRegistry) {
    registry.register(crate::command::registry::CommandInfo {
        name: "업데이트".to_string(),
        aliases: vec!["업".to_string()],
        handler: Arc::new(update_command),
        level: 1000,
        description: "JSON·설정 재로딩 (표현, 도우미 등). 관리자 1000 이상.".to_string(),
        usage: "업데이트 [표현|도우미]".to_string(),
    });
}
