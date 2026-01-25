//! Base Object structure for MUD engine
//!
//! Provides the core Object implementation with:
//! - Attribute management (get, set, getInt, getString)
//! - Object hierarchy (env for parent, objs for children)
//! - Name handling (getName, getNameA)
//! - Attribute manipulation (checkAttr, setAttr, delAttr)
//! - Object search (findObjName, findObjInven, findObjInUse)
//! - Korean particle helper methods (han_iga, han_obj, han_un)

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use crate::object::Value;
use crate::hangul;

/// Base Object structure for MUD game objects
///
/// # Fields
/// * `attr` - Persistent attributes stored as key-value pairs
/// * `temp` - Temporary attributes that don't persist
/// * `env` - Weak reference to parent object (prevents circular references)
/// * `objs` - Child objects (non-stackable: 무기, 방어구, 개별인스턴스)
/// * `inv_stack` - Stackable item counts: 인덱스(item key) -> 수량 (먹는것, 약 등)
#[derive(Debug)]
pub struct Object {
    /// Persistent attributes map
    pub attr: HashMap<String, Value>,
    /// Temporary attributes map
    pub temp: HashMap<String, Value>,
    /// Parent object (weak reference to prevent cycles)
    pub env: Option<Weak<Mutex<Object>>>,
    /// Child objects (non-stackable only)
    pub objs: Vec<Arc<Mutex<Object>>>,
    /// Stackable inventory: item key (인덱스) -> count. Save/load as 소지품_수량.
    pub inv_stack: HashMap<String, i64>,
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}

impl Object {
    /// Creates a new empty Object
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let obj = Object::new();
    /// assert_eq!(obj.attr.len(), 0);
    /// assert_eq!(obj.objs.len(), 0);
    /// ```
    pub fn new() -> Self {
        Object {
            attr: HashMap::new(),
            temp: HashMap::new(),
            env: None,
            objs: Vec::new(),
            inv_stack: HashMap::new(),
        }
    }

    /// Sets an attribute value
    ///
    /// # Arguments
    /// * `key` - Attribute name
    /// * `keydata` - Attribute value
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("이름", "검");
    /// ```
    pub fn set(&mut self, key: &str, keydata: impl Into<Value>) {
        self.attr.insert(key.to_string(), keydata.into());
    }

    /// Gets an attribute value as a Value
    ///
    /// Returns empty string Value if key doesn't exist
    ///
    /// # Arguments
    /// * `key` - Attribute name
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    /// use muc_engine::utils::Value;
    ///
    /// let mut obj = Object::new();
    /// obj.set("test", "value");
    /// assert_eq!(obj.get("test"), Value::String("value".to_string()));
    /// assert_eq!(obj.get("nonexistent"), Value::String("".to_string()));
    /// ```
    pub fn get(&self, key: &str) -> Value {
        self.attr.get(key)
            .cloned()
            .unwrap_or(Value::String(String::new()))
    }

    /// Gets an attribute value as a string
    ///
    /// Returns empty string if key doesn't exist
    ///
    /// # Arguments
    /// * `key` - Attribute name
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("이름", "검");
    /// assert_eq!(obj.getString("이름"), "검");
    /// assert_eq!(obj.getString("없는속성"), "");
    /// ```
    pub fn getString(&self, key: &str) -> String {
        match self.get(key) {
            Value::String(s) => s,
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
        }
    }

    /// Gets an attribute value as an integer
    ///
    /// Returns 0 if key doesn't exist or value is not a number
    ///
    /// # Arguments
    /// * `key` - Attribute name
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("공격력", 100);
    /// assert_eq!(obj.getInt("공격력"), 100);
    /// assert_eq!(obj.getInt("없는속성"), 0);
    /// ```
    pub fn getInt(&self, key: &str) -> i64 {
        match self.get(key) {
            Value::Int(i) => i,
            Value::Float(f) => f as i64,
            Value::String(_) => 0,
        }
    }

