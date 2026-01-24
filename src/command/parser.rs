//! Command parser for MUD engine
//!
//! Provides parsing functionality for player input commands.
//! Handles tokenization, command extraction, and parameter parsing.

use crate::command::DIRECTION_ALIASES;

/// Result of parsing a command line
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    /// The raw input line
    pub raw: String,
    /// The main command/verb
    pub command: String,
    /// Arguments/parameters for the command
    pub args: String,
    /// Individual argument tokens
    pub tokens: Vec<String>,
}

impl ParsedCommand {
    /// Creates a new ParsedCommand
    pub fn new(raw: String, command: String, args: String) -> Self {
        let tokens = if args.is_empty() {
            Vec::new()
        } else {
            args.split_whitespace()
                .map(|s| s.to_string())
                .collect()
        };
        ParsedCommand { raw, command, args, tokens }
    }

    /// Creates an empty ParsedCommand
    pub fn empty() -> Self {
        ParsedCommand {
            raw: String::new(),
            command: String::new(),
            args: String::new(),
            tokens: Vec::new(),
        }
    }

    /// Returns true if the command is empty
    pub fn is_empty(&self) -> bool {
        self.command.is_empty() && self.args.is_empty()
    }
}

/// Command parser for MUD input
pub struct CommandParser;

impl CommandParser {
    /// Parses a line of input into a ParsedCommand
    ///
    /// # Arguments
    /// * `line` - The raw input line to parse
    ///
    /// # Returns
    /// A ParsedCommand containing the parsed components
    ///
    /// # Examples
    /// ```
    /// use muc_engine::command::parser::CommandParser;
    ///
    /// let parsed = CommandParser::parse("동 검을 주워");
    /// assert_eq!(parsed.command, "동");
    /// assert_eq!(parsed.args, "검을 주워");
    /// ```
    pub fn parse(line: &str) -> ParsedCommand {
        // Trim newlines but keep trailing spaces for say command detection
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        // Empty input
        if line.trim().is_empty() {
            return ParsedCommand::empty();
        }

        // Check if line ends with sentence-ending punctuation or space (treat as 'say' command)
        if line.ends_with(' ') || line.ends_with('.') || line.ends_with('!')
            || line.ends_with('?') || line.ends_with(',') {
            // In Python: last char triggers 'say' command
            // The command is the full line (except trailing punctuation), args are the message
            let cmd = line.trim_end_matches(|c| c == ' ' || c == '.' || c == '!' || c == '?' || c == ',');
            return ParsedCommand::new(line.to_string(), "말".to_string(), cmd.to_string());
        }

        // For non-say commands, trim whitespace and parse normally
        let line = line.trim();

        // Split into command (last word) and parameters (everything before)
        // Python: cmd = line.split(' ')[-1], param = line.rstrip(cmd).strip()
        let words: Vec<&str> = line.split_whitespace().collect();

        if words.is_empty() {
            return ParsedCommand::empty();
        }

        // In the Python code, the command is the LAST word and params are everything before
        // This is opposite of typical MUD parsing but matches the Python implementation
        let cmd = words[words.len() - 1];
        let param_start = line.find(cmd).unwrap_or(0);
        let mut param = if param_start > 0 {
            line[..param_start].trim().to_string()
        } else {
            String::new()
        };

        // Resolve direction aliases
        let resolved_cmd = Self::resolve_alias(cmd);

        // Check if this is a movement command - if so, filter out pickup-related keywords
        let is_direction = matches!(resolved_cmd.as_str(), "동" | "서" | "남" | "북" | "위" | "아래" | "북동" | "북서" | "남동" | "남서");
        let pickup_keywords = ["주워", "집어", "집", "가져"];

        let tokens = if param.is_empty() {
            Vec::new()
        } else if is_direction {
            // Filter out pickup keywords when followed by a direction
            param.split_whitespace()
                .filter(|s| !pickup_keywords.contains(s))
                .map(|s| s.to_string())
                .collect()
        } else {
            param.split_whitespace()
                .map(|s| s.to_string())
                .collect()
        };

        ParsedCommand {
            raw: line.to_string(),
            command: resolved_cmd,
            args: param,
            tokens,
        }
    }

    /// Resolves a command alias to its full form
    ///
    /// # Arguments
    /// * `cmd` - The command or alias to resolve
    ///
    /// # Returns
    /// The resolved command name
    pub fn resolve_alias(cmd: &str) -> String {
        for (alias, full) in DIRECTION_ALIASES {
            if cmd == *alias {
                return full.to_string();
            }
        }
        cmd.to_string()
    }

