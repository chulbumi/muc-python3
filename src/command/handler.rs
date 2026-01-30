//! Command handler for MUD engine
//!
//! Provides the core command execution logic and result types.

use std::sync::Arc;
use crate::player::Body;
use crate::command::registry::CommandRegistry;

/// Multi-step input state (e.g. 암호변경: 이전암호 → 새암호 → 확인)
#[derive(Debug, Clone, PartialEq)]
pub enum PendingInput {
    ChangePasswordOld,
    ChangePasswordNew,
    ChangePasswordConfirm { new_password: String },
    /// $엔터$: 다음 입력 시 try_mob_event_resume( mob_key, event_key, words, line_num, resume_func ) 호출.
    /// resume_func: Rhai wait_enter 재개용. Some("step1")이면 do_event_rhai에서 step1() 호출. Legacy면 None.
    EventEnter {
        mob_key: String,
        event_key: String,
        words: Vec<String>,
        line_num: usize,
        resume_func: Option<String>,
    },
    /// $스크립트호출 무기강화 등: 다음 입력 시 run_script_chunk 또는 run_script_chunk_rhai.
    Script {
        name: String,
        lines: Vec<String>,
        line_num: usize,
        temp_input: Option<String>,
        from_confirm: bool,
        /// Rhai용 ob. None이면 legacy.
        script_ob: Option<std::collections::HashMap<String, String>>,
        /// Rhai용 재개 op. None이면 legacy.
        script_resume_op: Option<String>,
    },
    /// 쪽지 편집: 라인 누적, '.' 또는 10줄 시 저장 후 종료.
    NoteEdit {
        target_name: String,
        title: String,
        lines: Vec<String>,
    },
}

/// Result type for command execution
#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    /// Command executed successfully
    Ok,
    /// Command execution failed with error message
    Error(String),
    /// Command needs more arguments
    Usage(String),
    /// Player should be shown something (output message)
    Output(String),
    /// Command executed but should not show prompt
    NoPrompt,
    /// Movement command (direction)
    Move(String),
    /// Combat action
    Combat,
    /// 말(say): (발언자에게 to_self, 같은 방 다른 플레이어에게 to_room) 전송.
    SayToRoom(String, String),
    /// 외쳐(shout): 게임 접속 전체에 broadcast. 이름 없이 (외침타입) : 메시지. 외침거부 체크는 broadcast 쪽에서.
    Shout(String),
    /// 공지(notice): 게임 접속 전체에 전송. 외침거부와 무관하게 전원에게. [공지] 이름 : 메시지.
    Notice(String),
    /// 전음: 특정 대상에게 귓속말. (대상이름, 메시지). 전음거부·대상 검증은 handle에서.
    Tell(String, String),
    /// 셧다운: 서버 종료 요청. 전체 사용자에게 알리고 종료 시퀀스.
    Shutdown,
    /// 감정표현: (to_self, to_room, to_target). to_target=(대상이름, buf2)이면 방 전송 시 대상 제외 후 대상에게 buf2 전송.
    EmotionToRoom(String, String, Option<(String, String)>),
    /// 다음 입력 대기 (암호변경 등). prompt 전송 후 pending_input 설정.
    RequestInput { prompt: String, state: PendingInput },
    /// 주다: 플레이어에게 은전/금전/아이템 전달. handle에서 대상 조회·이전·메시지 전송.
    GiveToPlayer {
        target_name: String,
        giver_name: String,
        give_silver: Option<i64>,
        give_gold: Option<i64>,
        give_item: Option<(String, usize, usize)>, // (item_name, order, count) 비스택
        give_item_stack: Option<(String, i64)>,    // (인덱스, count) 스택
    },
    /// 방파/길드 말: 특정 플레이어 이름 목록에게 메시지 전송. [방파] 발신자 : 메시지.
    BroadcastToPlayers(Vec<String>, String),
    /// 스크립트 send_to_user에서 수집한 (이름, 메시지) 목록. handler에서 각자에게 전송.
    SendToUsers(Vec<(String, String)>),
    /// NPC 이벤트 스크립트 결과: 출력 라인들, $위치이동 시 (zone, room). 클라이언트에서 set_position 적용·방 검증.
    MobEvent {
        output_lines: Vec<String>,
        set_position: Option<(String, String)>,
    },
    /// $엔터$: 출력·이동 적용 후 엔터 대기. prompt 전송, PendingInput::EventEnter { ... }.
    /// resume_func: Rhai wait_enter(next_func, prompt) 시 Some(next_func). Legacy면 None.
    MobEventEnter {
        output_lines: Vec<String>,
        set_position: Option<(String, String)>,
        mob_key: String,
        event_key: String,
        words: Vec<String>,
        line_num: usize,
        prompt: String,
        resume_func: Option<String>,
    },
    /// $스크립트호출: data/script/ 무기강화 등. use_rhai면 run_script_chunk_rhai, 아니면 run_script_chunk.
    StartScript { script_name: String, lines: Vec<String>, use_rhai: bool },
    /// 쪽지: [이름] [제목] 후 편집 모드. 라인 단위 입력, '.' 또는 10줄이면 종료.
    StartNoteEdit { target_name: String, title: String },
    /// 강제로그아웃: 관리자가 플레이어를 강제로 로그아웃시킴.
    Kick { target_name: String, reason: String },
    /// 계정정지: 관리자가 플레이어를 일정 기간 동안 접속 차단. duration은 초 단위.
    Ban { target_name: String, duration: i64, reason: String },
}

