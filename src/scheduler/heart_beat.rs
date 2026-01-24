//! Heart Beat Registry - Per-Object Periodic Updates
//!
//! Implements LPMUD's heart_beat functionality.
//!
//! Allows objects to register for periodic updates:
//! ```rhai
//! set_heart_beat(true);  // Enable heart beat
//! set_heart_beat(false); // Disable heart beat
//! ```
//!
//! Each object with heart_beat enabled will have its heart_beat()
//! function called every tick (typically 1 second).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::player::Body;
use crate::script::ScriptStorage;

/// Heart beat configuration
#[derive(Debug, Clone)]
pub struct HeartBeatConfig {
    /// Tick interval (default: 1 second)
    pub tick_interval: Duration,
    /// Maximum number of heart beats allowed
    pub max_heart_beats: usize,
}

impl Default for HeartBeatConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(1),
            max_heart_beats: 10000,
        }
    }
}

/// Heart Beat Registry - Tracks objects with active heart beats
pub struct HeartBeatRegistry {
    /// Set of object IDs with active heart beats
    objects: HashSet<String>,
    /// Configuration
    config: HeartBeatConfig,
    /// Tick counter
    tick_count: u64,
}

impl HeartBeatRegistry {
    /// Create a new heart beat registry
    pub fn new(config: HeartBeatConfig) -> Self {
        Self {
            objects: HashSet::new(),
            config,
            tick_count: 0,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(HeartBeatConfig::default())
    }

    /// Enable heart beat for an object
    pub fn set_heart_beat(&mut self, object_id: &str, enabled: bool) -> bool {
        if enabled {
            if self.objects.len() >= self.config.max_heart_beats {
                warn!("Max heart beats reached, cannot enable for {}", object_id);
                return false;
            }
            debug!("Enabling heart beat for {}", object_id);
            self.objects.insert(object_id.to_string())
        } else {
            debug!("Disabling heart beat for {}", object_id);
            self.objects.remove(object_id)
        }
    }

    /// Check if an object has heart beat enabled
    pub fn has_heart_beat(&self, object_id: &str) -> bool {
        self.objects.contains(object_id)
    }

    /// Get all objects with active heart beats
    pub fn get_all(&self) -> Vec<String> {
        self.objects.iter().cloned().collect()
    }

    /// Get count of active heart beats
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Process all heart beats (called each tick)
    pub fn process_all(&mut self, storage: &ScriptStorage) -> HeartBeatResult {
        self.tick_count += 1;

        let object_ids: Vec<String> = self.objects.iter().cloned().collect();
        let mut result = HeartBeatResult {
            tick_count: self.tick_count,
            processed: 0,
            errors: Vec::new(),
        };

        for object_id in object_ids {
            match self.process_object(&object_id, storage) {
                Ok(_) => result.processed += 1,
                Err(e) => {
                    warn!("Heart beat error for {}: {}", object_id, e);
                    result.errors.push((object_id, e));
                }
            }
        }

        result
    }

    /// Process heart beat for a single object
    fn process_object(&self, object_id: &str, storage: &ScriptStorage) -> Result<(), String> {
        debug!("Processing heart beat for {}", object_id);

        // In a full implementation, we would:
        // 1. Load the object's script
        // 2. Call the heart_beat() function
        // 3. Handle any errors

        // For now, just verify the object exists
        if !storage.has_script(object_id) {
            // Object no longer exists, remove from registry
            return Err(format!("Object not found: {}", object_id));
        }

        Ok(())
    }

    /// Remove an object from the registry
    pub fn remove(&mut self, object_id: &str) -> bool {
        self.objects.remove(object_id)
    }

    /// Get the current tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }
}

/// Result of processing heart beats
#[derive(Debug)]
pub struct HeartBeatResult {
    pub tick_count: u64,
    pub processed: usize,
    pub errors: Vec<(String, String)>,
}

/// Heart Beat Manager - Coordinates heart beat processing
pub struct HeartBeatManager {
    registry: Arc<RwLock<HeartBeatRegistry>>,
    script_storage: Arc<RwLock<ScriptStorage>>,
}

impl HeartBeatManager {
    /// Create a new heart beat manager
    pub fn new(
        registry: Arc<RwLock<HeartBeatRegistry>>,
        script_storage: Arc<RwLock<ScriptStorage>>,
    ) -> Self {
        Self {
            registry,
            script_storage,
        }
    }

