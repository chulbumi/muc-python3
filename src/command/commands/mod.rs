//! Basic command implementations for MUD engine
//!
//! This module contains the core game commands that players can use.

pub mod movement;
pub mod info;
pub mod communication;
pub mod combat;
pub mod script;

pub use movement::*;
pub use info::*;
pub use communication::*;
pub use combat::*;
pub use script::*;

use std::sync::Arc;
use crate::command::{CommandResult, CommandFn};
use crate::command::registry::CommandRegistry;
use crate::player::Body;

/// Registers all basic commands with the registry
pub fn register_basic_commands(registry: &mut CommandRegistry) {
    register_movement_commands(registry);
    register_info_commands(registry);
    register_communication_commands(registry);
    register_combat_commands(registry);
}

/// Helper to create a command function wrapper
fn make_command<F>(f: F) -> CommandFn
where
    F: Fn(&mut Body, &[&str]) -> CommandResult + Send + Sync + 'static,
{
    Arc::new(f)
}
