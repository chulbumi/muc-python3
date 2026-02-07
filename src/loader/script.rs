//! CFG (Configuration) format parser and serializer
//!
//! CFG format:
//! ```text
//! [segment_name]
//! #key_name
//! :data
//! ;comment
//! ```
//!
//! - Lines starting with `[` and ending with `]` define segments
//! - Lines starting with `#` define keys
//! - Lines starting with `:` define data values for the current key
//! - Lines starting with `;` are comments and ignored
//! - Empty lines separate key-value groups

use crate::loader::{LoaderError, Result};
use std::collections::HashMap;
use std::path::Path;

/// Script value type - boxed to allow recursive structures
pub type ScriptValue = Box<ScriptValueInner>;

/// Inner value type for ScriptValue
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValueInner {
    String(String),
    Int(i64),
    Float(f64),
    List(Vec<ScriptValue>),
    Dict(HashMap<String, ScriptValue>),
}

impl ScriptValueInner {
    /// Check if this value is a dictionary
    pub fn is_dict(&self) -> bool {
        matches!(self, Self::Dict(_))
    }

    /// Check if this value is a list
    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    /// Get as dictionary reference
    pub fn as_dict(&self) -> Option<&HashMap<String, ScriptValue>> {
        match self {
            Self::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Get as list reference
    pub fn as_list(&self) -> Option<&Vec<ScriptValue>> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }

    /// Get as string reference
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

impl From<String> for ScriptValue {
    fn from(s: String) -> Self {
        Box::new(ScriptValueInner::String(s))
    }
}

impl From<&str> for ScriptValue {
    fn from(s: &str) -> Self {
        Box::new(ScriptValueInner::String(s.to_string()))
    }
}

impl From<i64> for ScriptValue {
    fn from(i: i64) -> Self {
        Box::new(ScriptValueInner::Int(i))
    }
}

impl From<f64> for ScriptValue {
    fn from(f: f64) -> Self {
        Box::new(ScriptValueInner::Float(f))
    }
}

impl From<Vec<ScriptValue>> for ScriptValue {
    fn from(v: Vec<ScriptValue>) -> Self {
        Box::new(ScriptValueInner::List(v))
    }
}

impl From<HashMap<String, ScriptValue>> for ScriptValue {
    fn from(m: HashMap<String, ScriptValue>) -> Self {
        Box::new(ScriptValueInner::Dict(m))
    }
}

/// Parser state for CFG format
struct Parser {
    segments: HashMap<String, ScriptValue>,
    current_segment: String,
    current_key: String,
    current_values: Vec<ScriptValue>,
    line_number: usize,
}

impl Parser {
    fn new() -> Self {
        Parser {
            segments: HashMap::new(),
            current_segment: String::new(),
            current_key: String::new(),
            current_values: Vec::new(),
            line_number: 0,
        }
    }

    /// Flush current key values to the segment
    fn flush_key(&mut self) {
        if !self.current_key.is_empty() && !self.current_values.is_empty() {
            if let Some(segment) = self.segments.get_mut(&self.current_segment) {
                if let ScriptValueInner::Dict(ref mut dict) = &mut **segment {
                    let value = if self.current_values.len() == 1 {
                        self.current_values.pop().unwrap()
                    } else {
                        Box::new(ScriptValueInner::List(self.current_values.clone()))
                    };
                    dict.insert(self.current_key.clone(), value);
                }
            }
            self.current_values.clear();
        }
    }

    /// Start a new segment
    fn start_segment(&mut self, name: &str) {
        self.flush_key();
        self.current_segment = name.to_string();
        self.current_key.clear();
        self.segments.insert(
            name.to_string(),
            Box::new(ScriptValueInner::Dict(HashMap::new())),
        );
    }

    /// Set the current key
    fn set_key(&mut self, key: &str) {
        self.flush_key();
        self.current_key = key.trim_start_matches('#').trim().to_string();
    }

    /// Add a data value for the current key
    fn add_value(&mut self, value: &str) {
        let trimmed = value.trim_start_matches(':').trim();
        if !trimmed.is_empty() {
            self.current_values
                .push(Box::new(ScriptValueInner::String(trimmed.to_string())));
        }
    }

    /// Finalize parsing and return the result
    fn finish(mut self) -> HashMap<String, ScriptValue> {
        self.flush_key();
        self.segments
    }

    /// Process a single line
    fn process_line(&mut self, line: &str) -> Result<()> {
        self.line_number += 1;
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            self.flush_key();
            return Ok(());
        }

        // Skip comments (but they also flush the current key)
        if trimmed.starts_with(';') {
            self.flush_key();
            return Ok(());
        }

        // Segment: [name]
        if let Some(name) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            self.start_segment(name);
            return Ok(());
        }

        // Key: #name
        if trimmed.starts_with('#') {
            self.set_key(trimmed);
            return Ok(());
        }

        // Data: :value
        if trimmed.starts_with(':') {
            // If no current segment, create a default one
            if self.current_segment.is_empty() {
                self.start_segment("default");
            }
            // If no current key, create one from line number or skip
            if self.current_key.is_empty() {
                self.current_key = format!("key_{}", self.line_number);
            }
            self.add_value(trimmed);
            return Ok(());
        }

        // Unknown line type - flush and continue
        self.flush_key();
        Ok(())
    }
}

