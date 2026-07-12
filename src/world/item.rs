//! Item module for MUD world
//!
//! This module provides item loading and management functionality.
//! Items are loaded from JSON files in the data/item/ directory.

use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

static RUNTIME_DELETED_ITEMS: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

fn runtime_deleted_items() -> &'static RwLock<HashSet<String>> {
    RUNTIME_DELETED_ITEMS.get_or_init(|| RwLock::new(HashSet::new()))
}

pub(crate) fn is_runtime_deleted(key: &str) -> bool {
    runtime_deleted_items()
        .read()
        .is_ok_and(|items| items.contains(key))
}

fn mark_runtime_deleted(key: &str) {
    if let Ok(mut items) = runtime_deleted_items().write() {
        items.insert(key.to_string());
    }
}

fn clear_runtime_deleted(key: &str) {
    if let Ok(mut items) = runtime_deleted_items().write() {
        items.remove(key);
    }
}

/// Raw item data from JSON
#[derive(Debug, Clone)]
pub struct RawItemData {
    /// Item name (이름)
    pub name: String,
    /// Item type (종류)
    pub item_type: String,
    /// Subtype (세부종류)
    pub subtype: String,
    /// Description (설명)
    pub description: Vec<String>,
    /// Reaction names (반응이름) - aliases for this item
    pub reaction_names: Vec<String>,
    /// Price (값)
    pub price: i64,
    /// Weight (무게)
    pub weight: i64,
    /// Level requirement (레벨제한)
    pub level_req: i64,
    /// Defense (방어력)
    pub defense: i64,
    /// Damage (공격력)
    pub damage: i64,
    /// Extra damage (추가타격)
    pub extra_damage: i64,
    /// Durability (내구도)
    pub durability: i64,
    /// Max durability (최대내구도)
    pub max_durability: i64,
    /// Equipment slot (장착부위) - weapon, armor, helmet, etc.
    pub equip_slot: String,
    /// Magic options (마법옵션)
    pub magic_options: Vec<(String, i64)>,
    /// Special flags
    pub flags: Vec<String>,
    /// Can be dropped
    pub droppable: bool,
    /// Can be traded
    pub tradable: bool,
    /// Can be sold to shop
    pub sellable: bool,
    /// Is consumable (먹는것)
    pub consumable: bool,
    /// Is equipment (장비)
    pub is_equipment: bool,
    /// Is money (돈)
    pub is_money: bool,
    /// Effect when used (사용효과)
    pub use_effect: Option<String>,
    /// Skill learned from this item (배울무공)
    pub learn_skill: Option<String>,
}

impl RawItemData {
    /// Create empty item data
    pub fn new() -> Self {
        Self {
            name: String::new(),
            item_type: "기타".to_string(),
            subtype: String::new(),
            description: Vec::new(),
            reaction_names: Vec::new(),
            price: 0,
            weight: 0,
            level_req: 0,
            defense: 0,
            damage: 0,
            extra_damage: 0,
            durability: 1000,
            max_durability: 1000,
            equip_slot: String::new(),
            magic_options: Vec::new(),
            flags: Vec::new(),
            droppable: true,
            tradable: true,
            sellable: true,
            consumable: false,
            is_equipment: false,
            is_money: false,
            use_effect: None,
            learn_skill: None,
        }
    }
}

impl Default for RawItemData {
    fn default() -> Self {
        Self::new()
    }
}

/// Active item instance in the game world
#[derive(Debug, Clone)]
pub struct ItemInstance {
    /// Original item key (filename)
    pub item_key: String,
    /// Instance name (might be customized)
    pub name: String,
    /// Current durability
    pub durability: i64,
    /// Owner (player name or None if on ground)
    pub owner: Option<String>,
    /// Location (zone:room or None if carried)
    pub location: Option<String>,
    /// Is equipped
    pub equipped: bool,
    /// Enchantment level
    pub enchant: i32,
    /// Custom flags
    pub flags: Vec<String>,
    /// Usage count
    pub usage_count: i32,
}