    /// Splits a string into name and order components
    ///
    /// Used for commands like "2.검" to get (name="검", order=2)
    ///
    /// # Arguments
    /// * `input` - The input string to parse
    ///
    /// # Returns
    /// A tuple of (name, order)
    ///
    /// # Examples
    /// ```
    /// use muc_engine::command::parser::CommandParser;
    ///
    /// let (name, order) = CommandParser::parse_name_order("2.검");
    /// assert_eq!(name, "검");
    /// assert_eq!(order, 2);
    /// ```
    pub fn parse_name_order(input: &str) -> (String, usize) {
        let input = input.trim();

        // Check for prefix number pattern like "2.검" or "2 검"
        if let Some(dot_pos) = input.find('.') {
            let prefix = &input[..dot_pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(order) = prefix.parse::<usize>() {
                    let name = input[dot_pos + 1..].trim().to_string();
                    if !name.is_empty() {
                        return (name, order);
                    }
                }
            }
        }

        // Check for pattern like "2 검"
        if let Some(space_pos) = input.find(' ') {
            let prefix = &input[..space_pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(order) = prefix.parse::<usize>() {
                    let name = input[space_pos + 1..].trim().to_string();
                    if !name.is_empty() {
                        return (name, order);
                    }
                }
            }
        }

        // No order specified, default to 1
        (input.to_string(), 1)
    }

    /// Checks if a line looks like a "say" command (ends with punctuation)
    ///
    /// # Arguments
    /// * `line` - The input line to check
    ///
    /// # Returns
    /// true if the line should be treated as speech
    pub fn is_say_command(line: &str) -> bool {
        // Don't trim first - we need to check the actual last character
        // But trim trailing newlines for user input
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        line.ends_with(' ') || line.ends_with('.') || line.ends_with('!')
            || line.ends_with('?') || line.ends_with(',')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let parsed = CommandParser::parse("");
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let parsed = CommandParser::parse("   ");
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_simple_command() {
        let parsed = CommandParser::parse("동");
        assert_eq!(parsed.command, "동");
        assert_eq!(parsed.args, "");
    }

    #[test]
    fn test_parse_command_with_args() {
        let parsed = CommandParser::parse("검을 주워 동");
        assert_eq!(parsed.command, "동");
        assert_eq!(parsed.args, "검을 주워");
    }

    #[test]
    fn test_parse_say_command() {
        let parsed = CommandParser::parse("안녕하세요.");
        assert_eq!(parsed.command, "말");
        assert_eq!(parsed.args, "안녕하세요");
    }

    #[test]
    fn test_parse_say_with_space() {
        let parsed = CommandParser::parse("안녕 ");
        assert_eq!(parsed.command, "말");
        assert_eq!(parsed.args, "안녕");
    }

    #[test]
    fn test_resolve_direction_alias() {
        assert_eq!(CommandParser::resolve_alias("n"), "북");
        assert_eq!(CommandParser::resolve_alias("s"), "남");
        assert_eq!(CommandParser::resolve_alias("e"), "동");
        assert_eq!(CommandParser::resolve_alias("w"), "서");
        assert_eq!(CommandParser::resolve_alias("u"), "위");
        assert_eq!(CommandParser::resolve_alias("d"), "아래");
        assert_eq!(CommandParser::resolve_alias("북"), "북");
    }

    #[test]
    fn test_parse_name_order_with_dot() {
        let (name, order) = CommandParser::parse_name_order("2.검");
        assert_eq!(name, "검");
        assert_eq!(order, 2);
    }

    #[test]
    fn test_parse_name_order_with_space() {
        let (name, order) = CommandParser::parse_name_order("3 검");
        assert_eq!(name, "검");
        assert_eq!(order, 3);
    }

    #[test]
    fn test_parse_name_order_default() {
        let (name, order) = CommandParser::parse_name_order("검");
        assert_eq!(name, "검");
        assert_eq!(order, 1);
    }

    #[test]
    fn test_is_say_command() {
        assert!(CommandParser::is_say_command("안녕."));
        assert!(CommandParser::is_say_command("안녕!"));
        assert!(CommandParser::is_say_command("안녕?"));
        assert!(CommandParser::is_say_command("안녕 "));
        assert!(CommandParser::is_say_command("안녕,"));
        assert!(!CommandParser::is_say_command("동"));
        assert!(!CommandParser::is_say_command("동쪽"));
    }

    #[test]
    fn test_parsed_command_tokens() {
        let parsed = CommandParser::parse("검 방패 주워 동");
        assert_eq!(parsed.command, "동");
        assert_eq!(parsed.tokens, vec!["검", "방패"]);
    }
}