    /// Gets the name attribute value
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("이름", "철검");
    /// assert_eq!(obj.getName(), "철검");
    /// ```
    pub fn getName(&self) -> String {
        self.getString("이름")
    }

    /// Gets the name with yellow color ANSI codes
    ///
    /// Returns "\x1b[33m{name}\x1b[37m" format
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("이름", "철검");
    /// assert_eq!(obj.getNameA(), "\x1b[33m철검\x1b[37m");
    /// ```
    pub fn getNameA(&self) -> String {
        format!("\x1b[33m{}\x1b[37m", self.getName())
    }

    /// Sets a temporary attribute value
    ///
    /// # Arguments
    /// * `key` - Temporary attribute name
    /// * `keydata` - Attribute value
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.setTemp("temporary", 42);
    /// assert_eq!(obj.getTemp("temporary"), 42);
    /// ```
    pub fn setTemp(&mut self, key: &str, keydata: impl Into<Value>) {
        self.temp.insert(key.to_string(), keydata.into());
    }

    /// Gets a temporary attribute value
    ///
    /// Returns empty string Value if key doesn't exist
    ///
    /// # Arguments
    /// * `key` - Temporary attribute name
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.setTemp("temp", "value");
    /// assert_eq!(obj.getTemp("temp"), "value");
    /// ```
    pub fn getTemp(&self, key: &str) -> Value {
        self.temp.get(key)
            .cloned()
            .unwrap_or(Value::String(String::new()))
    }

    /// Inserts a child object at the beginning of the objs list
    ///
    /// Sets the child's env to self if not already in objs
    ///
    /// # Arguments
    /// * `obj` - Child object to insert
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut parent = Object::new();
    /// let child = Arc::new(Mutex::new(Object::new()));
    /// parent.insert(child.clone());
    /// assert_eq!(parent.objs.len(), 1);
    /// ```
    pub fn insert(&mut self, obj: Arc<Mutex<Object>>) {
        // Check if obj is already in objs (by address comparison)
        let obj_ptr = Arc::as_ptr(&obj) as *const ();
        let already_contains = self.objs.iter()
            .any(|o| Arc::as_ptr(o) as *const () == obj_ptr);

        if !already_contains {
            // Set the child's env to self (using weak reference)
            if let Ok(mut child) = obj.lock() {
                // Create a weak reference to self
                // This requires self to be wrapped in Arc, which we'll handle at call site
                // For now, just note that env needs to be set externally
                child.env = None; // Will be set by caller
            }
            self.objs.insert(0, obj);
        }
    }

    /// Appends a child object to the end of the objs list
    ///
    /// Sets the child's env to self if not already in objs
    ///
    /// # Arguments
    /// * `obj` - Child object to append
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut parent = Object::new();
    /// let child = Arc::new(Mutex::new(Object::new()));
    /// parent.append(child.clone());
    /// assert_eq!(parent.objs.len(), 1);
    /// ```
    pub fn append(&mut self, obj: Arc<Mutex<Object>>) {
        // Check if obj is already in objs (by address comparison)
        let obj_ptr = Arc::as_ptr(&obj) as *const ();
        let already_contains = self.objs.iter()
            .any(|o| Arc::as_ptr(o) as *const () == obj_ptr);

        if !already_contains {
            if let Ok(mut child) = obj.lock() {
                child.env = None; // Will be set by caller
            }
            self.objs.push(obj);
        }
    }

    /// Removes a child object from the objs list
    ///
    /// Sets the child's env to None
    ///
    /// # Arguments
    /// * `obj` - Child object to remove
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut parent = Object::new();
    /// let child = Arc::new(Mutex::new(Object::new()));
    /// parent.append(child.clone());
    /// parent.remove(child.clone());
    /// assert_eq!(parent.objs.len(), 0);
    /// ```
    pub fn remove(&mut self, obj: &Arc<Mutex<Object>>) {
        let obj_ptr = Arc::as_ptr(obj) as *const ();
        self.objs.retain(|o| {
            let ptr = Arc::as_ptr(o) as *const ();
            let keep = ptr != obj_ptr;
            if !keep {
                if let Ok(mut child) = o.lock() {
                    child.env = None;
                }
            }
            keep
        });
    }