impl ItemInstance {
    /// Create a new item instance
    pub fn new(item_key: String, name: String) -> Self {
        Self {
            item_key,
            name,
            durability: 1000,
            owner: None,
            location: None,
            equipped: false,
            enchant: 0,
            flags: Vec::new(),
            usage_count: 0,
        }
    }

    /// Create item instance from raw data
    pub fn from_data(data: &RawItemData) -> Self {
        Self {
            item_key: data.name.clone(),
            name: data.name.clone(),
            durability: data.max_durability,
            owner: None,
            location: None,
            equipped: false,
            enchant: 0,
            flags: data.flags.clone(),
            usage_count: 0,
        }
    }

    /// Check if item is broken
    pub fn is_broken(&self) -> bool {
        self.durability <= 0
    }

    /// Use the item (decrease durability or consume)
    pub fn use_item(&mut self, amount: i32) -> bool {
        if self.is_broken() {
            return false;
        }
        self.durability -= amount as i64;
        self.usage_count += 1;
        true
    }

    /// Repair the item
    pub fn repair(&mut self, amount: i64) {
        self.durability = (self.durability + amount).min(1000);
    }

    /// Get display name with enchantment
    pub fn get_display_name(&self) -> String {
        if self.enchant > 0 {
            format!("+{} {}", self.enchant, self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Item cache for storing loaded item templates
#[derive(Debug)]
pub struct ItemCache {
    /// Cached item data indexed by filename
    items: HashMap<String, RawItemData>,
    /// Data directory path
    data_dir: PathBuf,
}

impl ItemCache {
    /// Create a new item cache
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            data_dir: PathBuf::from("data/item"),
        }
    }

    /// Create a new item cache with a custom data directory
    pub fn with_data_dir<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            items: HashMap::new(),
            data_dir: PathBuf::from(data_dir.as_ref()),
        }
    }

    /// Get item data by key (filename)
    pub fn get_item(&self, key: &str) -> Option<&RawItemData> {
        self.items.get(key)
    }

    /// Remove a loaded item template from the runtime registry. The source
    /// JSON remains on disk, matching Python's Item.Items deletion semantics.
    pub fn remove_item(&mut self, key: &str) -> bool {
        let removed = self.items.remove(key).is_some();
        if removed {
            mark_runtime_deleted(key);
        }
        removed
    }

    pub fn is_runtime_deleted(&self, key: &str) -> bool {
        is_runtime_deleted(key)
    }

    /// Find item by name ( searches through reaction names too)
    pub fn find_item_by_name(&self, name: &str) -> Option<&RawItemData> {
        // First try exact name match
        if let Some(item) = self.items.get(name) {
            return Some(item);
        }

        // Search through all items
        for item in self.items.values() {
            if item.name == name {
                return Some(item);
            }
            for alias in &item.reaction_names {
                if alias == name {
                    return Some(item);
                }
            }
        }

        None
    }

    /// Load an item from JSON file
    pub fn load_item(&mut self, filename: &str) -> Result<RawItemData, ItemError> {
        // Check cache first
        if let Some(data) = self.items.get(filename) {
            return Ok(data.clone());
        }

        // Build file path
        let file_path = self.data_dir.join(format!("{}.json", filename));

        if !file_path.exists() {
            return Err(ItemError::NotFound(filename.to_string()));
        }

        // Read and parse JSON
        let content =
            std::fs::read_to_string(&file_path).map_err(|e| ItemError::IoError(e.to_string()))?;

        let json: JsonValue =
            serde_json::from_str(&content).map_err(|e| ItemError::ParseError(e.to_string()))?;

        // Extract item info
        let item_info = json
            .get("아이템정보")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ItemError::ParseError("아이템정보 not found".to_string()))?;

        // Parse item data
        let data = self.parse_item_data(item_info)?;

        // Cache it
        self.items.insert(filename.to_string(), data.clone());
        clear_runtime_deleted(filename);

