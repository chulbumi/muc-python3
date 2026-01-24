//! Data loader module for CFG and JSON formats
//!
//! This module provides functionality for loading and saving data in both:
//! - CFG format: A custom configuration format with segments, keys, and data
//! - JSON format: Standard JSON files

pub mod script;
pub mod json;

// Re-export main types and functions
pub use script::{load_script, save_script, ScriptValue, ScriptValueInner};
pub use json::{load_json, save_json};


/// Result type for loader operations
pub type Result<T> = std::result::Result<T, LoaderError>;

/// Errors that can occur during loading/saving operations
#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("Invalid data format: {0}")]
    InvalidData(String),
}

/// Helper function to convert a string to a number when possible
/// Preserves leading zeros in strings (like Python toNumber)
pub fn to_number(s: &str) -> ScriptValue {
    let trimmed = s.trim();

    // Check if it has a decimal point
    let has_dot = trimmed.contains('.');

    // Check if it starts with '0' and has more characters (preserve as string)
    if !has_dot && trimmed.starts_with('0') && trimmed.len() > 1 {
        return Box::new(ScriptValueInner::String(trimmed.to_string()));
    }

    // Try to parse as integer first (if no dot)
    if !has_dot {
        if let Ok(i) = trimmed.parse::<i64>() {
            return Box::new(ScriptValueInner::Int(i));
        }
    }

    // Try to parse as float
    if let Ok(f) = trimmed.parse::<f64>() {
        return Box::new(ScriptValueInner::Float(f));
    }

    // Return as string if parsing fails
    Box::new(ScriptValueInner::String(trimmed.to_string()))
}

/// Helper function to find a substring in a line, accounting for multi-byte characters
/// Similar to Python's find function that handles Korean characters
pub fn find(line: &str, word: &str) -> Option<usize> {
    line.find(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_number_int() {
        let result = to_number("123");
        assert!(matches!(*result, ScriptValueInner::Int(123)));
    }

    #[test]
    fn test_to_number_float() {
        let result = to_number("123.45");
        assert!(matches!(*result, ScriptValueInner::Float(f) if (f - 123.45).abs() < 0.001));
    }

    #[test]
    fn test_to_number_leading_zero() {
        let result = to_number("0123");
        assert!(matches!(*result, ScriptValueInner::String(s) if s == "0123"));
    }

    #[test]
    fn test_to_number_string() {
        let result = to_number("abc");
        assert!(matches!(*result, ScriptValueInner::String(s) if s == "abc"));
    }
}
