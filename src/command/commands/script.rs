//! Script command execution
//!
//! This module provides command execution through Rhai scripts.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use crate::command::{CommandResult, CommandFn};
use crate::command::registry::{CommandRegistry, CommandInfo};
use crate::player::{Body, body::SendLine};
use crate::script::ScriptStorage;

/// Execute a script command
pub async fn execute_script_command(
    script_storage: &ScriptStorage,
    player: &mut Body,
    script_name: &str,
    line: &str,
) -> CommandResult {
    match script_storage.execute(script_name, player, line) {
        Ok(_outputs) => {
            // Script outputs are sent via send_line function in the script
            CommandResult::Ok
        }
        Err(e) => {
            // Return error message
            let msg = format!("스크립트 실행 오류: {}", e);
            player.send_line(&msg);
            CommandResult::Error(msg)
        }
    }
}

/// Create a command function that executes a script
pub fn create_script_command(
    script_storage: Arc<RwLock<ScriptStorage>>,
    script_name: String,
) -> CommandFn {
    Arc::new(move |player: &mut Body, args: &[&str]| -> CommandResult {
        let line = args.join(" ");
        let storage = script_storage.try_read();
        let storage = match storage {
            Ok(s) => s,
            Err(_) => {
                let msg = "Script storage unavailable".to_string();
                return CommandResult::Error(msg);
            }
        };
        match storage.execute(&script_name, player, &line) {
            Ok(outputs) => {
                // Return outputs via CommandResult::Output
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
pub async fn register_script_commands(
    registry: &mut CommandRegistry,
    script_storage: Arc<RwLock<ScriptStorage>>,
) {
    let scripts = script_storage.read().await;
    let script_names = scripts.script_names();
    drop(scripts);

    info!("[SCRIPT_CMD] Found {} scripts to register", script_names.len());

    for script_name in script_names {
        // Skip if command already exists (built-in commands take priority)
        // Check both primary command names and aliases of existing commands
        if registry.contains(&script_name) {
            info!("[SCRIPT_CMD] Skipping {} (already registered as built-in)", script_name);
            continue;
        }

        // Also check if any existing command has this as an alias
        let mut is_alias = false;
        for cmd in registry.all_commands() {
            if cmd.matches(&script_name) {
                info!("[SCRIPT_CMD] Skipping {} (alias of existing command {})", script_name, cmd.name);
                is_alias = true;
                break;
            }
        }
        if is_alias {
            continue;
        }

        let storage = script_storage.clone();
        let name_clone = script_name.clone();

        // Create command from script
        let command_fn = create_script_command(storage, script_name);

        // Get description from script if available
        let description = format!("{} 명령어", name_clone);

        // Create CommandInfo
        let info = CommandInfo::new(
            name_clone.clone(),
            vec![], // No aliases for now
            command_fn,
            0,     // Level 0 = all players can use
            description.clone(),
            description,
        );

        // Register the command
        registry.register(info);
        info!("[SCRIPT_CMD] Registered command: {}", name_clone);
    }
}
