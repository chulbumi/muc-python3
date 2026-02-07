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

/// Execute a script command
pub async fn execute_script_command(
    script_storage: &ScriptStorage,
    player: &mut Body,
    script_name: &str,
    line: &str,
    get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
    get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
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
    get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
    get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
) -> CommandFn {
    Arc::new(move |player: &mut Body, args: &[&str]| -> CommandResult {
        println!(
            "[DEBUG SCRIPT] Executing script command: {}, args: {:?}",
            script_name, args
        );
        let line = args.join(" ");
        let storage = script_storage.try_read();
        let storage = match storage {
            Ok(s) => s,
            Err(_) => {
                let msg = "Script storage unavailable".to_string();
                return CommandResult::Error(msg);
            }
        };
        match storage.execute(
            &script_name,
            player,
            &line,
            get_other_players_desc.clone(),
            get_other_players_map.clone(),
            call_out_scheduler.clone(),
        ) {
            Ok((outputs, special)) => {
                println!(
                    "[SCRIPT_CMD] Script {} executed, outputs.len()={}, special={:?}",
                    script_name,
                    outputs.len(),
                    special
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
                    println!("[SCRIPT_CMD] No outputs, returning Ok");
                    CommandResult::Ok
                } else {
                    println!("[SCRIPT_CMD] Returning {} output lines", outputs.len());
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
    get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
    get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
) {
    let scripts = script_storage.read().await;
    let script_names = scripts.script_names();
    drop(scripts);

    println!(
        "[SCRIPT_CMD] Found {} scripts to register",
        script_names.len()
    );

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

        // Rhai 전환된 주다/외쳐/전음/표현: 기존 alias 유지 (레지스트리 built-in 외 별도)
        let aliases: Vec<String> = match name_clone.as_str() {
            "주다" => vec![
                "줘".to_string(),
                "주".to_string(),
                "give".to_string(),
                "선물".to_string(),
                "선".to_string(),
            ],
            "외쳐" => vec![
                "외".to_string(),
                "외침".to_string(),
                "잡".to_string(),
                "잡담".to_string(),
                ",".to_string(),
                "shout".to_string(),
                "창".to_string(),
                "창룡".to_string(),
                "창룡후".to_string(),
                "외친다".to_string(),
            ],
            "전음" => vec!["전".to_string(), "/".to_string()],
            "표현" => vec!["표".to_string(), "'".to_string(), "emote".to_string()],
            _ => vec![],
        };

        // Create CommandInfo
        let info = CommandInfo::new(
            name_clone.clone(),
            aliases,
            command_fn,
            0, // Level 0 = all players can use
            description.clone(),
            usage,
        );

        // Register the command
        registry.register(info);
        info!("[SCRIPT_CMD] Registered command: {}", name_clone);
    }

    println!("[SCRIPT_CMD] Script registration complete");
}
