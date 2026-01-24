//! JSON format loader and saver
//!
//! Provides async functions to load and save JSON files,
//! with automatic conversion to/from ScriptValue types.

use crate::loader::{LoaderError, Result, ScriptValue, ScriptValueInner};
use serde_json::{Value as JsonValue};
use std::path::Path;

/// Load a JSON file and convert it to ScriptValue
///
/// # Arguments
/// * `path` - Path to the JSON file to load
///
/// # Returns
/// * `Ok(Some(ScriptValue))` - Parsed JSON data
/// * `Ok(None)` - File not found
/// * `Err(LoaderError)` - Parse or IO error
pub async fn load_json<P: AsRef<Path>>(path: P) -> Result<Option<ScriptValue>> {
    let path = path.as_ref();

    // Try to read the file
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(LoaderError::Io(e)),
    };

    // Parse JSON
    let json_value: JsonValue = serde_json::from_str(&content)?;
    Ok(Some(json_to_script(json_value)))
}

/// Convert a serde_json Value to ScriptValue
fn json_to_script(json: JsonValue) -> ScriptValue {
    match json {
        JsonValue::Null => Box::new(ScriptValueInner::String("null".to_string())),
        JsonValue::Bool(b) => Box::new(ScriptValueInner::Int(if b { 1 } else { 0 })),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(ScriptValueInner::Int(i))
            } else if let Some(f) = n.as_f64() {
                Box::new(ScriptValueInner::Float(f))
            } else {
                Box::new(ScriptValueInner::String(n.to_string()))
            }
        }
        JsonValue::String(s) => Box::new(ScriptValueInner::String(s)),
        JsonValue::Array(arr) => {
            let converted: Vec<ScriptValue> = arr.into_iter().map(json_to_script).collect();
            Box::new(ScriptValueInner::List(converted))
        }
        JsonValue::Object(obj) => {
            let converted: std::collections::HashMap<String, ScriptValue> = obj
                .into_iter()
                .map(|(k, v)| (k, json_to_script(v)))
                .collect();
            Box::new(ScriptValueInner::Dict(converted))
        }
    }
}

/// Save ScriptValue to a JSON file
///
/// # Arguments
/// * `path` - Path to save the JSON file
/// * `data` - Data to save
pub async fn save_json<P: AsRef<Path>>(path: P, data: &ScriptValue) -> Result<()> {
    let json_value = script_to_json(data);
    let content = serde_json::to_string_pretty(&json_value)?;
    tokio::fs::write(path.as_ref(), content).await?;
    Ok(())
}

/// Convert ScriptValue to serde_json Value
fn script_to_json(script: &ScriptValue) -> JsonValue {
    match &**script {
        ScriptValueInner::String(s) => JsonValue::String(s.clone()),
        ScriptValueInner::Int(i) => JsonValue::Number((*i).into()),
        ScriptValueInner::Float(f) => {
            serde_json::Number::from_f64(*f).map(JsonValue::Number).unwrap_or(JsonValue::Null)
        }
        ScriptValueInner::List(items) => {
            let arr: Vec<JsonValue> = items.iter().map(script_to_json).collect();
            JsonValue::Array(arr)
        }
        ScriptValueInner::Dict(map) => {
            let obj: serde_json::Map<String, JsonValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), script_to_json(v)))
                .collect();
            JsonValue::Object(obj)
        }
    }
}

/// Save ScriptValue to a JSON file with sorted keys (like Python's json.dump with sort_keys=True)
///
/// # Arguments
/// * `path` - Path to save the JSON file
/// * `data` - Data to save
pub async fn save_json_sorted<P: AsRef<Path>>(path: P, data: &ScriptValue) -> Result<()> {
    let json_value = script_to_json_sorted(data);
    let content = serde_json::to_string_pretty(&json_value)?;
    tokio::fs::write(path.as_ref(), content).await?;
    Ok(())
}

/// Convert ScriptValue to serde_json Value with sorted keys
fn script_to_json_sorted(script: &ScriptValue) -> JsonValue {
    match &**script {
        ScriptValueInner::String(s) => JsonValue::String(s.clone()),
        ScriptValueInner::Int(i) => JsonValue::Number((*i).into()),
        ScriptValueInner::Float(f) => {
            serde_json::Number::from_f64(*f).map(JsonValue::Number).unwrap_or(JsonValue::Null)
        }
        ScriptValueInner::List(items) => {
            let arr: Vec<JsonValue> = items.iter().map(script_to_json_sorted).collect();
            JsonValue::Array(arr)
        }
        ScriptValueInner::Dict(map) => {
            let mut sorted_keys: Vec<&String> = map.keys().collect();
            sorted_keys.sort();

            let mut obj = serde_json::Map::new();
            for key in sorted_keys {
                obj.insert(key.clone(), script_to_json_sorted(&map[key]));
            }
            JsonValue::Object(obj)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_json_round_trip() {
        let mut data = HashMap::new();
        data.insert("key1".to_string(), Box::new(ScriptValueInner::String("value1".to_string())));
        data.insert("key2".to_string(), Box::new(ScriptValueInner::Int(42)));

        let mut list = Vec::new();
        list.push(Box::new(ScriptValueInner::String("item1".to_string())));
        list.push(Box::new(ScriptValueInner::Int(123)));
        data.insert("list".to_string(), Box::new(ScriptValueInner::List(list)));

        let script_value = Box::new(ScriptValueInner::Dict(data));

        // Save to temp file
        let temp_path = "/tmp/test_loader.json";
        save_json(temp_path, &script_value).await.unwrap();

        // Load back
        let loaded = load_json(temp_path).await.unwrap().unwrap();

        // Verify
        if let ScriptValueInner::Dict(loaded_dict) = &*loaded {
            assert_eq!(loaded_dict.len(), 3);
            assert!(loaded_dict.contains_key("key1"));
            assert!(loaded_dict.contains_key("key2"));
            assert!(loaded_dict.contains_key("list"));
        } else {
            panic!("Loaded value should be a dict");
        }

        // Cleanup
        let _ = tokio::fs::remove_file(temp_path).await;
    }

    #[tokio::test]
    async fn test_json_not_found() {
        let result = load_json("/nonexistent/path/file.json").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_json_to_script_conversion() {
        let json_str = r#"{"string": "value", "number": 42, "float": 3.14, "array": [1, 2, 3]}"#;
        let json: JsonValue = serde_json::from_str(json_str).unwrap();
        let script = json_to_script(json);

        if let ScriptValueInner::Dict(dict) = &*script {
            assert_eq!(dict.len(), 4);

            if let ScriptValueInner::String(s) = &**dict.get("string").unwrap() {
                assert_eq!(s, "value");
            } else {
                panic!("string should be ScriptValueInner::String");
            }

            if let ScriptValueInner::Int(i) = &**dict.get("number").unwrap() {
                assert_eq!(*i, 42);
            } else {
                panic!("number should be ScriptValueInner::Int");
            }
        } else {
            panic!("script should be ScriptValueInner::Dict");
        }
    }

    #[test]
    fn test_script_to_json_conversion() {
        let mut data = HashMap::new();
        data.insert("key".to_string(), Box::new(ScriptValueInner::String("value".to_string())));
        let script = Box::new(ScriptValueInner::Dict(data));

        let json = script_to_json(&script);

        if let JsonValue::Object(obj) = json {
            assert_eq!(obj.len(), 1);
            assert!(obj.contains_key("key"));
            if let JsonValue::String(s) = &obj["key"] {
                assert_eq!(s, "value");
            } else {
                panic!("key should be a string");
            }
        } else {
            panic!("json should be an object");
        }
    }
}