        Ok(data)
    }

    /// Parse item data from JSON object
    fn parse_item_data(
        &self,
        item_info: &serde_json::Map<String, JsonValue>,
    ) -> Result<RawItemData, ItemError> {
        let mut data = RawItemData::new();

        // Name (이름)
        data.name = item_info
            .get("이름")
            .and_then(|v| v.as_str())
            .unwrap_or("이름 없는 아이템")
            .to_string();

        // Item type (종류)
        data.item_type = item_info
            .get("종류")
            .and_then(|v| v.as_str())
            .unwrap_or("기타")
            .to_string();

        // Determine if equipment/consumable based on type
        match data.item_type.as_str() {
            "무기" | "방패" | "방어구" | "투구" | "신발" | "장갑" | "망토" | "악세사리" =>
            {
                data.is_equipment = true;
            }
            "먹는것" | "약" => {
                data.consumable = true;
            }
            "돈" | "은전" => {
                data.is_money = true;
            }
            _ => {}
        }

        // Subtype (세부종류)
        if let Some(subtype) = item_info.get("세부종류") {
            data.subtype = subtype.as_str().unwrap_or("").to_string();
        }

        // Description (설명 or 설명2)
        if let Some(desc) = item_info.get("설명2").or_else(|| item_info.get("설명")) {
            if let Some(arr) = desc.as_array() {
                data.description = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            } else if let Some(s) = desc.as_str() {
                data.description = vec![s.to_string()];
            }
        }

        // Reaction names (반응이름)
        if let Some(names) = item_info.get("반응이름") {
            if let Some(arr) = names.as_array() {
                data.reaction_names = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            } else if let Some(s) = names.as_str() {
                data.reaction_names = vec![s.to_string()];
            }
        }

        // Price (값 or 가격)
        data.price = item_info
            .get("값")
            .and_then(|v| v.as_i64())
            .or_else(|| item_info.get("가격").and_then(|v| v.as_i64()))
            .unwrap_or(0);

        // Weight (무게)
        data.weight = item_info.get("무게").and_then(|v| v.as_i64()).unwrap_or(0);

        // Level requirement (레벨제한)
        data.level_req = item_info
            .get("레벨제한")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Defense (방어력)
        data.defense = item_info
            .get("방어력")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Damage (공격력 or 타격)
        data.damage = item_info
            .get("공격력")
            .and_then(|v| v.as_i64())
            .or_else(|| item_info.get("타격").and_then(|v| v.as_i64()))
            .unwrap_or(0);

        // Extra damage (추가타격)
        data.extra_damage = item_info
            .get("추가타격")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Durability (내구도 or 최대내구도)
        data.max_durability = item_info
            .get("최대내구도")
            .and_then(|v| v.as_i64())
            .or_else(|| item_info.get("내구도").and_then(|v| v.as_i64()))
            .unwrap_or(1000);
        data.durability = data.max_durability;

        // Equipment slot (장착부위)
        data.equip_slot = item_info
            .get("장착부위")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Parse magic options (마법옵션 or option)
        for key in item_info.keys() {
            if key.contains("옵션") || key.contains("option") {
                if let Some(opt_value) = item_info.get(key) {
                    if let Some(s) = opt_value.as_str() {
                        // Parse "옵션이름 수치" format
                        let parts: Vec<&str> = s.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(value) = parts[1].parse::<i64>() {
                                data.magic_options.push((parts[0].to_string(), value));
                            }
                        }
                    }
                }
            }
        }

        // Learn skill (배울무공)
        data.learn_skill = item_info
            .get("배울무공")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Use effect (사용효과)
        data.use_effect = item_info
            .get("사용효과")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(data)
    }

    /// Preload all items
    pub fn preload_all(&mut self) -> Result<usize, ItemError> {
        if !self.data_dir.exists() {
            return Err(ItemError::NotFound(
                self.data_dir.to_string_lossy().to_string(),
            ));
        }

        let entries =
            std::fs::read_dir(&self.data_dir).map_err(|e| ItemError::IoError(e.to_string()))?;

        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| ItemError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| ItemError::ParseError("Invalid file name".to_string()))?;

                self.load_item(name)?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Get the number of cached items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for ItemCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when working with items
