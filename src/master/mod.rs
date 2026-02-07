//! Master Object - Driver/Mudlib Coordination
//!
//! The Master Object is the central coordinator between the Rust driver
//! and the Rhai mudlib. It implements the LPMUD "applies" pattern.
//!
//! Based on MudOS/FluffOS architecture:
//! - https://www.fluffos.info/
//! - https://documentation.help/MudOS-v21c2-zh/chapter21.html

use rhai::{Dynamic, Engine, Scope};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::player::Body;
use crate::script::ScriptStorage;

/// Master object configuration
#[derive(Debug, Clone)]
pub struct MasterConfig {
    /// Path to the master script file
    pub master_script: String,
    /// Enable error logging to file
    pub log_errors: bool,
    /// Enable crash recovery
    pub enable_crash_recovery: bool,
}

impl Default for MasterConfig {
    fn default() -> Self {
        Self {
            master_script: "cmds/master.rhai".to_string(),
            log_errors: true,
            enable_crash_recovery: true,
        }
    }
}

/// Result type for master applies
pub type ApplyResult = Result<Option<Dynamic>, String>;

/// Master Object - Central coordinator for driver/mudlib communication
pub struct MasterObject {
    /// Script storage for loading master script
    script_storage: Arc<RwLock<ScriptStorage>>,
    /// Configuration
    config: MasterConfig,
    /// Cached master functions (for performance)
    cached_functions: Arc<RwLock<MasterFunctions>>,
}

/// Cached master function pointers
#[derive(Default)]
struct MasterFunctions {
    has_connect: bool,
    has_error_handler: bool,
    has_valid_compile: bool,
    has_crash: bool,
    has_create: bool,
    has_reset: bool,
    has_init: bool,
    has_move_or_destruct: bool,
}

impl MasterObject {
    /// Create a new master object
    pub fn new(script_storage: Arc<RwLock<ScriptStorage>>, config: MasterConfig) -> Self {
        let master = Self {
            script_storage,
            config,
            cached_functions: Arc::new(RwLock::new(MasterFunctions::default())),
        };

        // Cache available functions
        master.cache_functions();

        master
    }

    /// Create with default configuration
    pub fn default_storage(script_storage: Arc<RwLock<ScriptStorage>>) -> Self {
        Self::new(script_storage, MasterConfig::default())
    }

    /// Cache which functions are available in the master script
    fn cache_functions(&self) {
        let storage = self.script_storage.blocking_read();
        let has_script = storage.has_script(&self.script_name());

        if !has_script {
            warn!("Master script not found: {}", self.config.master_script);
            return;
        }

        // Check which functions exist by attempting to query them
        // For now, assume all exist if script is present
        let mut functions = self.cached_functions.blocking_write();
        functions.has_connect = true;
        functions.has_error_handler = true;
        functions.has_valid_compile = true;
        functions.has_crash = true;
        functions.has_create = true;
        functions.has_reset = true;
        functions.has_init = true;
        functions.has_move_or_destruct = true;

        info!("Master object loaded from {}", self.config.master_script);
    }

    /// Get the master script name (without extension)
    fn script_name(&self) -> String {
        self.config
            .master_script
            .trim_end_matches(".rhai")
            .to_string()
    }

