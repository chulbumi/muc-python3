//! Script command execution
//!
//! This module provides command execution through Rhai scripts.

use crate::command::registry::{CommandInfo, CommandRegistry};
use crate::command::{CommandFn, CommandResult};
use crate::player::{body::SendLine, Body};
use crate::scheduler::CallOutScheduler;
use crate::script::ScriptStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Type alias for player description callback
pub type PlayerDescFn = Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>;

/// Type alias for player map callback
pub type PlayerMapFn = Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>;

/// Rhai sources loaded by [`ScriptStorage`] that are not player commands.
///
/// `comm.py` is a Python helper module rather than a `CmdObj`, and its Rhai
/// counterpart must not become a command merely because it lives in `cmds/`.
/// The remaining entries are engine support or development/test scripts.
const NON_COMMAND_SCRIPTS: &[&str] = &[
    // Python Player.parse_command owns one-word exits/directions.  The Rhai
    // implementation is registered through CommandRegistry's private hook,
    // never as a player-visible command.
    "__movement",
    "__summon_move",
    "__death",
    "__combat_tick",
    "attack",
    "comm",
    "debug_test",
    "help",
    "inventory",
    "look",
    "say",
    "test",
    "test_output",
    "test_simple",
    "test_syntax",
    "디버그",
    "테스트명령",
    "master",
    // Python has cmds/줘.py, not cmds/주다.py. The duplicate Rhai source must
    // not create an additional command or shadow the exact `줘` command.
    "주다",
];

/// Command permission metadata copied from the authoritative Python
/// `cmds/<name>.py` `CmdObj.level` class attributes.
///
/// Python commands without an explicit `CmdObj.level` are public and retain
/// level 0. Keep this table in sync with the Python metadata while it remains
/// the compatibility reference; do not infer levels from command names or
/// from incidental checks inside a Rhai implementation.
const PYTHON_COMMAND_LEVELS: &[(&str, i32)] = &[
    // Python CmdObj.level = 1000
    ("기연", 1000),
    ("기연리스트", 1000),
    ("누가주나", 1000),
    ("리부팅", 1000),
    ("리젠", 1000),
    ("명령어리스트", 1000),
    ("몹찾기", 1000),
    ("몹회복", 1000),
    ("무공리스트", 1000),
    ("무공제거", 1000),
    ("방어구찾기", 1000),
    ("사용자몹소환", 1000),
    ("사용자몹제거", 1000),
    ("사용자몹제거1", 1000),
    ("소켓", 1000),
    ("속성", 1000),
    ("순위", 1000),
    ("아이템찾기", 1000),
    ("앞", 1000),
    ("업데이트", 1000),
    ("올숙리스트", 1000),
    ("이동", 1000),
    ("이동이동", 1000),
    ("이벤트", 1000),
    ("이벤트삭제", 1000),
    ("이벤트설정", 1000),
    ("정렬", 1000),
    ("정리", 1000),
    ("투명", 1000),
    ("회복", 1000),
    // Python CmdObj.level = 2000
    ("값값", 2000),
    ("값삭제", 2000),
    ("값설정", 2000),
    ("기연삭제", 2000),
    ("기연삭제1", 2000),
    ("기연정리", 2000),
    ("기연정리리", 2000),
    ("기연초기화", 2000),
    ("맵", 2000),
    ("명령", 2000),
    ("모두끝", 2000),
    ("모두소환", 2000),
    ("모두저장", 2000),
    ("몹삭제", 2000),
    ("몹생성", 2000),
    ("몹제거", 2000),
    ("무공전수", 2000),
    ("무공전수2", 2000),
    ("방설명", 2000),
    ("방이름", 2000),
    ("방제거", 2000),
    ("방파방설명", 2000),
    ("방파초기화", 2000),
    ("생성", 2000),
    ("성올려", 2000),
    ("소환", 2000),
    ("속성제거", 2000),
    ("속성추가", 2000),
    ("순위초기화", 2000),
    ("아이템삭제", 2000),
    ("앞앞", 2000),
    ("오브젝트저장", 2000),
    ("옵랜덤", 2000),
    ("옵설정", 2000),
    ("이동동", 2000),
    ("제이슨", 2000),
    ("죽여", 2000),
    ("줘줘", 2000),
    ("찾아라", 2000),
    ("청소", 2000),
    ("체인지", 2000),
    ("특정방파초기화", 2000),
];

fn is_player_command_script(script_name: &str) -> bool {
    !NON_COMMAND_SCRIPTS.contains(&script_name)
}

fn python_command_level(script_name: &str) -> i32 {
    PYTHON_COMMAND_LEVELS
        .iter()
        .find_map(|(name, level)| (*name == script_name).then_some(*level))
        .unwrap_or(0)
}

/// Execute a script command
pub async fn execute_script_command(
    script_storage: &ScriptStorage,
    player: &mut Body,
    script_name: &str,
    line: &str,
    get_other_players_desc: Option<PlayerDescFn>,
    get_other_players_map: Option<PlayerMapFn>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
) -> CommandResult {
    match script_storage.execute(
        script_name,
        player,
        line,
        get_other_players_desc,
        get_other_players_map,
        call_out_scheduler,
    ) {
        Ok((_outputs, special)) => {
            if let Some(cr) = special {
                cr
            } else {
                CommandResult::Ok
            }
        }
        Err(e) => {
            // Return error message
            let msg = format!("스크립트 실행 오류: {}", e);
            player.send_line(&msg);
            CommandResult::Error(msg)
        }
    }
}

