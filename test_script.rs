use muc_engine::script::{ScriptStorage, ScriptConfig};
use muc_engine::player::Body;

fn main() {
    let config = ScriptConfig::default();
    let storage = ScriptStorage::new(config);
    
    println!("Loaded scripts: {:?}", storage.script_names());
    
    // Test script execution
    let body = Body::new("test_player");
    if let Ok(_) = storage.execute("say", &body) {
        println!("Script executed successfully!");
    } else {
        println!("Script execution failed.");
    }
}
