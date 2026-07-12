//! Basic command implementations for MUD engine
//!
//! This module contains the core game commands that players can use.

pub mod combat;
pub mod info;
pub mod movement;
pub mod note;
pub mod script;
pub mod update;

pub use combat::*;
pub use info::*;
pub use movement::*;
pub(crate) use note::*;
pub use script::*;

use crate::command::registry::CommandRegistry;
use crate::command::{CommandFn, CommandResult};
use crate::player::Body;
use std::sync::Arc;

/// Registers all basic commands with the registry
pub fn register_basic_commands(registry: &mut CommandRegistry) {
    register_movement_commands(registry);
    register_info_commands(registry);
    register_combat_commands(registry);
}

/// Helper to create a command function wrapper
#[allow(dead_code)]
fn make_command<F>(f: F) -> CommandFn
where
    F: Fn(&mut Body, &[&str]) -> CommandResult + Send + Sync + 'static,
{
    Arc::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_registry_does_not_expose_python_absent_admin_commands() {
        let mut registry = CommandRegistry::new();
        register_basic_commands(&mut registry);

        for invented in [
            "아이템생성",
            "spawn",
            "create",
            "위치이동",
            "warp",
            "teleport",
            "관리자도움말",
            "관리자정보",
            "adminhelp",
            "업데이트",
        ] {
            assert!(!registry.contains(invented), "{invented}");
        }
    }
}