    /// Execute a master function and return its result
    fn execute_apply(&self, func_name: &str, scope: &mut Scope) -> ApplyResult {
        let storage = self.script_storage.blocking_read();

        match storage.execute_with_scope(&self.script_name(), scope) {
            Ok(_) => {
                // Check if function returned a value
                if let Some(result) = scope.get_value(func_name) {
                    Ok(Some(result))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                error!("Master apply {} failed: {}", func_name, e);
                Err(format!("Master apply failed: {}", e))
            }
        }
    }

    /// CONNECT apply - Called when a new player connects
    ///
    /// Return value:
    /// - String: Login object path or error message
    /// - None: Use default login
    pub fn connect(&self, player_name: &str) -> ApplyResult {
        debug!("Master::connect() - {}", player_name);

        let mut scope = Scope::new();
        scope.push("player_name", player_name.to_string());

        // In full implementation, would call master.rhai's connect() function
        // For now, return None to use default login
        Ok(None)
    }

    /// ERROR_HANDLER apply - Called when an error occurs
    ///
    /// Return value:
    /// - true: Continue execution
    /// - false: Shutdown the driver
    pub fn error_handler(&self, error: &str, source: &str) -> Result<bool, String> {
        debug!("Master::error_handler() - {}: {}", source, error);

        let mut scope = Scope::new();
        scope.push("error", error.to_string());
        scope.push("source", source.to_string());

        // Log error
        error!("[ERROR] {} in {}", error, source);

        // In full implementation, would call master.rhai's error_handler() function
        // For now, always continue (don't shutdown)
        Ok(true)
    }

    /// VALID_COMPILE apply - Check if a file can be compiled
    ///
    /// Return value:
    /// - true: Allow compilation
    /// - false: Deny compilation
    pub fn valid_compile(&self, path: &str) -> bool {
        debug!("Master::valid_compile() - {}", path);

        // Security check: only allow compiling from certain directories
        if path.contains("..") {
            warn!("Security: Attempted to compile path with ..: {}", path);
            return false;
        }

        // Only allow cmds/ and lib/ directories
        if !path.starts_with("cmds/") && !path.starts_with("lib/") {
            warn!(
                "Security: Attempted to compile outside allowed dirs: {}",
                path
            );
            return false;
        }

        true
    }

    /// CRASH apply - Called when the driver crashes
    ///
    /// This is a last-resort handler before shutdown
    pub fn crash(&self, error: &str) {
        error!("!!! MASTER CRASH !!!");
        error!("Error: {}", error);

        // In full implementation, would call master.rhai's crash() function
        // Could attempt to save game state before shutdown

        if self.config.log_errors {
            // Write to crash log
            let log_path = "log/crash.log";
            if let Err(e) = std::fs::write(
                log_path,
                format!("Crash at {}: {}\n", chrono::Utc::now().to_rfc3339(), error),
            ) {
                error!("Failed to write crash log: {}", e);
            }
        }
    }

    /// CREATE apply - Called when an object is first created
    ///
    /// This is called for mudlib objects (not players)
    pub fn create(&self, object_path: &str, object_data: &mut rhai::Map) -> Result<(), String> {
        debug!("Master::create() - {}", object_path);

        let mut scope = Scope::new();
        scope.push("object_path", object_path.to_string());
        scope.push("object_data", object_data.clone());

        // In full implementation, would call object's create() function
        Ok(())
    }

    /// RESET apply - Called periodically to refresh objects
    ///
    /// This is called for rooms, mobs, items to regenerate them
    pub fn reset(&self, object_path: &str, _object_data: &mut rhai::Map) {
        debug!("Master::reset() - {}", object_path);

        // In full implementation, would call object's reset() function
        // Rooms use this to regenerate mobs/items
    }

    /// INIT apply - Called when two objects meet
    ///
    /// This is called when a living object encounters another object
    pub fn init(&self, object: &str, living: &Body) {
        debug!(
            "Master::init() - {} encountered by {}",
            object,
            living.get_name()
        );

        // In full implementation, would call object's init() function
        // This is where add_action() would be called to register commands
    }

    /// MOVE_OR_DESTRUCT apply - Called when an object is being moved
    ///
    /// Return value:
    /// - true: Allow the move
    /// - false: Destruct the object instead
    pub fn move_or_destruct(&self, object: &str, destination: &str) -> bool {
        debug!("Master::move_or_destruct() - {} to {}", object, destination);

        // In full implementation, would call object's move_or_destruct() function
        // Default: allow the move
        true
    }

    /// GET_BB_UID apply - Get the base UID for an object
    pub fn get_bb_uid(&self, _object: &str) -> Option<String> {
        Some("mudlib".to_string())
    }

    /// GET_ROOT_UID apply - Get the root UID
    pub fn get_root_uid(&self) -> String {
        "root".to_string()
    }

    /// AUTHOR_FILE apply - Get the author file for a domain
    pub fn author_file(&self, domain: &str) -> String {
        format!("domains/{}/AUTHOR", domain)
    }

    /// DOMAIN_FILE apply - Get the domain file for a path
    pub fn domain_file(&self, path: &str) -> Option<String> {
        // Extract domain from path
        if path.starts_with("domains/") {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() > 1 {
                return Some(format!("domains/{}", parts[1]));
            }
        }
        None
    }

    /// PRELOAD apply - Called during driver startup to preload objects
    pub fn preload(&self) -> Vec<String> {
        // Return list of objects to preload
        // In full implementation, would read from a configuration file
        vec!["cmds/도움말.rhai".to_string(), "cmds/저장.rhai".to_string()]
    }

    /// EPILOG apply - Called after all objects are loaded
    pub fn epilog(&self) -> Result<(), String> {
        info!("Master::epilog() - All objects loaded");
        Ok(())
    }
}

/// Create a Rhai engine with master object functions registered
pub fn create_master_engine() -> Engine {
    let mut engine = Engine::new();

    // Register master-specific functions for mudlib scripts

    // Get the master object
    engine.register_fn("master", |name: &str| -> String {
        format!("master:{}", name)
    });

    // Error logging
    engine.register_fn("log_error", |error: &str, source: &str| {
        error!("[MUDLIB] {} in {}", error, source);
    });

    // Debug logging
    engine.register_fn("log_debug", |msg: &str| {
        debug!("[MUDLIB] {}", msg);
    });

    // Info logging
    engine.register_fn("log_info", |msg: &str| {
        info!("[MUDLIB] {}", msg);
    });

    engine
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptConfig;

    #[test]
    fn test_master_config_default() {
        let config = MasterConfig::default();
        assert_eq!(config.master_script, "cmds/master.rhai");
        assert!(config.log_errors);
        assert!(config.enable_crash_recovery);
    }

    #[test]
    fn test_master_new() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
        let master = MasterObject::default_storage(storage);
        assert_eq!(master.config.master_script, "cmds/master.rhai");
    }

