//! Combat command registration boundary.
//!
//! Python loads `cmds/쳐.py` and `cmds/도망.py` at runtime.  Their migrated
//! counterparts therefore live in `cmds/쳐.rhai` and `cmds/도망.rhai`; no Rust
//! built-in may shadow those hot-reloadable commands or own their output.

use crate::command::registry::CommandRegistry;

pub fn register_combat_commands(_registry: &mut CommandRegistry) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_combat_commands_are_not_registered_as_rust_builtins() {
        let mut registry = CommandRegistry::new();
        register_combat_commands(&mut registry);

        for name in [
            "쳐",
            "도망",
            "attack",
            "kill",
            "k",
            "flee",
            "run",
            "전투상태",
            "전상",
            "combat",
            "status",
            "결투",
            "duel",
            "pvp",
            "결투수락",
            "결투거절",
        ] {
            assert!(!registry.contains(name), "{name}");
        }
    }
}
