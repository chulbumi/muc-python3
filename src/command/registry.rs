//! Command registry for MUD engine
//!
//! Provides command registration, lookup, and management functionality.
//! Supports aliases, permission levels, and command metadata.

use std::collections::HashMap;
use std::sync::Arc;
use crate::player::Body;
use crate::command::CommandResult;

/// Function type for command handlers
///
/// Commands take a mutable reference to a player and a slice of argument strings
pub type CommandFn = Arc<dyn Fn(&mut Body, &[&str]) -> CommandResult + Send + Sync>;

/// Information about a registered command
#[derive(Clone)]
pub struct CommandInfo {
    /// The primary name of the command
    pub name: String,
    /// Alternative names that can be used to invoke the command
    pub aliases: Vec<String>,
    /// The handler function
    pub handler: CommandFn,
    /// Required permission/level to use this command (0 = all players)
    pub level: i32,
    /// Brief description of the command
    pub description: String,
    /// Usage example
    pub usage: String,
}

impl CommandInfo {
    /// Creates a new CommandInfo
    pub fn new(
        name: String,
        aliases: Vec<String>,
        handler: CommandFn,
        level: i32,
        description: String,
        usage: String,
    ) -> Self {
        CommandInfo {
            name,
            aliases,
            handler,
            level,
            description,
            usage,
        }
    }

    /// Creates a simple CommandInfo with minimal information
    pub fn simple(name: String, handler: CommandFn, description: &str) -> Self {
        CommandInfo {
            usage: format!("[인자...] {}", description),
            description: description.to_string(),
            name,
            aliases: Vec::new(),
            handler,
            level: 0,
        }
    }

    /// Checks if the given name matches this command (name or alias)
    pub fn matches(&self, name: &str) -> bool {
        if self.name == name {
            return true;
        }
        self.aliases.iter().any(|a| a == name)
    }
}

/// Registry for all game commands
pub struct CommandRegistry {
    /// Map of command names to CommandInfo
    commands: HashMap<String, CommandInfo>,
    /// Built-in aliases (like movement directions)
    aliases: HashMap<String, String>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    /// Creates a new empty CommandRegistry
    pub fn new() -> Self {
        CommandRegistry {
            commands: HashMap::new(),
            aliases: Self::built_in_aliases(),
        }
    }

    /// Returns the built-in alias mappings
    fn built_in_aliases() -> HashMap<String, String> {
        let mut aliases = HashMap::new();

        // Direction aliases
        aliases.insert("e".to_string(), "동".to_string());
        aliases.insert("w".to_string(), "서".to_string());
        aliases.insert("s".to_string(), "남".to_string());
        aliases.insert("n".to_string(), "북".to_string());
        aliases.insert("u".to_string(), "위".to_string());
        aliases.insert("d".to_string(), "아래".to_string());

        // Korean keyboard shortcuts for directions
        aliases.insert("ㄷ".to_string(), "동".to_string());
        aliases.insert("ㅅ".to_string(), "서".to_string());
        aliases.insert("ㄴ".to_string(), "남".to_string());
        aliases.insert("ㅂ".to_string(), "북".to_string());
        aliases.insert("ㅇ".to_string(), "위".to_string());
        aliases.insert("ㅁ".to_string(), "아래".to_string());

        // Command aliases (from objs/alias.py)
        aliases.insert("소".to_string(), "소지품".to_string());
        aliases.insert("소지".to_string(), "소지품".to_string());
        aliases.insert("보".to_string(), "봐".to_string());
        aliases.insert("look".to_string(), "봐".to_string());
        aliases.insert("바라보기".to_string(), "봐".to_string());
        aliases.insert("도".to_string(), "도망".to_string());
        aliases.insert("도움".to_string(), "도움말".to_string());
        aliases.insert("외".to_string(), "외쳐".to_string());
        aliases.insert("외침".to_string(), "외쳐".to_string());
        aliases.insert("잡".to_string(), "외쳐".to_string());
        aliases.insert("잡담".to_string(), "외쳐".to_string());
        aliases.insert(",".to_string(), "외쳐".to_string());
        aliases.insert("품".to_string(), "품목표".to_string());
        aliases.insert("품목".to_string(), "품목표".to_string());
        aliases.insert("판다".to_string(), "판매".to_string());
        aliases.insert("판".to_string(), "판매".to_string());
        aliases.insert("팔".to_string(), "판매".to_string());
        aliases.insert("팔다".to_string(), "판매".to_string());

        aliases
    }

    /// Registers a new command
    ///
    /// # Arguments
    /// * `info` - The CommandInfo to register
    pub fn register(&mut self, info: CommandInfo) {
        let name = info.name.clone();
        self.commands.insert(name.clone(), info);
    }

    /// Registers a command with a simple interface
    ///
    /// # Arguments
    /// * `name` - Command name
    /// * `handler` - Command handler function
    /// * `description` - Command description
    pub fn register_simple<F>(&mut self, name: &str, handler: F, description: &str)
    where
        F: Fn(&mut Body, &[&str]) -> CommandResult + Send + Sync + 'static,
    {
        let info = CommandInfo::simple(name.to_string(), Arc::new(handler), description);
        self.register(info);
    }

