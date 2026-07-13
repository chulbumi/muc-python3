//! Movement is a Python `Player.parse_command` special branch. The private
//! hot-reloaded `cmds/__movement.rhai` handler owns its behavior. Directions
//! must not appear in the player command registry.

use crate::command::registry::CommandRegistry;

pub fn register_movement_commands(registry: &mut CommandRegistry) {
    let _ = registry;
}
