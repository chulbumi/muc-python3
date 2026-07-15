//! Command registry for MUD engine
//!
//! Provides command registration, lookup, and management functionality.
//! Supports aliases, permission levels, and command metadata.

use crate::command::CommandResult;
use crate::player::Body;
use std::collections::HashMap;
use std::sync::Arc;

/// Function type for command handlers
///
/// Commands take a mutable reference to a player and a slice of argument strings
pub type CommandFn = Arc<dyn Fn(&mut Body, &[&str]) -> CommandResult + Send + Sync>;

/// Global command aliases used by the Python runtime.
///
/// `Player.parse_command()` imports only `objs.alias.alias` before looking up
/// `Player.cmdList`; `data/config/cmd.json` is not part of that execution
/// path. Keep this table identical to `objs/alias.py` and let exact command
/// filenames register under their own names.
pub(crate) const PYTHON_RUNTIME_ALIASES: &[(&str, &str)] = &[
    ("e", "동"),
    ("w", "서"),
    ("s", "남"),
    ("n", "북"),
    ("u", "위"),
    ("d", "아래"),
    ("ne", "북동"),
    ("nw", "북서"),
    ("se", "남동"),
    ("sw", "남서"),
    ("ㄷ", "동"),
    ("ㅅ", "서"),
    ("ㄴ", "남"),
    ("ㅂ", "북"),
    ("ㅇ", "위"),
    ("ㅁ", "아래"),
    ("해", "벗어"),
    ("벗", "벗어"),
    ("어", "어디"),
    ("해제", "벗어"),
    ("입", "입어"),
    ("착", "입어"),
    ("착용", "입어"),
    ("무장", "입어"),
    ("점", "점수"),
    ("소", "소지품"),
    ("누", "누구"),
    ("뒤", "뒤져"),
    ("소지", "소지품"),
    ("주워", "가져"),
    ("장", "장비"),
    ("업", "업데이트"),
    ("보", "봐"),
    ("귀", "귀환"),
    ("일", "일어나"),
    ("일어", "일어나"),
    ("일어서", "일어나"),
    ("일어난다", "일어나"),
    ("쉬", "쉬어"),
    ("쉰다", "쉬어"),
    ("자무", "자동무공"),
    ("시", "시전"),
    ("공지", "공지사항"),
    ("자무삭제", "자동무공삭제"),
    ("도움", "도움말"),
    ("표", "표현"),
    ("'", "표현"),
    ("설", "설정"),
    ("외", "외쳐"),
    ("잡", "외쳐"),
    ("잡담", "외쳐"),
    (",", "외쳐"),
    ("외침", "외쳐"),
    ("전", "전음"),
    ("/", "전음"),
    ("줄", "줄임말"),
    ("줄임", "줄임말"),
    ("품", "품목표"),
    ("품목", "품목표"),
    ("구", "구입"),
    ("구매", "구입"),
    ("사", "구입"),
    ("산다", "구입"),
    ("판다", "판매"),
    ("판", "판매"),
    ("팔", "판매"),
    ("팔다", "판매"),
    ("먹", "먹어"),
    ("먹는다", "먹어"),
    ("숙", "숙련도"),
    ("숙련", "숙련도"),
    ("기술", "무공"),
    ("무상", "무공상태"),
    ("값", "값설정"),
    ("도", "도망"),
    ("비", "비교"),
    ("상보", "상태보기"),
    ("꺼", "꺼내"),
    ("넣", "넣어"),
    ("상", "점수"),
    ("상태", "점수"),
    ("기상", "무공상태"),
    ("몹", "몹찾기"),
    ("아이템", "아이템찾기"),
    (":", "반전음"),
    ("방리", "방파리스트"),
    ("방상", "방파상태"),
    ("]", "방파말"),
    ("방", "방파말"),
    ("너", "넣어"),
    ("따", "따라"),
    ("집", "가져"),
    ("집어", "가져"),
    ("부숴", "부셔"),
    ("부수", "부셔"),
    ("부수다", "부셔"),
    ("귀가", "귀환"),
    ("정", "점수"),
    ("정보", "점수"),
    ("능력치", "점수"),
    ("반전", "반전음"),
    ("반", "반전음"),
    ("공", "쳐"),
    ("때", "쳐"),
    ("공격", "쳐"),
    ("때려", "쳐"),
    ("버", "버려"),
    ("던져", "투척"),
    ("날려", "투척"),
    ("[", "채널잡담"),
];

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
    /// Engine-only handlers.  These are intentionally absent from command
    /// lookup, aliases and command listings; Python invokes the corresponding
    /// parse-command branches without adding player commands.
    internal_handlers: HashMap<String, CommandFn>,
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
            internal_handlers: HashMap::new(),
            aliases: Self::built_in_aliases(),
        }
    }

    /// Returns the built-in alias mappings
    fn built_in_aliases() -> HashMap<String, String> {
        PYTHON_RUNTIME_ALIASES
            .iter()
            .map(|(alias, command)| ((*alias).to_string(), (*command).to_string()))
            .collect()
    }

    /// Registers a new command
    ///
    /// # Arguments
    /// * `info` - The CommandInfo to register
    pub fn register(&mut self, info: CommandInfo) {
        let name = info.name.clone();
        self.commands.insert(name.clone(), info);
    }

    /// Register a private parse-command handler.
    pub fn register_internal(&mut self, name: impl Into<String>, handler: CommandFn) {
        self.internal_handlers.insert(name.into(), handler);
    }

    /// Look up a private parse-command handler without exposing it as a
    /// player command.
    pub fn get_internal(&self, name: &str) -> Option<&CommandFn> {
        self.internal_handlers.get(name)
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
        self.commands.values().find(|cmd| cmd.matches(name))
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
        self.aliases
            .get(name)
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

    fn python_single_quoted_string(value: &str) -> Option<String> {
        let value = value.strip_prefix('\'')?.strip_suffix('\'')?;
        let mut decoded = String::new();
        let mut chars = value.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                decoded.push(chars.next()?);
            } else {
                decoded.push(ch);
            }
        }
        Some(decoded)
    }

    fn aliases_from_python_source(source: &str) -> HashMap<String, String> {
        source
            .lines()
            .filter_map(|line| {
                let entry = line.trim().strip_suffix(',')?;
                let (alias, command) = entry.split_once(": ")?;
                Some((
                    python_single_quoted_string(alias)?,
                    python_single_quoted_string(command)?,
                ))
            })
            .collect()
    }

    #[test]
    fn test_registry_new() {
        let registry = CommandRegistry::new();
        assert!(registry.commands.is_empty());
        assert!(!registry.aliases.is_empty());
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
    fn runtime_aliases_exactly_match_objs_alias_py() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("objs/alias.py"),
        )
        .expect("objs/alias.py must be readable");
        let expected = aliases_from_python_source(&source);
        let registry = CommandRegistry::new();

        assert_eq!(registry.aliases, expected);
        assert_eq!(registry.aliases.len(), 110);

        for invented in [
            "/h",
            "?",
            "look",
            "score",
            "stat",
            "바라보기",
            "창",
            "창룡",
            "창룡후",
            "외친다",
        ] {
            assert_eq!(registry.resolve_alias(invented), invented);
        }
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