/// Create a command function that executes a script.
/// get_other_players_desc: 봐(view_map_data) 시 같은 방 다른 유저 getDesc. None이면 빈 목록.
/// get_other_players_map: 봐 find_target에서 같은 방 다른 유저 (이름→getDesc) 맵. None이면 빈 맵.
/// call_out_scheduler: Some이면 call_out/call_later 사용 가능.
pub fn create_script_command(
    script_storage: Arc<RwLock<ScriptStorage>>,
    script_name: String,
    get_other_players_desc: Option<PlayerDescFn>,
    get_other_players_map: Option<PlayerMapFn>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
) -> CommandFn {
    Arc::new(move |player: &mut Body, args: &[&str]| -> CommandResult {
        tracing::debug!(script = %script_name, ?args, "Executing script command");
        let line = args.join(" ");
        player
            .temp_mut()
            .remove(crate::command::commands::update::UPDATE_COMMAND_REQUEST);
        let execution = {
            let storage = script_storage.try_read();
            let storage = match storage {
                Ok(s) => s,
                Err(_) => {
                    let msg = "Script storage unavailable".to_string();
                    return CommandResult::Error(msg);
                }
            };
            storage.execute(
                &script_name,
                player,
                &line,
                get_other_players_desc.clone(),
                get_other_players_map.clone(),
                call_out_scheduler.clone(),
            )
        };

        let command_reload_requested = matches!(
            player
                .temp_mut()
                .remove(crate::command::commands::update::UPDATE_COMMAND_REQUEST),
            Some(crate::object::Value::String(target)) if target == "명령어"
        );
        if command_reload_requested {
            let mut storage = match script_storage.try_write() {
                Ok(storage) => storage,
                Err(error) => {
                    tracing::error!(%error, "Python-compatible command reload lock failed");
                    return CommandResult::Ok;
                }
            };
            if let Err(error) = crate::command::commands::update::apply_command_reload(&mut storage)
            {
                tracing::error!(%error, "Python-compatible command reload failed");
                return CommandResult::Ok;
            }
        }

        match execution {
            Ok((outputs, special)) => {
                tracing::debug!(
                    script = %script_name,
                    outputs = outputs.len(),
                    ?special,
                    "Script command executed"
                );

                // Handle special actions first
                if let Some(cr) = special {
                    // Send outputs before handling special action
                    if !outputs.is_empty() {
                        player.send_line(&outputs.join("\r\n"));
                    }
                    return cr;
                }

                // No special action, return outputs or Ok
                if outputs.is_empty() {
                    CommandResult::Ok
                } else {
                    CommandResult::Output(outputs.join("\r\n"))
                }
            }
            Err(e) => {
                let msg = format!("오류: {}", e);
                CommandResult::Error(msg)
            }
        }
    })
}