impl CommandResult {
    /// Returns true if the command succeeded
    pub fn is_ok(&self) -> bool {
        matches!(self, CommandResult::Ok | CommandResult::Output(_) | CommandResult::Move(_) | CommandResult::Combat | CommandResult::NoPrompt | CommandResult::SayToRoom(_, _) | CommandResult::Shout(_) | CommandResult::Notice(_) | CommandResult::Tell(_, _) | CommandResult::Shutdown | CommandResult::EmotionToRoom(_, _, _) | CommandResult::RequestInput { .. } | CommandResult::GiveToPlayer { .. } | CommandResult::BroadcastToPlayers(_, _) | CommandResult::SendToUsers(_) | CommandResult::MobEvent { .. } | CommandResult::MobEventEnter { .. } | CommandResult::StartScript { .. } | CommandResult::StartNoteEdit { .. } | CommandResult::Kick { .. } | CommandResult::Ban { .. })
    }

    /// Returns true if the command should skip the prompt
    pub fn no_prompt(&self) -> bool {
        matches!(self, CommandResult::NoPrompt)
    }

    /// Gets the error message if this is an error result
    pub fn error_message(&self) -> Option<&str> {
        match self {
            CommandResult::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Context for command execution
pub struct CommandContext<'a> {
    /// Reference to the command registry
    pub registry: &'a CommandRegistry,
    /// Output messages to send to the player
    pub outputs: Vec<String>,
    /// Whether the command was handled
    pub handled: bool,
}

impl<'a> CommandContext<'a> {
    /// Creates a new CommandContext
    pub fn new(registry: &'a CommandRegistry) -> Self {
        CommandContext {
            registry,
            outputs: Vec::new(),
            handled: false,
        }
    }

    /// Adds an output message
    pub fn send(&mut self, message: String) {
        self.outputs.push(message);
    }

    /// Marks the command as handled
    pub fn mark_handled(&mut self) {
        self.handled = true;
    }
}

/// Command handler for executing commands
pub struct CommandHandler {
    registry: Arc<CommandRegistry>,
}

impl CommandHandler {
    /// Creates a new CommandHandler with the given registry
    pub fn new(registry: Arc<CommandRegistry>) -> Self {
        CommandHandler { registry }
    }

    /// Handles a command from a player
    ///
    /// # Arguments
    /// * `player` - The player executing the command
    /// * `command` - The command name
    /// * `args` - The command arguments
    ///
    /// # Returns
    /// CommandResult indicating the outcome
    pub fn handle_command(
        &self,
        player: &mut Body,
        command: &str,
        args: &[&str],
    ) -> CommandResult {
        // Resolve alias
        let resolved = self.registry.resolve_alias(command);

        // Find the command
        if let Some(cmd_info) = self.registry.get(&resolved) {
            // Check permission
            let player_level = player.get_int("관리자등급");

            if player_level < cmd_info.level as i64 {
                return CommandResult::Error("권한이 없습니다.".to_string());
            }

            // Execute the command
            let handler = cmd_info.handler.clone();
            handler(player, args)
        } else {
            CommandResult::Error("무슨 말인지 모르겠어요. *^_^*".to_string())
        }
    }

    /// Checks if a direction is valid for movement
    ///
    /// # Arguments
    /// * `direction` - The direction to check
    ///
    /// # Returns
    /// true if the direction is valid
    pub fn is_valid_direction(&self, direction: &str) -> bool {
        const VALID_DIRECTIONS: &[&str] = &["북", "남", "동", "서", "위", "아래"];
        VALID_DIRECTIONS.contains(&direction)
    }

    /// Returns help text for a command
    ///
    /// # Arguments
    /// * `command` - The command name
    ///
    /// # Returns
    /// Option<String> with the help text
    pub fn get_help(&self, command: &str) -> Option<String> {
        let resolved = self.registry.resolve_alias(command);
        self.registry.get(&resolved).map(|cmd| {
            format!(
                "{}\n사용법: {}",
                cmd.description, cmd.usage
            )
        })
    }

    /// Returns all available commands for a player level
    ///
    /// # Arguments
    /// * `player_level` - The player's permission level
    ///
    /// # Returns
    /// Vec of command names the player can use
    pub fn available_commands(&self, player_level: i32) -> Vec<String> {
        self.registry.all_commands()
            .iter()
            .filter(|cmd| cmd.level <= player_level)
            .map(|cmd| cmd.name.clone())
            .collect()
    }
}

/// Default message strings used by commands
pub mod messages {
    /// Default "unknown command" message
    pub const UNKNOWN_COMMAND: &str = "무슨 말인지 모르겠어요. *^_^*";

    /// Default "no arguments" message
    pub const NO_ARGS: &str = "사용법: ";

    /// "What?" message
    pub const SAY_WHAT: &str = "Say What???";

    /// "Too long" message for shouts
    pub const TOO_LONG: &str = "너무 길어요. ^^";

    /// "Cannot move during combat" message
    pub const CANNOT_MOVE_COMBAT: &str = "전투 중에는 이동 할 수 없습니다.";

    /// "Cannot flee when not fighting" message
    pub const CANNOT_FLEE: &str = "무림인은 아무때나 도망가는것이 아니라네";

    /// "Flee failed" message
    pub const FLEE_FAILED: &str = "도망 갈려다 잡혔어요. '흑흑~~ T_T'";
}

/// Helper functions for command handlers
pub mod helpers {
    use crate::player::Body;

    /// Checks if a player is in combat
    pub fn is_in_combat(player: &Body) -> bool {
        player.act == crate::player::ActState::Fight
    }

    /// Gets a player's level safely
    pub fn get_player_level(player: &Body) -> i64 {
        player.get_int("레벨").max(1)
    }

    /// Gets a player's admin level safely
    pub fn get_admin_level(player: &Body) -> i64 {
        player.get_int("관리자등급")
    }

    /// Gets a player's HP
    pub fn get_hp(player: &Body) -> i64 {
        player.get_hp()
    }

    /// Gets a player's max HP
    pub fn get_max_hp(player: &Body) -> i64 {
        player.get_max_hp()
    }

    /// Gets a player's MP
    pub fn get_mp(player: &Body) -> i64 {
        player.get_mp()
    }

    /// Gets a player's max MP
    pub fn get_max_mp(player: &Body) -> i64 {
        player.get_max_mp()
    }

    /// Formats a name with Korean particle (이/가)
    pub fn format_iga(name: &str) -> String {
        use crate::hangul;
        format!("{}{}", name, hangul::han_iga(name))
    }

    /// Formats a name with Korean particle (을/를)
    pub fn format_obj(name: &str) -> String {
        use crate::hangul;
        format!("{}{}", name, hangul::han_obj(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandRegistry;

    fn create_test_registry() -> Arc<CommandRegistry> {
        let mut registry = CommandRegistry::new();

        // Register test commands
        registry.register_simple("test", |_, args| {
            if args.is_empty() {
                CommandResult::Usage("test <args>".to_string())
            } else {
                CommandResult::Ok
            }
        }, "Test command");

        registry.register_simple("admin", |_, _| CommandResult::Ok, "Admin command");
        registry.get_mut("admin").unwrap().level = 100;

        Arc::new(registry)
    }

    fn create_test_player() -> Body {
        let mut player = Body::new();
        player.set("이름", "테스터");
        player.set("레벨", 10i64);
        player.set("관리자등급", 0i64);
        player
    }

    #[test]
    fn test_command_result_is_ok() {
        assert!(CommandResult::Ok.is_ok());
        assert!(CommandResult::Output("test".to_string()).is_ok());
        assert!(CommandResult::Move("북".to_string()).is_ok());
        assert!(!CommandResult::Error("error".to_string()).is_ok());
    }

    #[test]
    fn test_command_result_no_prompt() {
        assert!(CommandResult::NoPrompt.no_prompt());
        assert!(!CommandResult::Ok.no_prompt());
        assert!(!CommandResult::Output("test".to_string()).no_prompt());
    }

    #[test]
    fn test_command_result_error_message() {
        let result = CommandResult::Error("test error".to_string());
        assert_eq!(result.error_message(), Some("test error"));

        let result = CommandResult::Ok;
        assert_eq!(result.error_message(), None);
    }

    #[test]
    fn test_handle_command_success() {
        let registry = create_test_registry();
        let handler = CommandHandler::new(registry.clone());
        let mut player = create_test_player();

        let result = handler.handle_command(&mut player, "test", &["arg1"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_command_no_args() {
        let registry = create_test_registry();
        let handler = CommandHandler::new(registry);
        let mut player = create_test_player();

        let result = handler.handle_command(&mut player, "test", &[]);
        assert!(!result.is_ok());
    }

    #[test]
    fn test_handle_command_not_found() {
        let registry = create_test_registry();
        let handler = CommandHandler::new(registry);
        let mut player = create_test_player();

        let result = handler.handle_command(&mut player, "nonexistent", &[]);
        assert!(!result.is_ok());
    }

    #[test]
    fn test_handle_command_permission_denied() {
        let registry = create_test_registry();
        let handler = CommandHandler::new(registry);
        let mut player = create_test_player();

        let result = handler.handle_command(&mut player, "admin", &[]);
        assert!(!result.is_ok());
    }

    #[test]
    fn test_is_valid_direction() {
        let registry = create_test_registry();
        let handler = CommandHandler::new(registry);

        assert!(handler.is_valid_direction("북"));
        assert!(handler.is_valid_direction("남"));
        assert!(handler.is_valid_direction("동"));
        assert!(handler.is_valid_direction("서"));
        assert!(handler.is_valid_direction("위"));
        assert!(handler.is_valid_direction("아래"));
        assert!(!handler.is_valid_direction("대각선"));
    }

    #[test]
    fn test_get_help() {
        let registry = create_test_registry();
        let handler = CommandHandler::new(registry);

        let help = handler.get_help("test");
        assert!(help.is_some());
        assert!(help.unwrap().contains("Test command"));
    }

    #[test]
    fn test_available_commands() {
        let registry = create_test_registry();
        let handler = CommandHandler::new(registry);

        let cmds = handler.available_commands(0);
        assert!(cmds.contains(&"test".to_string()));
        assert!(!cmds.contains(&"admin".to_string()));

        let cmds = handler.available_commands(100);
        assert!(cmds.contains(&"test".to_string()));
        assert!(cmds.contains(&"admin".to_string()));
    }

    #[test]
    fn test_command_context() {
        let registry = create_test_registry();
        let mut ctx = CommandContext::new(&registry);

        assert!(!ctx.handled);
        assert_eq!(ctx.outputs.len(), 0);

        ctx.send("Test message".to_string());
        ctx.mark_handled();

        assert!(ctx.handled);
        assert_eq!(ctx.outputs.len(), 1);
        assert_eq!(ctx.outputs[0], "Test message");
    }

    #[test]
    fn test_helpers_get_player_level() {
        let player = create_test_player();
        assert_eq!(helpers::get_player_level(&player), 10);
    }

    #[test]
    fn test_helpers_get_admin_level() {
        let player = create_test_player();
        assert_eq!(helpers::get_admin_level(&player), 0);
    }

    #[test]
    fn test_helpers_format_iga() {
        assert_eq!(helpers::format_iga("검"), "검이");
        assert_eq!(helpers::format_iga("사과"), "사과가");
    }

    #[test]
    fn test_helpers_format_obj() {
        assert_eq!(helpers::format_obj("검"), "검을");
        assert_eq!(helpers::format_obj("사과"), "사과를");
    }
}