/// Load a CFG script file and parse it into a dictionary structure
///
/// # Arguments
/// * `path` - Path to the CFG file to load
///
/// # Returns
/// * `Ok(Some(ScriptValue))` - Parsed data as a dictionary
/// * `Ok(None)` - File not found or empty
/// * `Err(LoaderError)` - Parse error
pub async fn load_script<P: AsRef<Path>>(path: P) -> Result<Option<ScriptValue>> {
    let path = path.as_ref();

    // Try to read the file
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(LoaderError::Io(e)),
    };

    // Parse the content
    let mut parser = Parser::new();

    for line in content.lines() {
        parser.process_line(line)?;
    }

    let segments = parser.finish();

    if segments.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Box::new(ScriptValueInner::Dict(segments))))
    }
}

/// Save data in CFG format to a writer
///
/// # Arguments
/// * `writer` - Writer to save the CFG data to
/// * `data` - Data to save (must be a dictionary)
pub fn save_script<W: std::io::Write>(writer: &mut W, data: &ScriptValue) -> Result<()> {
    match &**data {
        ScriptValueInner::Dict(segments) => {
            for (seg_name, seg_data) in segments {
                writeln!(writer, "[{}]", seg_name)?;

                if let ScriptValueInner::Dict(keys) = &**seg_data {
                    for (key_name, key_data) in keys {
                        // Skip keys starting with '_' (like Python version)
                        if key_name.starts_with('_') {
                            continue;
                        }

                        writeln!(writer, "#{}", key_name)?;

                        match &**key_data {
                            ScriptValueInner::String(s) => {
                                writeln!(writer, ":{}", s)?;
                            }
                            ScriptValueInner::Int(i) => {
                                writeln!(writer, ":{}", i)?;
                            }
                            ScriptValueInner::Float(f) => {
                                writeln!(writer, ":{}", f)?;
                            }
                            ScriptValueInner::List(items) => {
                                for item in items {
                                    match &**item {
                                        ScriptValueInner::String(s) => {
                                            writeln!(writer, ":{}", s)?;
                                        }
                                        ScriptValueInner::Int(i) => {
                                            writeln!(writer, ":{}", i)?;
                                        }
                                        ScriptValueInner::Float(f) => {
                                            writeln!(writer, ":{}", f)?;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                writeln!(writer)?;
            }
            Ok(())
        }
        _ => Err(LoaderError::InvalidData(
            "save_script requires a dictionary".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_simple_cfg() {
        let cfg = "\
[测试]
#key1
:value1
#key2
:value2
:value3
";
        let mut parser = Parser::new();
        for line in cfg.lines() {
            parser.process_line(line).unwrap();
        }
        let result = parser.finish();

        assert!(result.contains_key("测试"));
        let segment = result.get("测试").unwrap();
        if let ScriptValueInner::Dict(dict) = &**segment {
            assert_eq!(dict.len(), 2);
            assert!(dict.contains_key("key1"));
            assert!(dict.contains_key("key2"));

            // key1 has single value
            if let ScriptValueInner::String(s) = &**dict.get("key1").unwrap() {
                assert_eq!(s, "value1");
            } else {
                panic!("key1 should be a string");
            }

            // key2 has multiple values -> becomes a list
            if let ScriptValueInner::List(list) = &**dict.get("key2").unwrap() {
                assert_eq!(list.len(), 2);
            } else {
                panic!("key2 should be a list");
            }
        } else {
            panic!("测试 should be a dictionary");
        }
    }

    #[test]
    fn test_save_script() {
        let mut data = HashMap::new();
        let mut segment = HashMap::new();

        segment.insert(
            "key1".to_string(),
            Box::new(ScriptValueInner::String("value1".to_string())),
        );

        let mut list = Vec::new();
        list.push(Box::new(ScriptValueInner::String("value2".to_string())));
        list.push(Box::new(ScriptValueInner::String("value3".to_string())));
        segment.insert("key2".to_string(), Box::new(ScriptValueInner::List(list)));

        data.insert(
            "测试".to_string(),
            Box::new(ScriptValueInner::Dict(segment)),
        );

        let value = Box::new(ScriptValueInner::Dict(data));
        let mut output = Vec::new();
        save_script(&mut output, &value).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("[测试]"));
        assert!(result.contains("#key1"));
        assert!(result.contains(":value1"));
        assert!(result.contains("#key2"));
        assert!(result.contains(":value2"));
        assert!(result.contains(":value3"));
    }
}