#[derive(Debug, thiserror::Error)]
pub enum ItemError {
    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Item is broken")]
    Broken,

    #[error("Item not equipped")]
    NotEquipped,

    #[error("Already equipped")]
    AlreadyEquipped,
}

/// Global item cache accessor
pub fn get_item_cache() -> &'static RwLock<ItemCache> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<RwLock<ItemCache>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(ItemCache::new()))
}

/// Read item weight from data/item/{key}.json. Returns 0 if not found. inv_stack 무게 합산용.
pub fn get_item_weight_by_key(key: &str) -> i64 {
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return 0,
    };
    let info = json.get("아이템정보").and_then(|v| v.as_object());
    info.and_then(|o| o.get("무게").and_then(|v| v.as_i64()))
        .unwrap_or(0)
}

/// 아이템 표시이름. data/item/{key}.json의 아이템정보.이름, 없으면 key.
pub fn get_item_display_name(key: &str) -> String {
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return key.to_string(),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return key.to_string(),
    };
    json.get("아이템정보")
        .and_then(|o| o.get("이름").and_then(|v| v.as_str()))
        .unwrap_or(key)
        .to_string()
}

/// Helper to create an item instance
pub fn create_item(item_key: &str) -> Result<ItemInstance, ItemError> {
    let cache = get_item_cache().read().unwrap();
    if let Some(data) = cache.get_item(item_key) {
        Ok(ItemInstance::from_data(data))
    } else {
        Err(ItemError::NotFound(item_key.to_string()))
    }
}

/// Helper to find and create an item by name
pub fn find_or_create_item(name: &str) -> Result<ItemInstance, ItemError> {
    let cache = get_item_cache().read().unwrap();
    if let Some(data) = cache.find_item_by_name(name) {
        let mut instance = ItemInstance::from_data(data);
        instance.item_key = data.name.clone();
        Ok(instance)
    } else {
        Err(ItemError::NotFound(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_item_data_new() {
        let data = RawItemData::new();
        assert_eq!(data.item_type, "기타");
        assert_eq!(data.price, 0);
        assert!(data.flags.is_empty());
    }

    #[test]
    fn test_item_instance_new() {
        let instance = ItemInstance::new("test_item".to_string(), "Test Item".to_string());
        assert_eq!(instance.name, "Test Item");
        assert_eq!(instance.durability, 1000);
        assert!(!instance.equipped);
    }

    #[test]
    fn test_item_instance_use() {
        let mut instance = ItemInstance::new("test_item".to_string(), "Test Item".to_string());
        assert!(!instance.is_broken());

        instance.use_item(100);
        assert_eq!(instance.durability, 900);
        assert_eq!(instance.usage_count, 1);

        instance.use_item(900);
        assert!(instance.is_broken());
        assert!(!instance.use_item(10)); // Can't use broken item
    }

    #[test]
    fn test_item_instance_repair() {
        let mut instance = ItemInstance::new("test_item".to_string(), "Test Item".to_string());
        instance.durability = 500;

        instance.repair(100);
        assert_eq!(instance.durability, 600);

        instance.repair(1000); // Should cap at 1000
        assert_eq!(instance.durability, 1000);
    }

    #[test]
    fn test_item_instance_get_display_name() {
        let mut instance = ItemInstance::new("test_item".to_string(), "검".to_string());
        assert_eq!(instance.get_display_name(), "검");

        instance.enchant = 5;
        assert_eq!(instance.get_display_name(), "+5 검");
    }

    #[test]
    fn test_item_cache_new() {
        let cache = ItemCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.data_dir, PathBuf::from("data/item"));
    }

    #[test]
    fn test_item_cache_with_data_dir() {
        let cache = ItemCache::with_data_dir("/custom/path");
        assert_eq!(cache.data_dir, PathBuf::from("/custom/path"));
    }
}