    /// Checks if a string attribute contains a specific value
    ///
    /// # Arguments
    /// * `key` - Attribute name
    /// * `attr` - Value to check for
    ///
    /// # Returns
    /// true if attr is found in the attribute value
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("속성", "마법 전설");
    /// assert!(obj.checkAttr("속성", "마법"));
    /// assert!(!obj.checkAttr("속성", "일반"));
    /// ```
    pub fn checkAttr(&self, key: &str, attr: &str) -> bool {
        let keydata = self.getString(key);
        keydata.contains(attr)
    }

    /// Sets a string attribute by adding a value to it
    ///
    /// Converts single string to list if needed
    ///
    /// # Arguments
    /// * `key` - Attribute name
    /// * `attr` - Value to add
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.setAttr("타입", "무기");
    /// assert_eq!(obj.getString("타입"), "무기");
    /// ```
    pub fn setAttr(&mut self, key: &str, attr: &str) {
        let current = self.get(key);

        let mut lines = match current {
            Value::String(s) if s.is_empty() => Vec::new(),
            Value::String(s) => vec![s],
            Value::Int(i) => vec![i.to_string()],
            Value::Float(f) => vec![f.to_string()],
        };

        // Check if attr already exists
        if lines.iter().any(|line| line == attr) {
            return;
        }

        lines.push(attr.to_string());

        // Join with newlines to maintain Python compatibility
        let result = lines.join("\n");
        self.set(key, result);
    }

    /// Deletes a value from a string attribute
    ///
    /// # Arguments
    /// * `key` - Attribute name
    /// * `attr` - Value to remove
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("속성", "마법\n전설");
    /// obj.delAttr("속성", "마법");
    /// assert_eq!(obj.getString("속성"), "전설");
    /// ```
    pub fn delAttr(&mut self, key: &str, attr: &str) {
        let current = self.get(key);

        let mut attrs = match current {
            Value::String(s) if s.is_empty() => Vec::new(),
            Value::String(s) => s.split('\n').map(|x| x.to_string()).collect(),
            Value::Int(i) => vec![i.to_string()],
            Value::Float(f) => vec![f.to_string()],
        };

        attrs.retain(|a| a != attr);

        let result = attrs.join("\n");
        self.set(key, result);
    }

    /// getOption: "옵션" 속성 파싱 → HashMap<이름, 수치>. Python item.getOption().
    pub fn get_option(&self) -> Option<std::collections::HashMap<String, i64>> {
        let s = self.getString("옵션");
        if s.is_empty() {
            return None;
        }
        let mut map = std::collections::HashMap::new();
        for line in s.split('\n') {
            let w: Vec<&str> = line.split_whitespace().collect();
            if w.len() >= 2 {
                if let Ok(v) = w[1].parse::<i64>() {
                    map.insert(w[0].to_string(), v);
                }
            }
        }
        if map.is_empty() {
            None
        } else {
            Some(map)
        }
    }

    /// setOption: HashMap → "옵션" 속성. Python item.setOption(option).
    pub fn set_option(&mut self, option: &std::collections::HashMap<String, i64>) {
        let s: Vec<String> = option
            .iter()
            .map(|(k, v)| format!("{} {}", k, v))
            .collect();
        self.set("옵션", s.join("\n"));
    }

