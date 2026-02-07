//! Test example for the loader module
//!
//! Run with: cargo run --example test_loader

use muc_engine::loader::{load_json, load_script, save_json, ScriptValue, ScriptValueInner};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Loader Module\n");

    // Test 1: Load JSON file
    println!("=== Test 1: Loading JSON ===");
    match load_json("data/config/cmd.json").await {
        Ok(Some(data)) => {
            println!("Successfully loaded cmd.json");
            if let ScriptValueInner::Dict(segments) = &*data {
                println!("Found {} segments", segments.len());
                for key in segments.keys().take(3) {
                    println!("  - {}", key);
                }
            }
        }
        Ok(None) => println!("File not found"),
        Err(e) => println!("Error: {}", e),
    }

    // Test 2: Load CFG file
    println!("\n=== Test 2: Loading CFG ===");
    match load_script("data/config/cmd.cfg").await {
        Ok(Some(data)) => {
            println!("Successfully loaded cmd.cfg");
            if let ScriptValueInner::Dict(segments) = &*data {
                println!("Found {} segments", segments.len());
                for key in segments.keys().take(3) {
                    println!("  - {}", key);
                }
            }
        }
        Ok(None) => println!("File not found"),
        Err(e) => println!("Error: {}", e),
    }

    // Test 3: Create and save test data
    println!("\n=== Test 3: Creating and saving test data ===");
    let mut test_data = HashMap::new();
    test_data.insert(
        "test_key".to_string(),
        Box::new(ScriptValueInner::String("test_value".to_string())),
    );
    test_data.insert(
        "number_key".to_string(),
        Box::new(ScriptValueInner::Int(42)),
    );

    let mut test_segment = HashMap::new();
    test_segment.insert(
        "string_val".to_string(),
        Box::new(ScriptValueInner::String("hello".to_string())),
    );
    test_segment.insert("int_val".to_string(), Box::new(ScriptValueInner::Int(123)));

    let mut full_data = HashMap::new();
    full_data.insert(
        "segment1".to_string(),
        Box::new(ScriptValueInner::Dict(test_segment)),
    );
    full_data.insert(
        "segment2".to_string(),
        Box::new(ScriptValueInner::Dict(test_data)),
    );

    let script_value = Box::new(ScriptValueInner::Dict(full_data));

    // Save as JSON
    if let Err(e) = save_json("/tmp/test_output.json", &script_value).await {
        println!("Error saving JSON: {}", e);
    } else {
        println!("Saved test data to /tmp/test_output.json");
        // Read back and display
        if let Ok(content) = tokio::fs::read_to_string("/tmp/test_output.json").await {
            println!("Content:\n{}", content);
        }
    }

    // Save as CFG
    {
        use muc_engine::loader::save_script;
        let mut cfg_output = Vec::new();
        if let Err(e) = save_script(&mut cfg_output, &script_value) {
            println!("Error saving CFG: {}", e);
        } else {
            println!("\nSaved test data as CFG format:");
            println!("{}", String::from_utf8_lossy(&cfg_output));
        }
    }

    Ok(())
}