    #[test]
    fn test_valid_compile() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
        let master = MasterObject::default_storage(storage);

        // Valid paths
        assert!(master.valid_compile("cmds/test.rhai"));
        assert!(master.valid_compile("lib/test.rhai"));

        // Invalid paths
        assert!(!master.valid_compile("../etc/passwd"));
        assert!(!master.valid_compile("/etc/passwd"));
        assert!(!master.valid_compile("tmp/test.rhai"));
    }

    #[test]
    fn test_get_root_uid() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
        let master = MasterObject::default_storage(storage);
        assert_eq!(master.get_root_uid(), "root");
    }

    #[test]
    fn test_author_file() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
        let master = MasterObject::default_storage(storage);
        assert_eq!(master.author_file("test"), "domains/test/AUTHOR");
    }

    #[test]
    fn test_domain_file() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
        let master = MasterObject::default_storage(storage);

        assert_eq!(
            master.domain_file("domains/test/room"),
            Some("domains/test".to_string())
        );
        assert_eq!(master.domain_file("cmds/test"), None);
    }

    #[test]
    fn test_move_or_destruct() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
        let master = MasterObject::default_storage(storage);

        // Default: allow move
        assert!(master.move_or_destruct("object1", "room1"));
    }

    #[test]
    fn test_preload() {
        let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
        let master = MasterObject::default_storage(storage);

        let preload = master.preload();
        assert!(!preload.is_empty());
        assert!(preload.iter().any(|p| p.contains("도움말")));
    }
}