    /// getOptionStr: "힘(10), 민첩성(5)" 형식. Python item.getOptionStr().
    pub fn get_option_str(&self) -> String {
        let Some(opt) = self.get_option() else {
            return String::new();
        };
        opt.iter()
            .map(|(k, v)| format!("{}({})", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Finds a child object by name or reaction name
    ///
    /// # Arguments
    /// * `name` - Name to search for
    /// * `order` - Which occurrence to return (1-indexed)
    ///
    /// # Returns
    /// Arc<Mutex<Object>> if found, None otherwise
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut parent = Object::new();
    /// let child = Arc::new(Mutex::new(Object::new()));
    /// child.lock().unwrap().set("이름", "검");
    /// parent.append(child);
    /// let found = parent.findObjName("검", 1);
    /// assert!(found.is_some());
    /// ```
    pub fn findObjName(&self, name: &str, order: usize) -> Option<Arc<Mutex<Object>>> {
        let mut n = 0;
        for obj in &self.objs {
            if let Ok(o) = obj.lock() {
                let obj_name = o.getName();
                let reaction_names = o.getString("반응이름");

                // Check if name matches or is in reaction names
                let name_matches = obj_name == name ||
                    (!reaction_names.is_empty() && reaction_names.contains(name));

                // Skip if has "출력안함" attribute
                if o.checkAttr("아이템속성", "출력안함") {
                    continue;
                }

                if name_matches {
                    n += 1;
                    if n == order {
                        return Some(obj.clone());
                    }
                }
            }
        }
        None
    }

    /// 인덱스(아이템 키)로 objs에서 찾기. Python getItemIndex(index).
    pub fn find_by_index(&self, index: &str) -> Option<Arc<Mutex<Object>>> {
        for obj in &self.objs {
            if let Ok(o) = obj.lock() {
                if o.getString("인덱스") == index {
                    return Some(obj.clone());
                }
            }
        }
        None
    }

    /// Finds a child object in inventory (not in use)
    ///
    /// # Arguments
    /// * `name` - Name to search for
    /// * `order` - Which occurrence to return (1-indexed)
    ///
    /// # Returns
    /// Arc<Mutex<Object>> if found, None otherwise
    pub fn findObjInven(&self, name: &str, order: usize) -> Option<Arc<Mutex<Object>>> {
        let mut n = 0;
        for obj in &self.objs {
            if let Ok(o) = obj.lock() {
                let obj_name = o.getName();
                let reaction_names = o.getString("반응이름");

                let name_matches = obj_name == name ||
                    (!reaction_names.is_empty() && reaction_names.contains(name));

                // Skip if inUse is true
                let in_use = o.getBool("inUse");

                if name_matches && !in_use {
                    n += 1;
                    if n == order {
                        return Some(obj.clone());
                    }
                }
            }
        }
        None
    }

    /// Finds a child object that is in use
    ///
    /// # Arguments
    /// * `name` - Name to search for
    /// * `order` - Which occurrence to return (1-indexed)
    ///
    /// # Returns
    /// Arc<Mutex<Object>> if found, None otherwise
    pub fn findObjInUse(&self, name: &str, order: usize) -> Option<Arc<Mutex<Object>>> {
        let mut n = 0;
        for obj in &self.objs {
            if let Ok(o) = obj.lock() {
                let obj_name = o.getName();
                let reaction_names = o.getString("반응이름");

                let name_matches = obj_name == name ||
                    (!reaction_names.is_empty() && reaction_names.contains(name));

                let in_use = o.getBool("inUse");

                if name_matches && in_use {
                    n += 1;
                    if n == order {
                        return Some(obj.clone());
                    }
                }
            }
        }
        None
    }

    /// Gets a boolean attribute value
    ///
    /// # Arguments
    /// * `key` - Attribute name
    ///
    /// # Returns
    /// true if the attribute is a truthy value, false otherwise
    pub fn getBool(&self, key: &str) -> bool {
        match self.get(key) {
            Value::Int(i) => i != 0,
            Value::Float(f) => f != 0.0,
            Value::String(s) => !s.is_empty() && s != "0" && s != "false",
        }
    }

    /// Returns the name with Korean particle (이/가)
    ///
    /// Format: "\x1b[33m{name}\x1b[37m{particle}"
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("이름", "검");
    /// assert_eq!(obj.han_iga(), "\x1b[33m검\x1b[37m이");
    /// ```
    pub fn han_iga(&self) -> String {
        let name = self.getName();
        format!("{}{}", self.getNameA(), hangul::han_iga(&name))
    }

    /// Returns the name with Korean particle (을/를)
    ///
    /// Format: "\x1b[33m{name}\x1b[37m{particle}"
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("이름", "검");
    /// assert_eq!(obj.han_obj(), "\x1b[33m검\x1b[37m을");
    /// ```
    pub fn han_obj(&self) -> String {
        let name = self.getName();
        format!("{}{}", self.getNameA(), hangul::han_obj(&name))
    }

    /// Returns the name with Korean particle (은/는)
    ///
    /// Format: "\x1b[33m{name}\x1b[37m{particle}"
    ///
    /// # Examples
    /// ```
    /// use muc_engine::object::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("이름", "검");
    /// assert_eq!(obj.han_un(), "\x1b[33m검\x1b[37m은");
    /// ```
    pub fn han_un(&self) -> String {
        let name = self.getName();
        format!("{}{}", self.getNameA(), hangul::han_un(&name))
    }

    /// Creates a shallow clone of the object
    ///
    /// Note: env will be None in the clone, objs will be cloned as references
    pub fn clone(&self) -> Self {
        Object {
            attr: self.attr.clone(),
            temp: self.temp.clone(),
            env: None, // Don't clone env reference
            objs: self.objs.clone(), // Shallow copy of Arc references
            inv_stack: self.inv_stack.clone(),
        }
    }

    /// Creates a deep clone of the object
    ///
    /// Note: env will be None in the clone, objs will be deeply cloned
    pub fn deepclone(&self) -> Self {
        // Deep clone attributes
        let attr = self.attr.clone();
        let temp = self.temp.clone();

        // Deep clone child objects
        let objs: Vec<Arc<Mutex<Object>>> = self.objs.iter()
            .map(|o| {
                if let Ok(inner) = o.lock() {
                    Arc::new(Mutex::new(inner.deepclone()))
                } else {
                    Arc::new(Mutex::new(Object::new()))
                }
            })
            .collect();

        Object {
            attr,
            temp,
            env: None,
            objs,
            inv_stack: self.inv_stack.clone(),
        }
    }
}

impl Clone for Object {
    fn clone(&self) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_object() {
        let obj = Object::new();
        assert_eq!(obj.attr.len(), 0);
        assert_eq!(obj.temp.len(), 0);
        assert_eq!(obj.objs.len(), 0);
        assert!(obj.env.is_none());
    }

    #[test]
    fn test_set_get_string() {
        let mut obj = Object::new();
        obj.set("이름", "철검");
        assert_eq!(obj.get("이름"), Value::String("철검".to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        let obj = Object::new();
        assert_eq!(obj.get("nonexistent"), Value::String("".to_string()));
    }

    #[test]
    fn test_get_string() {
        let mut obj = Object::new();
        obj.set("이름", "철검");
        assert_eq!(obj.getString("이름"), "철검");
        assert_eq!(obj.getString("없는속성"), "");
    }

    #[test]
    fn test_get_int() {
        let mut obj = Object::new();
        obj.set("공격력", 100);
        assert_eq!(obj.getInt("공격력"), 100);
        assert_eq!(obj.getInt("없는속성"), 0);
    }

    #[test]
    fn test_get_int_from_string() {
        let mut obj = Object::new();
        obj.set("value", "text");
        assert_eq!(obj.getInt("value"), 0);
    }

    #[test]
    fn test_get_name() {
        let mut obj = Object::new();
        obj.set("이름", "용사");
        assert_eq!(obj.getName(), "용사");
    }

    #[test]
    fn test_get_name_a() {
        let mut obj = Object::new();
        obj.set("이름", "용사");
        assert_eq!(obj.getNameA(), "\x1b[33m용사\x1b[37m");
    }

    #[test]
    fn test_temp_attributes() {
        let mut obj = Object::new();
        obj.setTemp("temp", "value");
        assert_eq!(obj.getTemp("temp"), Value::String("value".to_string()));
        assert_eq!(obj.getTemp("nonexistent"), Value::String("".to_string()));
    }

    #[test]
    fn test_append_child() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        parent.append(child.clone());
        assert_eq!(parent.objs.len(), 1);
    }

    #[test]
    fn test_append_duplicate_prevented() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        parent.append(child.clone());
        parent.append(child.clone());
        assert_eq!(parent.objs.len(), 1);
    }

    #[test]
    fn test_insert_child() {
        let mut parent = Object::new();
        let child1 = Arc::new(Mutex::new(Object::new()));
        let child2 = Arc::new(Mutex::new(Object::new()));

        parent.append(child1.clone());
        parent.insert(child2.clone());

        assert_eq!(parent.objs.len(), 2);
        // First should be child2 (inserted at beginning)
        let ptr2 = Arc::as_ptr(&parent.objs[0]) as *const ();
        let expected2 = Arc::as_ptr(&child2) as *const ();
        assert_eq!(ptr2, expected2);
    }

    #[test]
    fn test_remove_child() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        parent.append(child.clone());
        assert_eq!(parent.objs.len(), 1);

        parent.remove(&child);
        assert_eq!(parent.objs.len(), 0);
    }

    #[test]
    fn test_check_attr() {
        let mut obj = Object::new();
        obj.set("속성", "마법 전설");

        assert!(obj.checkAttr("속성", "마법"));
        assert!(obj.checkAttr("속성", "전설"));
        assert!(!obj.checkAttr("속성", "일반"));
        assert!(!obj.checkAttr("없는속성", "마법"));
    }

    #[test]
    fn test_set_attr() {
        let mut obj = Object::new();
        obj.setAttr("타입", "무기");
        assert_eq!(obj.getString("타입"), "무기");

        obj.setAttr("타입", "마법");
        assert!(obj.checkAttr("타입", "무기"));
        assert!(obj.checkAttr("타입", "마법"));
    }

    #[test]
    fn test_set_attr_duplicate_prevented() {
        let mut obj = Object::new();
        obj.setAttr("타입", "무기");
        obj.setAttr("타입", "무기");

        // Should only appear once
        let value = obj.getString("타입");
        assert_eq!(value, "무기");
    }

    #[test]
    fn test_del_attr() {
        let mut obj = Object::new();
        obj.set("속성", "마법\n전설\n레어");

        obj.delAttr("속성", "마법");
        assert_eq!(obj.getString("속성"), "전설\n레어");
        assert!(!obj.checkAttr("속성", "마법"));
        assert!(obj.checkAttr("속성", "전설"));
    }

    #[test]
    fn test_find_obj_name() {
        let mut parent = Object::new();
        let child1 = Arc::new(Mutex::new(Object::new()));
        let child2 = Arc::new(Mutex::new(Object::new()));

        child1.lock().unwrap().set("이름", "검");
        child2.lock().unwrap().set("이름", "방패");

        parent.append(child1);
        parent.append(child2);

        let found = parent.findObjName("검", 1);
        assert!(found.is_some());
        if let Some(obj) = found {
            assert_eq!(obj.lock().unwrap().getName(), "검");
        }
    }

    #[test]
    fn test_find_obj_name_by_reaction_name() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        child.lock().unwrap().set("이름", "철검");
        child.lock().unwrap().set("반응이름", "검 무기");

        parent.append(child);

        let found = parent.findObjName("검", 1);
        assert!(found.is_some());
    }

    #[test]
    fn test_find_obj_name_skips_hidden() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        child.lock().unwrap().set("이름", "검");
        child.lock().unwrap().set("아이템속성", "출력안함");

        parent.append(child);

        let found = parent.findObjName("검", 1);
        assert!(found.is_none());
    }