/// Register script-based commands from the cmds/ directory
///
/// Only registers commands that don't already exist in the registry.
/// Built-in commands take priority over script commands.
/// get_other_players_desc: 봐 시 같은 방 다른 유저 getDesc. None이면 봐에서도 빈 목록.
/// call_out_scheduler: Some이면 call_out/call_later 사용 가능.
pub async fn register_script_commands(
    registry: &mut CommandRegistry,
    script_storage: Arc<RwLock<ScriptStorage>>,
    get_other_players_desc: Option<PlayerDescFn>,
    get_other_players_map: Option<PlayerMapFn>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
) {
    let scripts = script_storage.read().await;
    let script_names = scripts.script_names();
    drop(scripts);

    println!(
        "[SCRIPT_CMD] Found {} scripts to register",
        script_names.len()
    );

    if script_names.iter().any(|name| name == "__movement") {
        registry.register_internal(
            "movement",
            create_script_command(
                script_storage.clone(),
                "__movement".to_string(),
                get_other_players_desc.clone(),
                get_other_players_map.clone(),
                call_out_scheduler.clone(),
            ),
        );
    }
    if script_names.iter().any(|name| name == "__death") {
        registry.register_internal(
            "death",
            create_script_command(
                script_storage.clone(),
                "__death".to_string(),
                get_other_players_desc.clone(),
                get_other_players_map.clone(),
                call_out_scheduler.clone(),
            ),
        );
    }
    if script_names.iter().any(|name| name == "__combat_tick") {
        registry.register_internal(
            "combat_tick",
            create_script_command(
                script_storage.clone(),
                "__combat_tick".to_string(),
                get_other_players_desc.clone(),
                get_other_players_map.clone(),
                call_out_scheduler.clone(),
            ),
        );
    }
    if script_names.iter().any(|name| name == "__summon_move") {
        registry.register_internal(
            "summon_move",
            create_script_command(
                script_storage.clone(),
                "__summon_move".to_string(),
                get_other_players_desc.clone(),
                get_other_players_map.clone(),
                call_out_scheduler.clone(),
            ),
        );
    }

    // Collect existing command aliases once to avoid O(n*m) complexity
    let existing_aliases: std::collections::HashSet<String> = registry
        .all_commands()
        .iter()
        .flat_map(|cmd| {
            let mut aliases = cmd.aliases.clone();
            aliases.push(cmd.name.clone());
            aliases
        })
        .collect();

    for script_name in script_names {
        if !is_player_command_script(&script_name) {
            info!(
                "[SCRIPT_CMD] Skipping {} (not a player command)",
                script_name
            );
            continue;
        }

        // Skip if command already exists (built-in commands take priority)
        if registry.contains(&script_name) {
            info!(
                "[SCRIPT_CMD] Skipping {} (already registered as built-in)",
                script_name
            );
            continue;
        }

        // Also check if any existing command has this as an alias
        if existing_aliases.contains(&script_name) {
            info!(
                "[SCRIPT_CMD] Skipping {} (alias of existing command)",
                script_name
            );
            continue;
        }

        let storage = script_storage.clone();
        let name_clone = script_name.clone();

        // Create command from script
        let command_fn = create_script_command(
            storage,
            script_name,
            get_other_players_desc.clone(),
            get_other_players_map.clone(),
            call_out_scheduler.clone(),
        );

        // Get description from script if available
        let description = format!("{} 명령어", name_clone);
        let usage = description.clone();

        // Create CommandInfo
        let info = CommandInfo::new(
            name_clone.clone(),
            Vec::new(),
            command_fn,
            python_command_level(&name_clone),
            description.clone(),
            usage,
        );

        // Register the command
        registry.register(info);
        info!("[SCRIPT_CMD] Registered command: {}", name_clone);
    }

    println!("[SCRIPT_CMD] Script registration complete");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptConfig;
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};

    /// Read only the class-level `CmdObj.level` assignment used by the Python
    /// command loader. Four leading spaces distinguish it from local variables
    /// inside `cmd` methods.
    fn cmdobj_level(source: &str) -> Option<i32> {
        source.lines().find_map(|line| {
            let assignment = line.strip_prefix("    level")?.trim_start();
            let value = assignment.strip_prefix('=')?.trim();
            value.parse().ok()
        })
    }

    fn python_cmdobj_levels(command_dir: &Path) -> HashMap<String, i32> {
        std::fs::read_dir(command_dir)
            .expect("cmds directory must be readable")
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
                    return None;
                }
                let source = std::fs::read_to_string(&path).ok()?;
                let level = cmdobj_level(&source)?;
                let name = path.file_stem()?.to_str()?.to_string();
                Some((name, level))
            })
            .collect()
    }

    fn repository_script_config() -> ScriptConfig {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        ScriptConfig {
            script_dir: root.join("cmds"),
            hot_reload: false,
            extension: ".rhai".to_string(),
            data_dir: root.join("data/config"),
            // Command regression tests execute the same shared Rhai helpers
            // as production (death, entry combat, formatting).
            lib_dir: root.join("lib"),
        }
    }

    #[test]
    fn non_command_sources_are_explicitly_filtered() {
        for name in NON_COMMAND_SCRIPTS {
            assert!(!is_player_command_script(name), "{name} must stay hidden");
        }

        for name in ["봐", "점수", "줘", "이벤트설정"] {
            assert!(
                is_player_command_script(name),
                "normal command {name} must remain exposed"
            );
        }
    }

    #[test]
    fn permission_table_exactly_matches_python_cmdobj_metadata() {
        let command_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cmds");
        let actual = python_cmdobj_levels(&command_dir);
        let expected: HashMap<String, i32> = PYTHON_COMMAND_LEVELS
            .iter()
            .map(|(name, level)| ((*name).to_string(), *level))
            .collect();

        assert_eq!(actual, expected);
        assert_eq!(
            expected.values().filter(|level| **level == 1000).count(),
            30
        );
        assert_eq!(
            expected.values().filter(|level| **level == 2000).count(),
            42
        );
    }

    #[test]
    fn unguarded_admin_migrations_have_registry_permissions() {
        let commands = [
            ("값값", 2000),
            ("무공전수2", 2000),
            ("무공제거", 1000),
            ("방설명", 2000),
            ("방이름", 2000),
            ("방파방설명", 2000),
            ("소켓", 1000),
            ("속성", 1000),
            ("속성제거", 2000),
            ("속성추가", 2000),
            ("아이템삭제", 2000),
            ("아이템찾기", 1000),
            ("업데이트", 1000),
            ("옵랜덤", 2000),
            ("옵설정", 2000),
            ("정렬", 1000),
            ("줘줘", 2000),
            ("찾아라", 2000),
            ("체인지", 2000),
        ];

        for (name, expected_level) in commands {
            assert_eq!(python_command_level(name), expected_level, "{name}");
        }
        assert_eq!(python_command_level("점수"), 0);
    }

    #[tokio::test]
    async fn registration_filters_sources_and_applies_python_levels() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();

        register_script_commands(&mut registry, storage, None, None, None).await;

        let registered_names = registry.command_names();
        for name in NON_COMMAND_SCRIPTS {
            assert!(
                !registered_names.iter().any(|registered| registered == name),
                "{name} must not be registered"
            );
        }

        for (name, expected_level) in PYTHON_COMMAND_LEVELS {
            let command = registry
                .get(name)
                .unwrap_or_else(|| panic!("Python command {name} must be registered"));
            assert_eq!(command.level, *expected_level, "{name}");
        }
        assert_eq!(registry.get("점수").unwrap().level, 0);

        assert_eq!(registry.get("/").unwrap().name, "전음");
        for invented in [
            "give",
            "주",
            "선물",
            "선",
            "shout",
            "창",
            "창룡",
            "창룡후",
            "외친다",
            "emote",
            "무공시전",
            ".",
            "메모보냄",
        ] {
            assert!(registry.get(invented).is_none(), "{invented}");
        }
        assert_eq!(registry.get("줘").unwrap().name, "줘");
        assert!(registry.get("주다").is_none());

        let movement = registry
            .get_internal("movement")
            .expect("private movement hook must be registered");
        assert!(registry.get("__movement").is_none());
        assert!(!registry
            .command_names()
            .iter()
            .any(|name| name == "__movement"));
        let mut body = Body::new();
        assert_eq!(
            movement(&mut body, &["__limit", "북"]),
            CommandResult::InternalNotHandled
        );
    }

    #[tokio::test]
    async fn update_is_hot_reloadable_rhai_with_exact_python_messages() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        assert!(
            registry.get("업데이트").is_none(),
            "the removed Rust built-in must not shadow cmds/업데이트.rhai"
        );

        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("업데이트").unwrap();
        assert_eq!(command.level, 1000);
        assert!(command.aliases.is_empty());
        assert_eq!(registry.get("업").unwrap().name, "업데이트");

        let mut body = Body::new();
        let denied = (command.handler)(&mut body, &["표현"]);
        assert_eq!(
            denied,
            CommandResult::Output("☞ 무슨 말인지 모르겠어요. *^_^*".to_string())
        );

        body.set("관리자등급", 1000_i64);
        assert_eq!(
            (command.handler)(&mut body, &[]),
            CommandResult::Output(
                "* 명령어, 무림별호, 도움말, 무공, 표현, 도우미, 메인설정, 스크립트 중에 선택하세요"
                    .to_string()
            )
        );

        let cases = [
            ("명령어", "* 명령어가 업데이트 되었습니다."),
            ("무림별호", "* 무림별호가 업데이트 되었습니다."),
            ("도움말", "* 도움말이 업데이트 되었습니다."),
            ("무공", "* 무공이 업데이트 되었습니다."),
            ("표현", "* 표현이 업데이트 되었습니다."),
            ("도우미", "* 도우미가 업데이트 되었습니다."),
            ("메인설정", "* 메인설정이 업데이트 되었습니다."),
            ("스크립트", "* 스크립트가 업데이트 되었습니다."),
        ];
        for (target, expected) in cases {
            assert_eq!(
                (command.handler)(&mut body, &[target]),
                CommandResult::Output(expected.to_string()),
                "{target}"
            );
        }

        assert_eq!(
            (command.handler)(&mut body, &["알 수 없음"]),
            CommandResult::Ok,
            "Python has no final else output"
        );
    }

    #[tokio::test]
    async fn leave_and_reboot_are_rhai_commands_without_invented_system_names() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        register_script_commands(&mut registry, storage, None, None, None).await;

        for invented in ["quit", "셧다운", "shutdown"] {
            assert!(registry.get(invented).is_none(), "{invented}");
        }

        let mut body = Body::new();
        for name in ["끝", "종료"] {
            let command = registry.get(name).unwrap();
            assert_eq!(
                (command.handler)(&mut body, &[]),
                CommandResult::Disconnect("\r\n다음에 또 만나요~!!!\r\n".to_string()),
                "{name}"
            );

            body.act = crate::player::ActState::Fight;
            assert_eq!(
                (command.handler)(&mut body, &[]),
                CommandResult::Output(
                    "☞ 지금은 무림을 떠나기에 좋은 상황이 아니네요. ^_^".to_string()
                ),
                "{name}"
            );
            body.act = crate::player::ActState::Stand;
        }

        let reboot = registry.get("리부팅").unwrap();
        assert_eq!(
            (reboot.handler)(&mut body, &[]),
            CommandResult::Output("☞ 무슨 말인지 모르겠어요. *^_^*".to_string())
        );
        body.set("관리자등급", 1000_i64);
        assert_eq!((reboot.handler)(&mut body, &[]), CommandResult::Reboot);
    }

    #[tokio::test]
    async fn update_commands_reloads_script_source_before_success_returns() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "muc_update_command_reload_{}_{}",
            std::process::id(),
            nonce
        ));
        let command_dir = root.join("cmds");
        std::fs::create_dir_all(&command_dir).unwrap();
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cmds/업데이트.rhai"),
            command_dir.join("업데이트.rhai"),
        )
        .unwrap();
        std::fs::write(
            command_dir.join("probe.rhai"),
            r#"fn main(ob, line) { send_line(ob, "before"); }"#,
        )
        .unwrap();

        let config = ScriptConfig {
            script_dir: command_dir.clone(),
            hot_reload: false,
            extension: ".rhai".to_string(),
            data_dir: root.join("config"),
            lib_dir: root.join("lib"),
        };
        let storage = Arc::new(RwLock::new(ScriptStorage::new(config)));
        let mut registry = CommandRegistry::new();
        register_script_commands(&mut registry, storage, None, None, None).await;
        let update = registry.get("업데이트").unwrap().handler.clone();
        let probe = registry.get("probe").unwrap().handler.clone();

        let mut body = Body::new();
        body.set("관리자등급", 1000_i64);
        assert_eq!(
            probe(&mut body, &[]),
            CommandResult::Output("before".to_string())
        );
        std::fs::write(
            command_dir.join("probe.rhai"),
            r#"fn main(ob, line) { send_line(ob, "after"); }"#,
        )
        .unwrap();

        assert_eq!(
            update(&mut body, &["명령어"]),
            CommandResult::Output("* 명령어가 업데이트 되었습니다.".to_string())
        );
        assert_eq!(
            probe(&mut body, &[]),
            CommandResult::Output("after".to_string())
        );

        std::fs::write(command_dir.join("probe.rhai"), "fn main(").unwrap();
        assert_eq!(
            update(&mut body, &["명령어"]),
            CommandResult::Ok,
            "Python logs a reload exception and does not invent a user error"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn python_aliases_resolve_and_exact_command_filenames_keep_their_own_handlers() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        register_script_commands(&mut registry, storage, None, None, None).await;

        for (alias, target) in crate::command::registry::PYTHON_RUNTIME_ALIASES {
            let command = registry
                .get(alias)
                .unwrap_or_else(|| panic!("Python alias {alias} -> {target} must resolve"));
            assert_eq!(command.name, *target, "Python alias {alias}");
        }

        let mut expected_names: HashSet<String> = [
            "북", "남", "동", "서", "위", "아래", "북서", "북동", "남서", "남동", "끝", "종료",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let command_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cmds");
        for entry in std::fs::read_dir(command_dir).expect("cmds directory must be readable") {
            let path = entry.expect("command entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("Python command must be readable");
            if !source.contains("class CmdObj") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("UTF-8 command filename");
            expected_names.insert(name.to_string());
            let command = registry
                .get(name)
                .unwrap_or_else(|| panic!("Python command {name} must register"));
            assert_eq!(command.name, name, "exact Python command name {name}");
        }

        let actual_names: HashSet<String> = registry.command_names().into_iter().collect();
        assert_eq!(
            actual_names, expected_names,
            "registry must expose only Python CmdObj files plus Python runtime-special commands"
        );
    }

    #[tokio::test]
    async fn password_change_is_executed_by_rhai_after_builtin_registration() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        assert!(
            registry.get("암호변경").is_none(),
            "Rust built-in must not shadow cmds/암호변경.rhai"
        );

        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("암호변경").unwrap();
        let mut body = Body::new();
        body.set("이름", "암호검사");
        let result = (command.handler)(&mut body, &[]);

        assert!(matches!(
            result,
            CommandResult::RequestInput {
                ref prompt,
                state: crate::command::PendingInput::ChangePasswordOld { ref text }
            } if prompt == "이전암호ː "
                && text.wrong_password == "☞ 현재의 암호가 맞지 않아요. ^^\r\n"
                && text.new_password_prompt == "☞ 변경 하실 암호를 입력해주세요. \r\n존함암호ː"
                && text.confirm_prompt == "☞ 한번 더 암호를 입력해주세요. \r\n암호확인ː"
                && text.mismatch == "☞ 이전 입력과 다릅니다. 암호변경을 취소합니다.\r\n"
                && text.success == "☞ 암호가 변경되었습니다."
        ));
    }

    #[tokio::test]
    async fn note_is_executed_by_rhai_with_python_location_and_view_layout() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        assert!(
            registry.get("쪽지").is_none(),
            "Rust built-in must not shadow cmds/쪽지.rhai"
        );

        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("쪽지").expect("Rhai note command");

        let player_name = "쪽지라이편집자";
        let mut body = Body::new();
        body.set("이름", player_name);
        body.memos.insert(
            "메모:보낸이".to_string(),
            crate::player::MemoRecord {
                제목: "안부".to_string(),
                시간: "2026-07-10 12:34:56".to_string(),
                작성자: "보낸이".to_string(),
                내용: "첫줄\r\n둘째줄".to_string(),
            },
        );

        // Python은 인수 여부보다 정보수집소 위치를 먼저 검사한다.
        let outside = (command.handler)(&mut body, &[]);
        assert!(matches!(
            outside,
            CommandResult::Output(ref output)
                if output == "정보수집소에서 할 수 있습니다."
        ));
        assert_eq!(body.memos.len(), 1, "outside view must not consume notes");

        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.set_player_position(
                player_name,
                crate::world::PlayerPosition::new("낙양성".to_string(), "11".to_string()),
            );
        }
        let viewed = (command.handler)(&mut body, &[]);
        let expected = concat!(
            "┌────────────────────────────────────┐\r\n",
            "│◁                    무           림           첩                    ▷│\r\n",
            "└────────────────────────────────────┘\r\n",
            "\x1b[33m보 낸 이\x1b[37m : 보낸이\r\n",
            "\x1b[33m제    목\x1b[37m : 안부\r\n",
            "\x1b[33m작성시각\x1b[37m : 2026-07-10 12:34:56\r\n\r\n",
            "첫줄\r\n둘째줄\r\n",
            " ─────────────────────────────────────"
        );
        let CommandResult::Output(output) = viewed else {
            panic!("note view must return Rhai output");
        };
        assert_eq!(output, expected);
        assert!(body.memos.is_empty());

        crate::script::set_precomputed_connected_names(vec![rhai::Dynamic::from("접속수신자")]);
        let connected = (command.handler)(&mut body, &["접속수신자", "제목"]);
        crate::script::clear_precomputed_all_online();
        assert!(matches!(
            connected,
            CommandResult::Output(ref output)
                if output == "접속중인 사용자에게는 보낼 수 없습니다."
        ));

        crate::world::get_world_state()
            .write()
            .unwrap()
            .remove_player_position(player_name);
    }

    #[tokio::test]
    async fn tell_and_reply_are_python_backed_rhai_with_exact_target_rules() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        register_script_commands(&mut registry, storage, None, None, None).await;

        let tell = registry.get("전음").expect("cmds/전음.rhai");
        let reply = registry.get("반전음").expect("cmds/반전음.rhai");
        assert_eq!(registry.get("전").unwrap().name, "전음");
        assert_eq!(registry.get("/").unwrap().name, "전음");
        assert_eq!(registry.get(":").unwrap().name, "반전음");
        assert_eq!(registry.get("반전").unwrap().name, "반전음");
        assert_eq!(registry.get("반").unwrap().name, "반전음");
        for invented in ["tell", "reply", "whisper"] {
            assert!(registry.get(invented).is_none(), "{invented}");
        }

        let sender_name = "전음라이발신자";
        let target_name = "전음라이수신자";
        let target_token = "127.0.0.1:31111";
        let mut body = Body::new();
        body.set("이름", sender_name);
        assert!(matches!(
            (tell.handler)(&mut body, &[]),
            CommandResult::Output(ref output)
                if output == "☞ 사용법: [대상] [내용] 전음(/)"
        ));
        assert!(matches!(
            (reply.handler)(&mut body, &[]),
            CommandResult::Output(ref output)
                if output == "☞ 사용법: [내용] 반전음(:)"
        ));
        assert!(matches!(
            (reply.handler)(&mut body, &["답장"]),
            CommandResult::Output(ref output)
                if output == "☞ 전음이 전달될만한 상대가 없어요. ^^"
        ));
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.set_player_position(
                sender_name,
                crate::world::PlayerPosition::new("전음검사존".to_string(), "1".to_string()),
            );
        }

        // Python은 대상 탐색을 먼저 하므로 발신자가 전음거부 중이어도
        // 대상이 없으면 "상대가 없어요"가 먼저다.
        body.set("설정상태", "전음거부 1");
        crate::script::set_precomputed_tell_players(Vec::new());
        let missing = (tell.handler)(&mut body, &[target_name, "안녕"]);
        assert!(matches!(
            missing,
            CommandResult::Output(ref output)
                if output == "☞ 전음이 전달될만한 상대가 없어요. ^^"
        ));

        // 대상과 발신자의 거부 설정은 대상 탐색 다음, noComm 이전에
        // 같은 Python 문구로 처리된다.
        body.set("설정상태", "전음거부 0");
        crate::script::set_precomputed_tell_players(vec![crate::script::TellPlayerSnapshot::new(
            target_token.to_string(),
            target_name.to_string(),
            true,
            false,
            "전음거부 1",
            1,
            31,
            45,
            7,
            9,
            false,
        )]);
        assert!(matches!(
            (tell.handler)(&mut body, &[target_name, "안녕"]),
            CommandResult::Output(ref output) if output == "☞ 전음 거부중이에요. ^^"
        ));
        body.set("설정상태", "전음거부 1");
        crate::script::set_precomputed_tell_players(vec![crate::script::TellPlayerSnapshot::new(
            target_token.to_string(),
            target_name.to_string(),
            true,
            false,
            "전음거부 0",
            1,
            31,
            45,
            7,
            9,
            false,
        )]);
        assert!(matches!(
            (tell.handler)(&mut body, &[target_name, "안녕"]),
            CommandResult::Output(ref output) if output == "☞ 전음 거부중이에요. ^^"
        ));

        body.set("설정상태", "전음거부 0");
        let no_comm_room = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let room = world.room_cache.get_room("절강성", "224").unwrap();
            world.set_player_position(
                sender_name,
                crate::world::PlayerPosition::new("절강성".to_string(), "224".to_string()),
            );
            room
        };
        no_comm_room
            .write()
            .unwrap()
            .properties
            .push("모든통신금지".to_string());
        assert!(matches!(
            (tell.handler)(&mut body, &[target_name, "안녕"]),
            CommandResult::Output(ref output)
                if output == "☞ 이지역에서는 어떠한 통신도 불가능합니다."
        ));
        no_comm_room
            .write()
            .unwrap()
            .properties
            .retain(|property| property != "모든통신금지");
        crate::world::get_world_state()
            .write()
            .unwrap()
            .set_player_position(
                sender_name,
                crate::world::PlayerPosition::new("전음검사존".to_string(), "1".to_string()),
            );

        crate::script::set_precomputed_tell_players(vec![crate::script::TellPlayerSnapshot::new(
            target_token.to_string(),
            target_name.to_string(),
            true,
            false,
            "전음거부 0\n엘피출력 0",
            1,
            31,
            45,
            7,
            9,
            false,
        )]);
        let sent = (tell.handler)(&mut body, &[target_name, "여러", "단어"]);
        let tag = "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] ";
        let msg1 = format!("{tag}{target_name}에게 보냄 : 여러 단어 ");
        let msg2 = format!("{tag}{sender_name} : 여러 단어 ");
        assert!(matches!(
            sent,
            CommandResult::Tell {
                ref target_token,
                ref sender_output,
                ref recipient_output,
                ref history_line,
            } if target_token == "127.0.0.1:31111"
                && sender_output == &format!("{msg1}\r\n\r\n")
                && recipient_output == &format!(
                    "\r\n{msg2}\r\n\r\n\x1b[0;37;40m[ 31/45, 7/9 ] "
                )
                && history_line == &msg2
        ));

        // Python은 자기 자신도 ACTIVE/비투명 대상 탐색에 포함한다.
        crate::script::set_precomputed_tell_players(vec![crate::script::TellPlayerSnapshot::new(
            "127.0.0.1:31110".to_string(),
            sender_name.to_string(),
            true,
            false,
            "",
            1,
            50,
            60,
            7,
            8,
            true,
        )]);
        let self_tell = (tell.handler)(&mut body, &[sender_name, "혼잣말"]);
        assert!(matches!(
            self_tell,
            CommandResult::Tell {
                ref sender_output,
                ref recipient_output,
                ..
            } if sender_output.ends_with("혼잣말 \r\n")
                && !sender_output.ends_with("혼잣말 \r\n\r\n")
                && recipient_output.ends_with("[ 50/60, 7/8 ] \r\n")
        ));

        // 반전음은 저장된 접속 객체가 channel.players에 있기만 하면
        // ACTIVE/투명 여부를 다시 검사하지 않는다.
        body.temp_mut().insert(
            crate::script::TELL_TALKER_TOKEN.to_string(),
            crate::object::Value::String(target_token.to_string()),
        );
        crate::script::set_precomputed_tell_players(vec![crate::script::TellPlayerSnapshot::new(
            target_token.to_string(),
            target_name.to_string(),
            false,
            true,
            "",
            1,
            30,
            45,
            6,
            9,
            false,
        )]);
        let replied = (reply.handler)(&mut body, &["답", "장"]);
        assert!(
            matches!(replied, CommandResult::Tell { ref history_line, .. }
            if history_line == &format!("{tag}{sender_name} : 답 장 "))
        );

        // 접속 객체가 사라지면 Python처럼 `_talker`도 비운다.
        crate::script::set_precomputed_tell_players(Vec::new());
        let disconnected = (reply.handler)(&mut body, &["답장"]);
        assert!(matches!(
            disconnected,
            CommandResult::Output(ref output)
                if output == "☞ 전음이 전달될만한 상대가 없어요. ^^"
        ));
        assert!(!body.temp().contains_key(crate::script::TELL_TALKER_TOKEN));

        crate::script::clear_precomputed_all_online();
        crate::world::get_world_state()
            .write()
            .unwrap()
            .remove_player_position(sender_name);
    }

    #[tokio::test]
    async fn return_home_is_executed_by_rhai_and_only_notifies_both_rooms() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        assert!(
            registry.get("귀환").is_none(),
            "Rust movement commands must not shadow cmds/귀환.rhai"
        );

        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("귀환").expect("cmds/귀환.rhai must register");
        assert_eq!(registry.get("귀").unwrap().name, "귀환");
        assert_eq!(registry.get("귀가").unwrap().name, "귀환");

        let self_name = "귀환라이검사본인";
        let old_room_name = "귀환라이검사출발방";
        let destination_name = "귀환라이검사도착방";
        let outsider_name = "귀환라이검사다른방";
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "1").unwrap();
            world.room_cache.get_room("동정호", "1").unwrap();
            world.set_player_position(
                self_name,
                crate::world::PlayerPosition::new("산동성".to_string(), "1".to_string()),
            );
            world.set_player_position(
                old_room_name,
                crate::world::PlayerPosition::new("산동성".to_string(), "1".to_string()),
            );
            world.set_player_position(
                destination_name,
                crate::world::PlayerPosition::new("동정호".to_string(), "1".to_string()),
            );
            world.set_player_position(
                outsider_name,
                crate::world::PlayerPosition::new("동정호".to_string(), "2".to_string()),
            );
        }

        let mut body = Body::new();
        body.set("이름", self_name);
        body.set("레벨", 1);
        body.set("귀환지맵", "동정호:1");
        let result = (command.handler)(&mut body, &[]);

        let actor = format!(
            "\x1b[1m{}\x1b[0;37m{}",
            self_name,
            crate::hangul::han_iga(self_name)
        );
        let old_room_message = format!(
            "{} 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'",
            actor
        );
        let destination_message = format!("{} 하늘에서 사뿐히 내려 앉습니다. '척~~~'", actor);
        assert!(matches!(
            result,
            CommandResult::OutputAndSendToUsers(ref output, ref sends)
                if output.starts_with("당신이 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'\r\n")
                    && output.contains("동정호변 백석로")
                    && sends.contains(&(old_room_name.to_string(), old_room_message.clone()))
                    && sends.contains(&(destination_name.to_string(), destination_message.clone()))
                    && !sends.iter().any(|(name, _)| name == outsider_name)
        ));
        assert_eq!(body.get_string("위치"), "동정호:1");
        assert_eq!(body.get_string("현재방"), "동정호:1");

        let mut world = crate::world::get_world_state().write().unwrap();
        for name in [self_name, old_room_name, destination_name, outsider_name] {
            world.remove_player_position(name);
        }
    }

    #[tokio::test]
    async fn death_progress_presentation_is_owned_by_combat_tick_rhai() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry
            .get_internal("combat_tick")
            .expect("cmds/__combat_tick.rhai must register");
        let mut body = Body::new();
        body.set("이름", "사망단계라이검사");
        body.set("체력", 0_i64);
        body.set("최고체력", 900_i64);
        body.set("내공", 18_i64);
        body.set("최고내공", 18_i64);
        crate::script::combat_commands::queue_combat_presentation_event(
            &mut body,
            serde_json::json!({
                "kind": "death_progress", "step": 0, "insured_items": 0,
            }),
        );

        let result = (command)(&mut body, &[]);
        assert!(matches!(
            result,
            CommandResult::Output(ref output)
                if output.contains("기혈이 거꾸로 돌며 정신이 혼미해 집니다.")
                    && output.contains("[ 0/900, 18/18 ]")
        ));
    }

    #[tokio::test]
    async fn return_home_preserves_python_guard_order_and_invalid_map_behavior() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("귀환").unwrap();

        let name = "귀환검사순서";
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.room_cache.get_room("혈광", "1").unwrap();
            world.room_cache.get_room("산동성", "1").unwrap();
            world.set_player_position(
                name,
                crate::world::PlayerPosition::new("혈광".to_string(), "1".to_string()),
            );
        }

        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1);
        body.set("귀환지맵", "산동성:1");
        body.act = crate::player::ActState::Rest;
        assert!(matches!(
            (command.handler)(&mut body, &[]),
            CommandResult::Output(ref output) if output == "☞ 이곳에선 귀환하실 수 없어요. ^^"
        ));

        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.set_player_position(
                name,
                crate::world::PlayerPosition::new("산동성".to_string(), "1".to_string()),
            );
        }
        body.act = crate::player::ActState::Stand;
        body.set("귀환지맵", "콜론이없는맵");
        assert!(matches!(
            (command.handler)(&mut body, &[]),
            CommandResult::Output(ref output)
                if output == "귀환지맵이 없습니다. 관리자에게 연락하세요."
        ));

        body.set("귀환지맵", "산동성:1");
        assert!(matches!(
            (command.handler)(&mut body, &[]),
            CommandResult::Output(ref output) if output == "☞ 같은 자리에요. ^^"
        ));

        crate::world::get_world_state()
            .write()
            .unwrap()
            .remove_player_position(name);
    }

    #[tokio::test]
    async fn return_home_room_scripts_override_both_default_movement_messages() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("귀환").unwrap();

        let self_name = "귀환방스크립검사본인";
        let old_room_name = "귀환방스크립검사출발";
        let destination_name = "귀환방스크립검사도착";
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "2").unwrap();
            world.room_cache.get_room("동정호", "2").unwrap();
            world.set_player_position(
                self_name,
                crate::world::PlayerPosition::new("산동성".to_string(), "2".to_string()),
            );
            world.set_player_position(
                old_room_name,
                crate::world::PlayerPosition::new("산동성".to_string(), "2".to_string()),
            );
            world.set_player_position(
                destination_name,
                crate::world::PlayerPosition::new("동정호".to_string(), "2".to_string()),
            );
            world.get_room_attrs_mut("산동성", "2").insert(
                "이동스크립:귀환".to_string(),
                "[공](이/가) 안개 속으로 사라집니다.".to_string(),
            );
            world.get_room_attrs_mut("동정호", "2").insert(
                "진입스크립:귀환".to_string(),
                "[공](이/가) 안개 속에서 나타납니다.".to_string(),
            );
        }

        let mut body = Body::new();
        body.set("이름", self_name);
        body.set("레벨", 1);
        body.set("귀환지맵", "동정호:2");
        let result = (command.handler)(&mut body, &[]);

        let name_a = format!("\x1b[1m{}\x1b[0;37m", self_name);
        let expected_old = format!("\r\n{}이 안개 속으로 사라집니다.", name_a);
        let expected_destination = format!("{}이 안개 속에서 나타납니다.", name_a);
        assert!(matches!(
            result,
            CommandResult::OutputAndSendToUsers(ref output, ref sends)
                if output.starts_with("\r\n당신이 안개 속으로 사라집니다.\r\n")
                    && sends.contains(&(old_room_name.to_string(), expected_old))
                    && sends.contains(&(destination_name.to_string(), expected_destination))
                    && !output.contains("무영지신")
                    && !sends.iter().any(|(_, message)| message.contains("무영지신") || message.contains("척~~~"))
        ));

        let mut world = crate::world::get_world_state().write().unwrap();
        world
            .get_room_attrs_mut("산동성", "2")
            .remove("이동스크립:귀환");
        world
            .get_room_attrs_mut("동정호", "2")
            .remove("진입스크립:귀환");
        for name in [self_name, old_room_name, destination_name] {
            world.remove_player_position(name);
        }
    }

    #[tokio::test]
    async fn return_home_runs_destination_entry_events_after_showing_the_room() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("귀환").unwrap();

        let name = "귀환입장이벤트검사";
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "3").unwrap();
            world.room_cache.get_room("낙양성", "99999").unwrap();
            world.set_player_position(
                name,
                crate::world::PlayerPosition::new("산동성".to_string(), "3".to_string()),
            );
        }

        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1);
        body.set("귀환지맵", "낙양성:99999");
        let result = (command.handler)(&mut body, &[]);
        let CommandResult::Output(output) = result else {
            panic!("entry-event return must emit the room and event output");
        };
        let room_at = output.find("낙양성-대장간내부").unwrap();
        let event_at = output
            .find("어서 오게나!!! 장비에 이름을 새기러 왔나?")
            .unwrap();
        assert!(
            room_at < event_at,
            "Python shows the room before entry events"
        );
        assert!(output.contains("[안내문] 이라고 쳐보게나~\""));
        assert_eq!(body.get_string("위치"), "낙양성:99999");

        let mut world = crate::world::get_world_state().write().unwrap();
        let mobs = world.mob_cache.get_all_mobs_in_room("낙양성", "99999");
        assert!(
            mobs.iter().all(|mob| mob.tick == 1),
            "safe Room.update must preserve Python Mob.update tick"
        );
        world.remove_player_position(name);
    }

    #[tokio::test]
    async fn return_home_applies_nonlethal_room_hazard_after_entry_output() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("귀환").unwrap();

        let name = "귀환함정검사본인";
        let observer = "귀환함정검사목격자";
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "4").unwrap();
            world.room_cache.get_room("호북성", "578").unwrap();
            world.set_player_position(
                name,
                crate::world::PlayerPosition::new("산동성".to_string(), "4".to_string()),
            );
            world.set_player_position(
                observer,
                crate::world::PlayerPosition::new("호북성".to_string(), "578".to_string()),
            );
        }

        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1);
        body.set("체력", 2000);
        body.set("최고체력", 2000);
        body.set("귀환지맵", "호북성:578");
        let result = (command.handler)(&mut body, &[]);
        assert!(matches!(
            result,
            CommandResult::OutputAndSendToUsers(ref output, ref sends)
                if output.contains("당신이 우물 밑바닥으로 떨어지며 심각한 부상을 입었습니다.")
                    && sends.iter().any(|(target, message)|
                        target == observer && message.contains("우물 밑바닥으로 떨어지며 심각한 부상을 입었습니다."))
        ));
        let CommandResult::OutputAndSendToUsers(output, _) = &result else {
            unreachable!();
        };
        let prompt_at = output.find("\x1b[0;37;40m[ 2000/2000, 0/0 ] ").unwrap();
        let hazard_at = output
            .find("당신이 우물 밑바닥으로 떨어지며 심각한 부상을 입었습니다.")
            .unwrap();
        assert!(prompt_at < hazard_at, "Python lpPrompt runs before minusHP");
        assert_eq!(body.get_hp(), 1004);
        assert_eq!(body.get_string("위치"), "호북성:578");

        let mut world = crate::world::get_world_state().write().unwrap();
        world.remove_player_position(name);
        world.remove_player_position(observer);
    }

    #[tokio::test]
    async fn return_home_lethal_hazard_moves_then_runs_rhai_death_drop() {
        let mut config = repository_script_config();
        config.lib_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib");
        let storage = Arc::new(RwLock::new(ScriptStorage::new(config)));
        let mut registry = CommandRegistry::new();
        register_script_commands(&mut registry, storage, None, None, None).await;
        let command = registry.get("귀환").unwrap();
        let name = "귀환치명함정통합검사";
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "4").unwrap();
            world.room_cache.get_room("호북성", "578").unwrap();
            world.set_player_position(
                name,
                crate::world::PlayerPosition::new("산동성".to_string(), "4".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1_i64);
        body.set("체력", 500_i64);
        body.set("최고체력", 500_i64);
        body.set("귀환지맵", "호북성:578");
        let mut item = crate::object::Object::new();
        item.set("인덱스", "치명함정검");
        item.set("이름", "치명함정검");
        item.set("반응이름", "검");
        item.set("아이템속성", "보험적용안됨");
        body.object
            .objs
            .push(std::sync::Arc::new(std::sync::Mutex::new(item)));

        let result = (command.handler)(&mut body, &[]);
        let output = match result {
            CommandResult::Output(output) => output,
            CommandResult::OutputAndSendToUsers(output, _) => output,
            other => panic!("unexpected lethal-return result: {other:?}"),
        };
        let hazard = output.find("우물 밑바닥으로 떨어지며").unwrap();
        let dropped = output.find("치명함정검").unwrap();
        let coma = output.find("당신은 정신이 혼미합니다.").unwrap();
        assert!(hazard < dropped && dropped < coma);
        assert_eq!(body.act, crate::player::ActState::Death);
        assert_eq!(body.get_hp(), 0);
        assert_eq!(body.get_string("위치"), "호북성:578");
        assert!(body.object.objs.is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        assert_eq!(world.get_room_objs("호북성", "578").len(), 1);
        world.get_room_objs_mut("호북성", "578").clear();
        world.remove_player_position(name);
    }

    #[tokio::test]
    async fn vision_commands_are_python_backed_rhai_without_invented_commands() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);

        for name in ["비전", "비전삭제", "비전목록", "비전수련"] {
            assert!(
                !registry.contains(name),
                "{name} must not be a Rust built-in"
            );
        }

        register_script_commands(&mut registry, storage, None, None, None).await;
        assert!(registry.get("비전목록").is_none());
        assert!(registry.get("비전수련").is_none());

        let set_command = registry.get("비전").expect("cmds/비전.rhai must register");
        let delete_command = registry
            .get("비전삭제")
            .expect("cmds/비전삭제.rhai must register");
        let mut body = Body::new();
        body.set("이름", "비전검사");
        body.set("비전이름", "강룡십팔장비전|무극검비전");

        assert!(matches!(
            (set_command.handler)(&mut body, &[]),
            CommandResult::Output(ref output) if output == "☞ 비전 : 없음"
        ));
        assert!(matches!(
            (set_command.handler)(&mut body, &["없는비전"]),
            CommandResult::Output(ref output)
                if output == "☞ 당신은 그런 비전을 배운적이 없습니다."
        ));
        assert!(matches!(
            (set_command.handler)(&mut body, &["강룡십팔장비전"]),
            CommandResult::Output(ref output) if output == "☞ 비전을 지정하였습니다."
        ));
        assert_eq!(body.get_vision_setting(), "강룡십팔장비전");

        assert!(matches!(
            (delete_command.handler)(&mut body, &[]),
            CommandResult::Output(ref output) if output == "☞ 지정된 비전을 삭제합니다."
        ));
        assert_eq!(body.get_vision_setting(), "");
        assert!(matches!(
            (delete_command.handler)(&mut body, &[]),
            CommandResult::Output(ref output) if output == "☞ 지정된 비전이 없습니다."
        ));
    }

    #[tokio::test]
    async fn say_is_rhai_and_targets_only_players_in_the_same_room() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        assert!(registry.get("말").is_none());
        assert!(registry.get("속삭여").is_none());

        register_script_commands(&mut registry, storage, None, None, None).await;
        assert!(registry.get(".").is_none());
        assert!(registry.get("say").is_none());
        assert!(registry.get("whisper").is_none());

        let self_name = "말동일방검사본인";
        let same_room_name = "말동일방검사상대";
        let other_room_name = "말다른방검사상대";
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.set_player_position(
                self_name,
                crate::world::PlayerPosition::new("말검사존".to_string(), "1".to_string()),
            );
            world.set_player_position(
                same_room_name,
                crate::world::PlayerPosition::new("말검사존".to_string(), "1".to_string()),
            );
            world.set_player_position(
                other_room_name,
                crate::world::PlayerPosition::new("말검사존".to_string(), "2".to_string()),
            );
        }

        let command = registry.get("말").unwrap();
        let mut body = Body::new();
        body.set("이름", self_name);
        let result = (command.handler)(&mut body, &["안녕"]);
        let own = "당신이 말합니다 : '안녕\x1b[0;40;37m'";
        let room = format!(
            "\x1b[33m{}\x1b[37m{} 말합니다 : '안녕\x1b[0;40;37m'",
            self_name,
            crate::hangul::han_iga(self_name)
        );
        assert!(matches!(
            result,
            CommandResult::OutputAndSendToUsers(ref output, ref sends)
                if output == &format!("{own}\r\n{room}")
                    && sends == &vec![(same_room_name.to_string(), room)]
        ));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.remove_player_position(self_name);
        world.remove_player_position(same_room_name);
        world.remove_player_position(other_room_name);
    }

    #[tokio::test]
    async fn equipment_is_python_backed_rhai_without_invented_commands() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        for name in ["장비", "입고", "벗고", "아이템정보"] {
            assert!(registry.get(name).is_none(), "{name} must not be built in");
        }

        register_script_commands(&mut registry, storage, None, None, None).await;
        assert!(registry.get("장비").is_some());
        assert!(registry.get("입고").is_none());
        assert!(registry.get("벗고").is_none());
        assert!(registry.get("아이템정보").is_none());
        assert!(registry.get("equipment").is_none());
        assert!(registry.get("equip").is_none());
        assert!(registry.get("unequip").is_none());
    }

    #[tokio::test]
    async fn help_uses_the_python_backed_korean_rhai_command() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        register_script_commands(&mut registry, storage, None, None, None).await;

        assert!(registry.get("help").is_none());
        assert!(registry.get("?").is_none());
        assert!(registry.get("/h").is_none());

        let mut body = Body::new();
        body.set("이름", "도움말검사");
        let command = registry.get("도움말").unwrap();
        let result = (command.handler)(&mut body, &["__없는_항목__"]);
        assert!(matches!(
            result,
            CommandResult::Output(ref output) if output == "☞ 해당 도움말이 없어요. ^^"
        ));

        // Python does not trim a non-empty HELP key.  This must not resolve
        // to the ordinary "도움말" topic.
        let spaced = (command.handler)(&mut body, &[" 도움말 "]);
        assert!(matches!(
            spaced,
            CommandResult::Output(ref output) if output == "☞ 해당 도움말이 없어요. ^^"
        ));
    }

    #[tokio::test]
    async fn mugong_is_registered_as_hot_reloadable_rhai_not_rust_builtin() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);

        assert!(
            !registry.contains("무공"),
            "무공 Python 출력은 Rust built-in에 남아 있으면 안 된다"
        );
        register_script_commands(&mut registry, storage, None, None, None).await;

        let command = registry.get("무공").expect("cmds/무공.rhai must register");
        assert_eq!(command.description, "무공 명령어");
        assert_eq!(registry.get("기술").unwrap().name, "무공");

        let mut body = Body::new();
        body.set("이름", "검사자");
        let result = (command.handler)(&mut body, &[]);
        let CommandResult::Output(output) = result else {
            panic!("Rhai 무공 명령 출력이어야 한다");
        };
        assert!(output.starts_with("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n"));
        assert!(output.contains("☞ 깨우친 무공이 없습니다."));
        assert!(!output.contains("기술 목록"));
    }

    #[tokio::test]
    async fn cast_is_registered_as_hot_reloadable_rhai_not_rust_placeholder() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
        let mut registry = CommandRegistry::new();
        crate::command::commands::combat::register_combat_commands(&mut registry);

        assert!(
            !registry.contains("시전"),
            "Rust combat commands must not shadow cmds/시전.rhai"
        );
        register_script_commands(&mut registry, storage, None, None, None).await;

        assert_eq!(registry.get("시전").unwrap().description, "시전 명령어");
        assert_eq!(registry.get("시").unwrap().name, "시전");
        assert!(registry.get("무공시전").is_none());
        assert!(registry.get("cast").is_none());
        assert!(registry.get("skill").is_none());
    }
}
