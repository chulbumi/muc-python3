//! Command system for MUD engine
//!
//! This module provides a comprehensive command parsing and execution system
//! for handling player input in the MUD.

pub mod parser;
pub mod handler;
pub mod registry;
pub mod commands;

pub use parser::{CommandParser, ParsedCommand};
pub use handler::{CommandHandler, CommandResult, CommandContext, PendingInput};
pub use registry::{CommandRegistry, CommandInfo, CommandFn};
pub use commands::{register_basic_commands};

/// Direction constants for movement commands
pub const DIRECTIONS: &[(&str, &str, &str)] = &[
    ("북", "north", "북쪽"),
    ("남", "south", "남쪽"),
    ("동", "east", "동쪽"),
    ("서", "west", "서쪽"),
    ("위", "up", "위로"),
    ("아래", "down", "아래로"),
];

/// Direction aliases for movement
pub const DIRECTION_ALIASES: &[(&str, &str)] = &[
    ("n", "북"),
    ("s", "남"),
    ("e", "동"),
    ("w", "서"),
    ("u", "위"),
    ("d", "아래"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directions_count() {
        assert_eq!(DIRECTIONS.len(), 6);
    }

    #[test]
    fn test_direction_aliases() {
        assert_eq!(DIRECTION_ALIASES.len(), 6);
    }
}