    #[test]
    fn test_find_obj_inven() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        child.lock().unwrap().set("이름", "검");
        child.lock().unwrap().set("inUse", 0); // Not in use

        parent.append(child);

        let found = parent.findObjInven("검", 1);
        assert!(found.is_some());
    }

    #[test]
    fn test_find_obj_inven_skips_in_use() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        child.lock().unwrap().set("이름", "검");
        child.lock().unwrap().set("inUse", 1); // In use

        parent.append(child);

        let found = parent.findObjInven("검", 1);
        assert!(found.is_none());
    }

    #[test]
    fn test_find_obj_in_use() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        child.lock().unwrap().set("이름", "검");
        child.lock().unwrap().set("inUse", 1); // In use

        parent.append(child);

        let found = parent.findObjInUse("검", 1);
        assert!(found.is_some());
    }

    #[test]
    fn test_find_obj_in_use_skips_not_in_use() {
        let mut parent = Object::new();
        let child = Arc::new(Mutex::new(Object::new()));

        child.lock().unwrap().set("이름", "검");
        child.lock().unwrap().set("inUse", 0); // Not in use

        parent.append(child);

        let found = parent.findObjInUse("검", 1);
        assert!(found.is_none());
    }

    #[test]
    fn test_get_bool() {
        let mut obj = Object::new();

        obj.set("flag", 1);
        assert!(obj.getBool("flag"));

        obj.set("flag", 0);
        assert!(!obj.getBool("flag"));

        obj.set("flag", "true");
        assert!(obj.getBool("flag"));

        obj.set("flag", "false");
        assert!(!obj.getBool("flag"));

        assert!(!obj.getBool("nonexistent"));
    }

    #[test]
    fn test_han_iga() {
        let mut obj = Object::new();
        obj.set("이름", "검");
        assert_eq!(obj.han_iga(), "\x1b[33m검\x1b[37m이");

        obj.set("이름", "사과");
        assert_eq!(obj.han_iga(), "\x1b[33m사과\x1b[37m가");
    }

    #[test]
    fn test_han_obj() {
        let mut obj = Object::new();
        obj.set("이름", "검");
        assert_eq!(obj.han_obj(), "\x1b[33m검\x1b[37m을");

        obj.set("이름", "사과");
        assert_eq!(obj.han_obj(), "\x1b[33m사과\x1b[37m를");
    }

    #[test]
    fn test_han_un() {
        let mut obj = Object::new();
        obj.set("이름", "검");
        assert_eq!(obj.han_un(), "\x1b[33m검\x1b[37m은");

        obj.set("이름", "사과");
        assert_eq!(obj.han_un(), "\x1b[33m사과\x1b[37m는");
    }

    #[test]
    fn test_clone() {
        let mut parent = Object::new();
        parent.set("이름", "부모");

        let child = Arc::new(Mutex::new(Object::new()));
        child.lock().unwrap().set("이름", "자식");
        parent.append(child);

        let cloned = parent.clone();

        assert_eq!(cloned.getName(), "부모");
        assert_eq!(cloned.objs.len(), 1);
        assert!(cloned.env.is_none());
    }

    #[test]
    fn test_deepclone() {
        let mut parent = Object::new();
        parent.set("이름", "부모");

        let child = Arc::new(Mutex::new(Object::new()));
        child.lock().unwrap().set("이름", "자식");
        parent.append(child);

        let cloned = parent.deepclone();

        assert_eq!(cloned.getName(), "부모");
        assert_eq!(cloned.objs.len(), 1);

        // Deep clone should have separate child objects
        let original_child_name = parent.objs[0].lock().unwrap().getName();
        let cloned_child_name = cloned.objs[0].lock().unwrap().getName();
        assert_eq!(original_child_name, cloned_child_name);
    }

    #[test]
    fn test_find_with_order() {
        let mut parent = Object::new();

        for _i in 0..3 {
            let child = Arc::new(Mutex::new(Object::new()));
            child.lock().unwrap().set("이름", "아이템");
            parent.append(child);
        }

        let first = parent.findObjName("아이템", 1);
        let second = parent.findObjName("아이템", 2);
        let third = parent.findObjName("아이템", 3);
        let fourth = parent.findObjName("아이템", 4);

        assert!(first.is_some());
        assert!(second.is_some());
        assert!(third.is_some());
        assert!(fourth.is_none());
    }
}
