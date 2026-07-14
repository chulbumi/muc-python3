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
            args.split_whitespace().map(|s| s.to_string()).collect()
        };
        ParsedCommand {
            raw,
            command,
            args,
            tokens,
        }
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
    /// Python `lib.func.stripANSI`의 입력 정규화 동작.
    ///
    /// 일반적인 ANSI 파서와 일부러 다르다. 원본은 ESC를 본 뒤 첫 `m`까지
    /// 버리고, C1 CSI(155)는 그 바이트만 버리며, 백스페이스는 이미 누적된
    /// 마지막 문자를 하나 지운다. 명령 해석 결과를 바꾸지 않도록 이 동작을
    /// 그대로 보존한다.
    pub fn strip_python_ansi(line: &str) -> String {
        let mut found_escape = false;
        let mut output = String::new();

        for character in line.chars() {
            match character {
                '\u{009b}' => continue,
                '\u{0008}' => {
                    output.pop();
                    continue;
                }
                '\u{001b}' => {
                    found_escape = true;
                    continue;
                }
                'm' if found_escape => {
                    found_escape = false;
                    continue;
                }
                _ if found_escape => continue,
                _ => output.push(character),
            }
        }
        output
    }

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
    /// // Command is the last word ("주워" -> resolved to itself)
    /// assert_eq!(parsed.command, "주워");
    /// // Args are everything before the command
    /// assert_eq!(parsed.args, "동 검을");
    /// ```
    pub fn parse(line: &str) -> ParsedCommand {
        let sanitized = Self::strip_python_ansi(line);
        // Whitespace only separates tokens.  Surrounding whitespace must not
        // turn an otherwise valid one-word command into speech.
        let line = sanitized
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .trim();

        // Empty input
        if line.trim().is_empty() {
            return ParsedCommand::empty();
        }

        // Check if line ends with sentence-ending punctuation or space (treat as 'say' command)
        if line.ends_with('.') || line.ends_with('!') || line.ends_with('?') {
            // Python passes the entire original line to cmds/말.py.  Sentence
            // punctuation and a trailing space are part of the spoken message.
            return ParsedCommand::new(line.to_string(), "말".to_string(), line.to_string());
        }

        // Split into command (last word) and parameters (everything before)
        // Python: cmd = line.split(' ')[-1], param = line.rstrip(cmd).strip()
        let words: Vec<&str> = line.split_whitespace().collect();

        if words.is_empty() {
            return ParsedCommand::empty();
        }

        // In the Python code, the command is the LAST word and params are everything before
        // This is opposite of typical MUD parsing but matches the Python implementation
        let cmd = words[words.len() - 1];
        // Python: `param = line.rstrip(cmd).strip()`.  `rstrip` receives the
        // command as a character set, but the separating whitespace stops it
        // after the final command token.  In particular, an earlier occurrence
        // of the command inside a target name must not truncate the arguments.
        let param = line
            .trim_end_matches(|character| cmd.contains(character))
            .trim()
            .to_string();

        // Resolve direction aliases
        let resolved_cmd = Self::resolve_alias(cmd);

        // Check if this is a movement command - if so, filter out pickup-related keywords
        let is_direction = matches!(
            resolved_cmd.as_str(),
            "동" | "서" | "남" | "북" | "위" | "아래" | "북동" | "북서" | "남동" | "남서"
        );
        let pickup_keywords = ["주워", "집어", "집", "가져"];

        let tokens = if param.is_empty() {
            Vec::new()
        } else if is_direction {
            // Filter out pickup keywords when followed by a direction
            param
                .split_whitespace()
                .filter(|s| !pickup_keywords.contains(s))
                .map(|s| s.to_string())
                .collect()
        } else {
            param.split_whitespace().map(|s| s.to_string()).collect()
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
    /// Python `getNameOrder`, e.g. "2검" -> (name="검", order=2)
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
    /// let (name, order) = CommandParser::parse_name_order("2검");
    /// assert_eq!(name, "검");
    /// assert_eq!(order, 2);
    /// ```
    pub fn parse_name_order(input: &str) -> (String, usize) {
        let input = input.trim();
        // Python getNameOrder(): getInt accepts a leading run of digits and,
        // when nonzero, removes only those digit characters. Separators are
        // not syntax: `2.검` becomes `.검`, `2 검` becomes ` 검`.
        let digit_end = input
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_digit())
            .map(|(index, character)| index + character.len_utf8())
            .last()
            .unwrap_or(0);
        let order = if digit_end > 0 {
            input[..digit_end].parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        if order == 0 {
            (input.to_string(), 1)
        } else if digit_end < input.len() {
            (input[digit_end..].to_string(), order)
        } else {
            // With a pure number Python's stripping loop finds no non-digit,
            // so the numeric name remains for Room.findObjName.
            (input.to_string(), order)
        }
    }

    /// Checks if a line looks like a "say" command (ends with punctuation)
    ///
    /// # Arguments
    /// * `line` - The input line to check
    ///
    /// # Returns
    /// true if the line should be treated as speech
    pub fn is_say_command(line: &str) -> bool {
        let sanitized = Self::strip_python_ansi(line);
        let line = sanitized
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .trim();
        line.ends_with('.') || line.ends_with('!') || line.ends_with('?')
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
    fn python_ansi_and_backspace_are_removed_before_command_detection() {
        assert_eq!(
            CommandParser::strip_python_ansi("\x1b[31m안\x08녕 말\x1b[0m"),
            "녕 말"
        );
        let parsed = CommandParser::parse("\x1b[31m안녕\x1b[0m 말");
        assert_eq!(parsed.raw, "안녕 말");
        assert_eq!(parsed.command, "말");
        assert_eq!(parsed.args, "안녕");
    }

    #[test]
    fn python_strip_ansi_preserves_its_nonstandard_c1_and_unclosed_escape_rules() {
        // Python drops codepoint 155 itself without entering escape mode.
        assert_eq!(CommandParser::strip_python_ansi("\u{009b}31m북"), "31m북");
        // An ESC without a later `m` consumes the rest of the input.
        assert_eq!(CommandParser::strip_python_ansi("북\x1b[31K남"), "북");
    }

    #[test]
    fn test_parse_simple_command() {
        let parsed = CommandParser::parse("동");
        assert_eq!(parsed.command, "동");
        assert_eq!(parsed.args, "");
    }

    #[test]
    fn dot_before_attack_is_the_python_first_mob_selector() {
        let parsed = CommandParser::parse(". 쳐");
        assert_eq!(parsed.command, "쳐");
        assert_eq!(parsed.args, ".");
        assert_eq!(parsed.tokens, ["."]);
    }

    #[test]
    fn test_parse_command_with_args() {
        let parsed = CommandParser::parse("검을 주워 동");
        assert_eq!(parsed.command, "동");
        assert_eq!(parsed.args, "검을 주워");
    }

    #[test]
    fn test_parse_uses_the_final_command_token_not_an_earlier_substring() {
        let parsed = CommandParser::parse("쪽지왕  여러 단어 제목 쪽지");
        assert_eq!(parsed.command, "쪽지");
        assert_eq!(parsed.args, "쪽지왕  여러 단어 제목");
    }

    #[test]
    fn test_parse_say_command() {
        let parsed = CommandParser::parse("안녕하세요.");
        assert_eq!(parsed.command, "말");
        assert_eq!(parsed.args, "안녕하세요.");
    }

    #[test]
    fn surrounding_and_repeated_whitespace_only_separates_tokens() {
        let parsed = CommandParser::parse(" 힘  ");
        assert_eq!(parsed.command, "힘");
        assert_eq!(parsed.args, "");

        let parsed = CommandParser::parse("  대상   힘  ");
        assert_eq!(parsed.command, "힘");
        assert_eq!(parsed.args, "대상");
        assert_eq!(parsed.tokens, vec!["대상"]);
    }

    #[test]
    fn test_parse_single_period_as_spoken_message() {
        let parsed = CommandParser::parse(".");
        assert_eq!(parsed.command, "말");
        assert_eq!(parsed.args, ".");
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
        assert_eq!(name, ".검");
        assert_eq!(order, 2);
    }

    #[test]
    fn test_parse_name_order_with_space() {
        let (name, order) = CommandParser::parse_name_order("3 검");
        assert_eq!(name, " 검");
        assert_eq!(order, 3);
    }

    #[test]
    fn test_parse_name_order_matches_python_bare_numeric_prefix() {
        assert_eq!(
            CommandParser::parse_name_order("12검"),
            ("검".to_string(), 12)
        );
        assert_eq!(
            CommandParser::parse_name_order("0검"),
            ("0검".to_string(), 1)
        );
        assert_eq!(CommandParser::parse_name_order("2"), ("2".to_string(), 2));
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
        assert!(!CommandParser::is_say_command("안녕 "));
        assert!(!CommandParser::is_say_command("안녕,"));
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
