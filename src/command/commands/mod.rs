//! Basic command implementations for MUD engine
//!
//! This module contains the core game commands that players can use.

pub mod admin;
pub mod combat;
pub mod communication;
pub mod equipment;
pub mod give;
pub mod info;
pub mod movement;
pub mod note;
pub mod script;
pub mod skills;
pub mod system;
pub mod update;
pub mod vision;

pub use admin::*;
pub use combat::*;
pub use communication::*;
pub use equipment::*;
pub use give::*;
pub use info::*;
pub use movement::*;
pub use note::*;
pub use script::*;
pub use skills::*;
pub use system::*;
pub use update::*;
pub use vision::*;

use crate::command::registry::CommandRegistry;
use crate::command::{CommandFn, CommandResult};
use crate::player::Body;
use std::sync::Arc;

/// Registers all basic commands with the registry
pub fn register_basic_commands(registry: &mut CommandRegistry) {
    register_movement_commands(registry);
    register_info_commands(registry);
    register_communication_commands(registry);
    register_combat_commands(registry);
    register_skill_commands(registry);
    register_vision_commands(registry);
    register_system_commands(registry);
    register_give_commands(registry);
    register_update_commands(registry);
    register_note_commands(registry);
    register_equipment_commands(registry);
    register_admin_commands(registry);
}

/// Helper to create a command function wrapper
#[allow(dead_code)]
fn make_command<F>(f: F) -> CommandFn
where
    F: Fn(&mut Body, &[&str]) -> CommandResult + Send + Sync + 'static,
{
    Arc::new(f)
}
