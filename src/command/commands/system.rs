//! 시스템/관리 명령 (셧다운 등)
//!
//! 서버 종료 등 관리자 전용 명령. 파이썬 cmds/리부팅.py, 셧다운 참조.

use std::sync::Arc;
use crate::command::CommandResult;
use crate::command::registry::CommandRegistry;
use crate::player::Body;

/// 셧다운: 서버 종료. 관리자등급 1000 이상.
/// 전체 사용자에게 종료 안내 → 정리 후 종료. (파이썬 셧다운/리부팅 참조)
fn shutdown_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    if player.get_int("관리자등급") < 1000 {
        return CommandResult::Error("☞ 무슨 말인지 모르겠어요. *^_^*".to_string());
    }
    CommandResult::Shutdown
}

/// 시스템 명령 등록
pub fn register_system_commands(registry: &mut CommandRegistry) {
    registry.register(crate::command::registry::CommandInfo {
        name: "셧다운".to_string(),
        aliases: vec!["shutdown".to_string()],
        handler: Arc::new(shutdown_command),
        level: 1000,
        description: "서버를 종료합니다. 전체 사용자에게 알린 뒤 종료. (관리자 1000 이상)".to_string(),
        usage: "셧다운".to_string(),
    });
}