    /// Set heart beat for an object
    pub async fn set_heart_beat(&self, object_id: &str, enabled: bool) -> bool {
        let mut registry = self.registry.write().await;
        registry.set_heart_beat(object_id, enabled)
    }

    /// Check if an object has heart beat
    pub async fn has_heart_beat(&self, object_id: &str) -> bool {
        let registry = self.registry.read().await;
        registry.has_heart_beat(object_id)
    }

    /// Process all heart beats
    pub async fn process_all(&self) -> HeartBeatResult {
        let storage = self.script_storage.read().await;
        let mut registry = self.registry.write().await;
        registry.process_all(&storage)
    }

    /// Get count of active heart beats
    pub async fn count(&self) -> usize {
        let registry = self.registry.read().await;
        registry.len()
    }
}

/// Create a Rhai engine with heart_beat functions registered
pub fn create_heart_beat_engine(manager: Arc<HeartBeatManager>) -> rhai::Engine {
    let mut engine = rhai::Engine::new();

    // set_heart_beat(enabled)
    let manager_clone = manager.clone();
    engine.register_fn("set_heart_beat", move |object_id: &str, enabled: bool| -> bool {
        // Note: This is a synchronous wrapper around an async function
        // In practice, we'd need to handle this differently
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            handle.block_on(manager_clone.set_heart_beat(object_id, enabled))
        } else {
            false
        }
    });

    // has_heart_beat()
    let manager_clone = manager.clone();
    engine.register_fn("has_heart_beat", move |object_id: &str| -> bool {
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            handle.block_on(manager_clone.has_heart_beat(object_id))
        } else {
            false
        }
    });

    // query_heart_beat() - get count
    let manager_clone = manager.clone();
    engine.register_fn("query_heart_beat", move || -> i64 {
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            handle.block_on(manager_clone.count()) as i64
        } else {
            0
        }
    });

    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heart_beat_config_default() {
        let config = HeartBeatConfig::default();
        assert_eq!(config.tick_interval, Duration::from_secs(1));
        assert_eq!(config.max_heart_beats, 10000);
    }

    #[test]
    fn test_heart_beat_registry_new() {
        let registry = HeartBeatRegistry::new(HeartBeatConfig::default());
        assert!(registry.is_empty());
        assert_eq!(registry.tick_count(), 0);
    }

    #[test]
    fn test_heart_beat_registry_set() {
        let mut registry = HeartBeatRegistry::new(HeartBeatConfig::default());

        // Enable heart beat
        assert!(registry.set_heart_beat("obj1", true));
        assert!(registry.has_heart_beat("obj1"));
        assert_eq!(registry.len(), 1);

        // Disable heart beat
        assert!(registry.set_heart_beat("obj1", false));
        assert!(!registry.has_heart_beat("obj1"));
        assert!(registry.is_empty());
    }

    #[test]
    fn test_heart_beat_registry_get_all() {
        let mut registry = HeartBeatRegistry::new(HeartBeatConfig::default());

        registry.set_heart_beat("obj1", true);
        registry.set_heart_beat("obj2", true);
        registry.set_heart_beat("obj3", true);

        let all = registry.get_all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"obj1".to_string()));
        assert!(all.contains(&"obj2".to_string()));
        assert!(all.contains(&"obj3".to_string()));
    }

    #[test]
    fn test_heart_beat_registry_max() {
        let config = HeartBeatConfig {
            max_heart_beats: 2,
            ..Default::default()
        };
        let mut registry = HeartBeatRegistry::new(config);

        assert!(registry.set_heart_beat("obj1", true));
        assert!(registry.set_heart_beat("obj2", true));
        assert!(!registry.set_heart_beat("obj3", true)); // Exceeds max
    }

    #[test]
    fn test_heart_beat_registry_remove() {
        let mut registry = HeartBeatRegistry::new(HeartBeatConfig::default());

        registry.set_heart_beat("obj1", true);
        assert!(registry.remove("obj1"));
        assert!(!registry.has_heart_beat("obj1"));
        assert!(!registry.remove("obj1")); // Already removed
    }
}
