//! String utility functions with Korean UTF-8 support
//!
//! Provides functions for:
//! - String searching with Korean character awareness (UTF-8 multi-byte)
//! - String to number conversion
//! - UTF-8 decode helpers (EUC-KR 미지원)

/// Represents a value that can be an integer, float, or string
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
}

impl Value {
    /// Returns true if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns true if the value is a number (int or float)
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Float(_))
    }

    /// Returns the string value if present
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        to_number(s)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        to_number(&s)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Value::Int(u as i64)
    }
}

impl From<u32> for Value {
    fn from(u: u32) -> Self {
        Value::Int(u as i64)
    }
}

impl From<usize> for Value {
    fn from(u: usize) -> Self {
        Value::Int(u as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

/// Returns the byte length of a UTF-8 character from its leading byte.
#[inline]
fn utf8_char_width(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

/// Find a word in a line, accounting for UTF-8 multi-byte characters (e.g. 한글)
///
/// # Arguments
/// * `line` - The text to search in (UTF-8)
/// * `word` - The word to search for
///
/// # Returns
/// The byte index where the word is found, or -1 if not found
///
/// # Examples
/// ```
/// use muc_engine::utils::l::find;
///
/// assert_eq!(find("hello world", "world"), 6);
/// assert_eq!(find("hello", "world"), -1);
/// ```
pub fn find(line: &str, word: &str) -> i32 {
    let line_bytes = line.as_bytes();
    let word_bytes = word.as_bytes();
    let word_len = word_bytes.len();
    let line_len = line_bytes.len();

    if word_len > line_len {
        return -1;
    }

    if word.is_empty() {
        return 0;
    }

    let count = line_len - word_len;
    let mut i = 0;

    while i <= count {
        if i + word_len <= line_len && &line_bytes[i..i + word_len] == word_bytes {
            return i as i32;
        }

        // Advance by UTF-8 character width (EUC-KR 미지원)
        let advance = utf8_char_width(line_bytes[i]).min(line_len - i).max(1);
        i += advance;
    }

    -1
}

/// Convert a string to a number, preserving strings that start with '0'
///
/// This function is similar to `to_number`, but strings starting with '0'
/// are returned as-is (as String variant). This is useful for handling
/// octal-like numbers or IDs that should not be converted.
///
/// # Arguments
/// * `s` - The string to convert
///
/// # Returns
/// * `Value::Int` - if the string represents an integer
/// * `Value::Float` - if the string represents a float
/// * `Value::String` - if conversion fails or string starts with '0' (and length > 1)
///
/// # Examples
/// ```
/// use muc_engine::utils::l::{tto_number, Value};
///
/// // String starting with '0' is preserved
/// assert_eq!(tto_number("0123"), Value::String("0123".to_string()));
///
/// // Normal numbers are converted
/// assert_eq!(tto_number("123"), Value::Int(123));
/// assert_eq!(tto_number("3.14"), Value::Float(3.14));
///
/// // Invalid numbers return as string
/// assert_eq!(tto_number("abc"), Value::String("abc".to_string()));
/// ```
pub fn tto_number(s: &str) -> Value {
    let trimmed = s.trim();

    // Preserve strings starting with '0' (and longer than 1 character)
    // But exclude floats like "0.5" or "-0.5"
    if trimmed.len() > 1 && trimmed.starts_with('0') && !trimmed.starts_with("0.") {
        return Value::String(trimmed.to_string());
    }

    // Try to parse as integer first
    if let Ok(i) = trimmed.parse::<i64>() {
        return Value::Int(i);
    }

    // Try to parse as float
    if let Ok(f) = trimmed.parse::<f64>() {
        return Value::Float(f);
    }

    // Return as string if parsing fails
    Value::String(trimmed.to_string())
}

/// Convert a string to a number
///
/// Attempts to parse the string as a number. If the string contains a decimal
/// point and can be parsed as a float, it returns a Float. If it's a whole
/// number, it returns an Int. Otherwise, returns the original string.
///
/// # Arguments
/// * `s` - The string to convert
///
/// # Returns
/// * `Value::Int` - if the string represents a whole number
/// * `Value::Float` - if the string represents a decimal number
/// * `Value::String` - if conversion fails
///
/// # Examples
/// ```
/// use muc_engine::utils::l::{to_number, Value};
///
/// // Integers
/// assert_eq!(to_number("123"), Value::Int(123));
/// assert_eq!(to_number("-456"), Value::Int(-456));
///
/// // Floats
/// assert_eq!(to_number("3.14"), Value::Float(3.14));
/// assert_eq!(to_number("-2.5"), Value::Float(-2.5));
///
/// // Strings that can't be parsed
/// assert_eq!(to_number("abc"), Value::String("abc".to_string()));
///
/// // Strings starting with '0' are still converted (unlike tto_number)
/// assert_eq!(to_number("0123"), Value::Int(123));
/// ```
pub fn to_number(s: &str) -> Value {
    let trimmed = s.trim();

    // Try to parse as float first (handles both int and float)
    if let Ok(f) = trimmed.parse::<f64>() {
        // Check if it's a whole number (no decimal point in original string)
        if !trimmed.contains('.') {
            if f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                return Value::Int(f as i64);
            }
        }
        return Value::Float(f);
    }

    // Return as string if parsing fails
    Value::String(trimmed.to_string())
}

/// UTF-8 문자열을 String으로 반환 (입력이 이미 UTF-8일 때)
///
/// # Arguments
/// * `s` - UTF-8 입력 (&str는 Rust에서 이미 UTF-8)
///
/// # Returns
/// A UTF-8 encoded String (EUC-KR 미지원)
pub fn to_uni(s: &str) -> String {
    s.to_string()
}

/// UTF-8 바이트를 String으로 디코딩
///
/// # Arguments
/// * `bytes` - UTF-8 encoded bytes
///
/// # Returns
/// A UTF-8 encoded String (손상된 바이트는 U+FFFD로 대체)
pub fn bytes_to_uni(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_basic() {
        assert_eq!(find("hello world", "world"), 6);
        assert_eq!(find("hello world", "hello"), 0);
        assert_eq!(find("hello world", "lo wo"), 3);
    }

    #[test]
    fn test_find_not_found() {
        assert_eq!(find("hello world", "xyz"), -1);
        assert_eq!(find("hello", "hello world"), -1);
    }

    #[test]
    fn test_find_empty_word() {
        assert_eq!(find("hello", ""), 0);
    }

    #[test]
    fn test_find_empty_line() {
        assert_eq!(find("", "hello"), -1);
        assert_eq!(find("", ""), 0);
    }

    #[test]
    fn test_find_word_longer_than_line() {
        assert_eq!(find("hi", "hello"), -1);
    }

    #[test]
    fn test_find_case_sensitive() {
        assert_eq!(find("Hello World", "hello"), -1);
        assert_eq!(find("Hello World", "Hello"), 0);
    }

    #[test]
    fn test_tto_number_preserve_zero_prefix() {
        // Strings starting with '0' (and length > 1) are preserved
        assert_eq!(tto_number("0123"), Value::String("0123".to_string()));
        assert_eq!(tto_number("007"), Value::String("007".to_string()));
        assert_eq!(tto_number("0"), Value::Int(0)); // Single "0" is converted
        assert_eq!(tto_number("01"), Value::String("01".to_string()));
    }

    #[test]
    fn test_tto_number_integers() {
        assert_eq!(tto_number("123"), Value::Int(123));
        assert_eq!(tto_number("-456"), Value::Int(-456));
        assert_eq!(tto_number("0"), Value::Int(0));
        assert_eq!(tto_number("999999"), Value::Int(999999));
    }

    #[test]
    fn test_tto_number_floats() {
        assert_eq!(tto_number("3.14"), Value::Float(3.14));
        assert_eq!(tto_number("-2.5"), Value::Float(-2.5));
        assert_eq!(tto_number("0.5"), Value::Float(0.5));
        assert_eq!(tto_number("123.456"), Value::Float(123.456));
    }

    #[test]
    fn test_tto_number_invalid() {
        assert_eq!(tto_number("abc"), Value::String("abc".to_string()));
        assert_eq!(tto_number("12abc"), Value::String("12abc".to_string()));
        assert_eq!(tto_number(""), Value::String("".to_string()));
    }

    #[test]
    fn test_tto_number_whitespace() {
        assert_eq!(tto_number("  123  "), Value::Int(123));
        assert_eq!(tto_number("  0123  "), Value::String("0123".to_string()));
        assert_eq!(tto_number("  3.14  "), Value::Float(3.14));
    }

    #[test]
    fn test_to_number_integers() {
        assert_eq!(to_number("123"), Value::Int(123));
        assert_eq!(to_number("-456"), Value::Int(-456));
        assert_eq!(to_number("0"), Value::Int(0));
        assert_eq!(to_number("999999"), Value::Int(999999));
        // Unlike tto_number, this converts zero-prefixed numbers
        assert_eq!(to_number("0123"), Value::Int(123));
        assert_eq!(to_number("007"), Value::Int(7));
    }

    #[test]
    fn test_to_number_floats() {
        assert_eq!(to_number("3.14"), Value::Float(3.14));
        assert_eq!(to_number("-2.5"), Value::Float(-2.5));
        assert_eq!(to_number("0.5"), Value::Float(0.5));
        assert_eq!(to_number("123.456"), Value::Float(123.456));
    }

    #[test]
    fn test_to_number_invalid() {
        assert_eq!(to_number("abc"), Value::String("abc".to_string()));
        assert_eq!(to_number("12abc"), Value::String("12abc".to_string()));
        assert_eq!(to_number(""), Value::String("".to_string()));
    }

    #[test]
    fn test_to_number_whitespace() {
        assert_eq!(to_number("  123  "), Value::Int(123));
        assert_eq!(to_number("  3.14  "), Value::Float(3.14));
    }

    #[test]
    fn test_to_uni_basic() {
        // Test with ASCII (should be unchanged)
        assert_eq!(to_uni("hello"), "hello");
        assert_eq!(to_uni("hello world"), "hello world");
    }

    #[test]
    fn test_to_uni_empty() {
        assert_eq!(to_uni(""), "");
    }

    #[test]
    fn test_bytes_to_uni_ascii() {
        assert_eq!(bytes_to_uni(b"hello"), "hello");
    }

    #[test]
    fn test_value_is_string() {
        assert!(Value::String("test".to_string()).is_string());
        assert!(!Value::Int(123).is_string());
        assert!(!Value::Float(3.14).is_string());
    }

    #[test]
    fn test_value_is_number() {
        assert!(!Value::String("test".to_string()).is_number());
        assert!(Value::Int(123).is_number());
        assert!(Value::Float(3.14).is_number());
    }

    #[test]
    fn test_value_as_str() {
        assert_eq!(Value::String("test".to_string()).as_str(), Some("test"));
        assert_eq!(Value::Int(123).as_str(), None);
        assert_eq!(Value::Float(3.14).as_str(), None);
    }

    #[test]
    fn test_value_from_str() {
        assert_eq!(Value::from("123"), Value::Int(123));
        assert_eq!(Value::from("3.14"), Value::Float(3.14));
        assert_eq!(Value::from("abc"), Value::String("abc".to_string()));
    }

    #[test]
    fn test_find_at_start() {
        assert_eq!(find("hello world", "hello"), 0);
        assert_eq!(find("test", "t"), 0);
    }

    #[test]
    fn test_find_at_end() {
        assert_eq!(find("hello world", "d"), 10);
        assert_eq!(find("hello", "lo"), 3);
    }

    #[test]
    fn test_find_overlap() {
        // The find function should find the first occurrence
        assert_eq!(find("aaa", "aa"), 0);
    }
}
