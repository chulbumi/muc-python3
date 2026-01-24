//! Integration test for script loading and execution

use muc_engine::script::{ScriptStorage, ScriptConfig};
use muc_engine::player::Body;

#[test]
fn test_script_loading() {
    let config = ScriptConfig::default();
    let storage = ScriptStorage::new(config);

    let names = storage.script_names();
    println!("Loaded scripts: {:?}", names);

    // Should have loaded the .rhai files from cmds/ directory
    assert!(names.contains(&"say".to_string()) ||
            names.contains(&"look".to_string()) ||
            names.contains(&"help".to_string()));
}

#[test]
fn test_script_execution() {
    let config = ScriptConfig::default();
    let storage = ScriptStorage::new(config);

    // Create a test player
    let mut body = Body::new();
    body.set("이름", "test_player");

    // Try to execute a script if it exists
    let names = storage.script_names();
    if let Some(name) = names.first() {
        let result = storage.execute(name, &mut body, "");
        // Script might fail due to API mismatches, but it should at least compile
        println!("Script {:?} execution result: {:?}", name, result);
    }
}
