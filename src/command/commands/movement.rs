//! Movement is a Python `Player.parse_command` special branch. The private
//! hot-reloaded `cmds/__movement.rhai` handler owns its behavior; these silent
//! registry entries only make Python's direction aliases resolve to a known
//! command when the special branch declines a line.

use crate::command::registry::CommandRegistry;
use crate::command::CommandResult;

pub fn register_movement_commands(registry: &mut CommandRegistry) {
    for direction in [
        "북", "남", "동", "서", "위", "아래", "북서", "북동", "남서", "남동",
    ] {
        registry.register_simple(
            direction,
            |_body, _args| CommandResult::InternalNotHandled,
            "",
        );
    }
}