    /// Finds a command by name or alias
    ///
    /// # Arguments
    /// * `name` - The command name or alias to search for
    ///
    /// # Returns
    /// Option<&CommandInfo> if found
    pub fn get(&self, name: &str) -> Option<&CommandInfo> {
        // First check if it's a built-in alias (registry-level aliases)
        let resolved = if let Some(alias) = self.aliases.get(name) {
            alias
        } else {
            name
        };

        // Check by exact name match
        if let Some(cmd) = self.commands.get(resolved) {
            return Some(cmd);
        }

        // Also check CommandInfo.aliases for each command
        for cmd in self.commands.values() {
            if cmd.matches(name) {
                return Some(cmd);
            }
        }

        None
    }

    /// Finds and returns a mutable reference to a command
    pub fn get_mut(&mut self, name: &str) -> Option<&mut CommandInfo> {
        let resolved = if let Some(alias) = self.aliases.get(name) {
            alias.clone()
        } else {
            name.to_string()
        };

        self.commands.get_mut(&resolved)
    }

    /// Checks if a command exists
    ///
    /// # Arguments
    /// * `name` - The command name to check
    ///
    /// # Returns
    /// true if the command exists
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Returns all registered command names
    pub fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// Returns all commands
    pub fn all_commands(&self) -> Vec<&CommandInfo> {
        self.commands.values().collect()
    }

    /// Adds a custom alias
    ///
    /// # Arguments
    /// * `alias` - The alias to add
    /// * `command` - The command it maps to
    pub fn add_alias(&mut self, alias: String, command: String) {
        self.aliases.insert(alias, command);
    }

    /// Removes a command from the registry
    ///
    /// # Arguments
    /// * `name` - The command name to remove
    pub fn unregister(&mut self, name: &str) -> Option<CommandInfo> {
        self.commands.remove(name)
    }

    /// Resolves an alias to its command name
    ///
    /// # Arguments
    /// * `name` - The alias or command name
    ///
    /// # Returns
    /// The resolved command name
    pub fn resolve_alias(&self, name: &str) -> String {
        self.aliases.get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Checks if a player has permission to use a command
    ///
    /// # Arguments
    /// * `command_name` - The command to check
    /// * `player_level` - The player's permission level
    ///
    /// # Returns
    /// true if the player can use the command
    pub fn check_permission(&self, command_name: &str, player_level: i32) -> bool {
        if let Some(cmd) = self.get(command_name) {
            player_level >= cmd.level
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandResult;

    #[test]
    fn test_registry_new() {
        let registry = CommandRegistry::new();
        assert_eq!(registry.commands.len(), 0);
        assert!(registry.aliases.len() > 0);
    }

    #[test]
    fn test_register_command() {
        let mut registry = CommandRegistry::new();
        registry.register_simple("test", |_, _| CommandResult::Ok, "Test command");

        assert!(registry.contains("test"));
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn test_builtin_aliases() {
        let registry = CommandRegistry::new();

        // Test direction aliases
        assert_eq!(registry.resolve_alias("n"), "북");
        assert_eq!(registry.resolve_alias("s"), "남");
        assert_eq!(registry.resolve_alias("e"), "동");
        assert_eq!(registry.resolve_alias("w"), "서");
        assert_eq!(registry.resolve_alias("u"), "위");
        assert_eq!(registry.resolve_alias("d"), "아래");

        // Test Korean shortcuts
        assert_eq!(registry.resolve_alias("ㄷ"), "동");
        assert_eq!(registry.resolve_alias("ㅅ"), "서");
        assert_eq!(registry.resolve_alias("ㄴ"), "남");
        assert_eq!(registry.resolve_alias("ㅂ"), "북");
    }

    #[test]
    fn test_add_custom_alias() {
        let mut registry = CommandRegistry::new();
        registry.add_alias("cmd".to_string(), "command".to_string());

        assert_eq!(registry.resolve_alias("cmd"), "command");
    }

    #[test]
    fn test_unregister_command() {
        let mut registry = CommandRegistry::new();
        registry.register_simple("test", |_, _| CommandResult::Ok, "Test command");

        assert!(registry.contains("test"));

        let removed = registry.unregister("test");
        assert!(removed.is_some());
        assert!(!registry.contains("test"));
    }

    #[test]
    fn test_command_info_matches() {
        let handler = Arc::new(|_: &mut Body, _: &[&str]| CommandResult::Ok);
        let info = CommandInfo::new(
            "test".to_string(),
            vec!["t".to_string(), "tp".to_string()],
            handler,
            0,
            "Test".to_string(),
            "test [args]".to_string(),
        );

        assert!(info.matches("test"));
        assert!(info.matches("t"));
        assert!(info.matches("tp"));
        assert!(!info.matches("other"));
    }

    #[test]
    fn test_command_names() {
        let mut registry = CommandRegistry::new();
        registry.register_simple("cmd1", |_, _| CommandResult::Ok, "Command 1");
        registry.register_simple("cmd2", |_, _| CommandResult::Ok, "Command 2");

        let names = registry.command_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"cmd1".to_string()));
        assert!(names.contains(&"cmd2".to_string()));
    }

    #[test]
    fn test_check_permission() {
        let mut registry = CommandRegistry::new();
        registry.register_simple("public", |_, _| CommandResult::Ok, "Public");
        registry.register_simple("admin", |_, _| CommandResult::Ok, "Admin");
        registry.get_mut("admin").unwrap().level = 100;

        assert!(registry.check_permission("public", 0));
        assert!(registry.check_permission("public", 100));

        assert!(!registry.check_permission("admin", 0));
        assert!(registry.check_permission("admin", 100));
        assert!(registry.check_permission("admin", 200));
    }
}
