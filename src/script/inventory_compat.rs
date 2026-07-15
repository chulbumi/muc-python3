//! Inventory persistence and legacy Python record compatibility.
//!
//! Pristine copies of an item template stay counted in `inv_stack` both at
//! runtime and in save files. An extended, strengthened, UUID-bearing,
//! equipped, unique, or otherwise changed item stays as an individual object
//! and is stored with only its `변경` fields and any template fields listed in
//! `제거`. Legacy Python item records, the short-lived full `속성` shape, and
//! old Rust `소지품_수량` files remain readable.

use crate::object::{Object, Value};
use crate::player::Body;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::sync::{Arc, Mutex};

const JSON_ARRAY_MARKER_PREFIX: &str = "_python_json_array:";

/// `Player.load()` normally runs on a freshly constructed Body. Rust also
/// uses this loader for an already allocated Body (offline note edits and
/// tests), so clear derived runtime state first to make a repeated load behave
/// like a new Python Player rather than accumulating equipment/skill bonuses.
pub(super) fn reset_body_derived_state(body: &mut Body) {
    body.active_skills.clear();
    body.unwear_all();
    body.dex = 0;
}

fn json_array_marker(field: &str) -> String {
    format!("{JSON_ARRAY_MARKER_PREFIX}{field}")
}

/// Mark an item field as a Python JSON array after an in-memory mutation that
/// has Python `list` semantics (for example Object.setAttr or list.append).
/// Rust stores the elements in a newline-delimited scalar internally, so the
/// shape marker is required for Player.save() to reconstruct the array.
pub(crate) fn mark_item_field_as_json_array(item: &mut Object, field: &str) {
    item.temp.insert(json_array_marker(field), Value::Int(1));
}

/// Python's `value in item[field]`: list-valued JSON fields use exact element
/// membership, while legacy scalar strings use substring membership.
pub(crate) fn python_item_field_contains(item: &Object, field: &str, wanted: &str) -> bool {
    let raw = item.getString(field);
    if item.temp.contains_key(&json_array_marker(field)) {
        raw.split('\n').any(|entry| entry == wanted)
    } else {
        raw.contains(wanted)
    }
}

/// Preserve whether a Python item field was a JSON array even though the
/// current Rust `Value` type stores list data as a string. Array elements are
/// separated with newlines because option entries such as `"힘 10"` contain
/// spaces themselves.
pub(crate) fn set_item_json_field(item: &mut Object, field: &str, value: &JsonValue) {
    if !matches!(value, JsonValue::Array(_) | JsonValue::Object(_)) {
        // A saved scalar replaces (rather than inherits) an array-valued
        // template field in Python. Keep the shape marker synchronized with
        // that assignment so Box/Player save does not turn it back into an
        // array.
        item.temp.remove(&json_array_marker(field));
    }
    match value {
        JsonValue::Null => item.set(field, ""),
        JsonValue::Bool(value) => item.set(field, i64::from(*value)),
        JsonValue::Number(value) => {
            if let Some(value) = value.as_i64() {
                item.set(field, value);
            } else if let Some(value) = value.as_f64() {
                item.set(field, value);
            }
        }
        JsonValue::String(value) => item.set(field, Value::String(value.clone())),
        JsonValue::Array(values) => {
            let values = values
                .iter()
                .filter_map(JsonValue::as_str)
                .collect::<Vec<_>>()
                .join("\n");
            item.set(field, Value::String(values));
            item.temp.insert(json_array_marker(field), Value::Int(1));
        }
        // Python's persisted item fields used here are scalars or string
        // arrays. Do not invent a conversion for object-valued data.
        JsonValue::Object(_) => {}
    }
}

fn plain_value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Int(value) => JsonValue::Number((*value).into()),
        Value::Float(value) => serde_json::Number::from_f64(*value)
            .map(JsonValue::Number)
            .unwrap_or_else(|| JsonValue::Number(0.into())),
        Value::String(value) => JsonValue::String(value.clone()),
    }
}

/// `Object::get` returns a scalar default for a missing attribute.  Some
/// legacy mutation helpers materialize that default into `attr` (most notably
/// `고유번호 = ""`) even though the observable item state did not change.
/// Do not persist such entries as deltas merely because the HashMap now has a
/// key the template omits.
fn is_missing_attribute_default(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => true,
        JsonValue::Bool(value) => !value,
        JsonValue::Number(value) => value.as_i64() == Some(0) || value.as_f64() == Some(0.0),
        JsonValue::String(value) => value.is_empty(),
        JsonValue::Array(value) => value.is_empty(),
        JsonValue::Object(value) => value.is_empty(),
    }
}

pub(crate) fn item_field_to_json(item: &Object, field: &str) -> JsonValue {
    let value = item.get(field);
    let marked_array = item.temp.contains_key(&json_array_marker(field));
    if marked_array {
        let raw = match value {
            Value::String(value) => value,
            Value::Int(value) => value.to_string(),
            Value::Float(value) => value.to_string(),
        };
        return JsonValue::Array(
            raw.split('\n')
                .filter(|entry| !entry.is_empty())
                .map(|entry| JsonValue::String(entry.to_string()))
                .collect(),
        );
    }

    plain_value_to_json(&value)
}

/// Serialize the complete Python `Item.attr` mapping.  This is used by the
/// lending catalogue: Python stores an item's whole attribute dictionary at
/// registration time, rather than a selected persistence subset.
pub(crate) fn item_attributes_to_json(item: &Object) -> JsonValue {
    let mut attributes = JsonMap::new();
    for field in item.attr.keys() {
        // Rust keeps `인덱스` in attr as an implementation convenience, but
        // Python stores it on Item.index rather than Item.attr. The lending
        // catalogue must not leak that internal representation into `attr`.
        if field == "인덱스" {
            continue;
        }
        attributes.insert(field.clone(), item_field_to_json(item, field));
    }
    JsonValue::Object(attributes)
}

/// True when an inventory object has no persistent or nested state beyond its
/// item JSON template and can therefore live in `inv_stack` at runtime.
pub(crate) fn is_pristine_template_object(item: &Object) -> bool {
    let index = item.getString("인덱스");
    if index.is_empty()
        || !super::is_stackable(&index)
        || !item.objs.is_empty()
        || item.inv_stack.values().any(|count| *count > 0)
    {
        return false;
    }
    super::object_from_item_json(&index)
        .and_then(|(template, _)| {
            template
                .lock()
                .ok()
                .map(|template| item_attributes_to_json(item) == item_attributes_to_json(&template))
        })
        .unwrap_or(false)
}

fn is_single_item_template(key: &str) -> bool {
    super::object_from_item_json(key)
        .and_then(|(item, _)| {
            item.lock()
                .ok()
                .map(|item| item.checkAttr("아이템속성", "단일아이템"))
        })
        .unwrap_or(false)
}

pub(crate) fn inventory_contains_index(inventory: &Object, key: &str) -> bool {
    inventory.inv_stack.get(key).copied().unwrap_or(0) > 0
        || inventory.objs.iter().any(|item| {
            item.lock()
                .ok()
                .is_some_and(|item| item.getString("인덱스") == key)
        })
}

pub(crate) fn can_accept_object(inventory: &Object, item: &Object) -> bool {
    let key = item.getString("인덱스");
    !item.checkAttr("아이템속성", "단일아이템")
        || key.is_empty()
        || !inventory_contains_index(inventory, &key)
}

/// Insert an acquired item while preserving the runtime representation.
/// Returns false only when accepting it would duplicate a `단일아이템`.
pub(crate) fn store_acquired_object(
    inventory: &mut Object,
    item: Arc<Mutex<Object>>,
    prepend: bool,
) -> bool {
    if item
        .lock()
        .ok()
        .is_some_and(|item| !can_accept_object(inventory, &item))
    {
        return false;
    }
    if absorb_pristine_object(inventory, &item) {
        return true;
    }
    if prepend {
        inventory.objs.insert(0, item);
    } else {
        inventory.objs.push(item);
    }
    true
}

pub(crate) fn add_pristine_count(inventory: &mut Object, key: &str, count: i64) -> bool {
    if key.is_empty() || count <= 0 || !super::is_stackable(key) {
        return false;
    }
    *inventory.inv_stack.entry(key.to_string()).or_insert(0) += count;
    true
}

pub(crate) fn remove_pristine_count(inventory: &mut Object, key: &str, count: i64) -> bool {
    if key.is_empty() || count <= 0 {
        return false;
    }
    let have = inventory.inv_stack.get(key).copied().unwrap_or(0);
    if have < count {
        return false;
    }
    if have == count {
        inventory.inv_stack.remove(key);
    } else {
        inventory.inv_stack.insert(key.to_string(), have - count);
    }
    true
}

/// Store an acquired pristine object as a count. Changed/unique objects stay
/// as objects and preserve their identity.
pub(crate) fn absorb_pristine_object(inventory: &mut Object, item: &Arc<Mutex<Object>>) -> bool {
    let key = item
        .lock()
        .ok()
        .filter(|item| is_pristine_template_object(item))
        .map(|item| item.getString("인덱스"));
    key.is_some_and(|key| add_pristine_count(inventory, &key, 1))
}

/// Split exactly one pristine count into an object for a stateful operation
/// such as equipping, strengthening, event mutation, or putting it in a
/// legacy object-only container.
pub(crate) fn materialize_one(
    inventory: &mut Object,
    key: &str,
    prepend: bool,
) -> Option<Arc<Mutex<Object>>> {
    if !remove_pristine_count(inventory, key, 1) {
        return None;
    }
    let Some((item, _)) = super::object_from_item_json(key) else {
        *inventory.inv_stack.entry(key.to_string()).or_insert(0) += 1;
        return None;
    };
    if prepend {
        inventory.objs.insert(0, item.clone());
    } else {
        inventory.objs.push(item.clone());
    }
    Some(item)
}

/// Python `대여` performs `item.attr = itm["attr"]` on a deep-cloned item.
/// Replace the complete persistent map (while retaining Rust's internal item
/// index, which corresponds to Python's separate `item.index` field) and
/// restore array-shape markers along with scalar values.
pub(crate) fn replace_item_attributes_from_json(item: &mut Object, attrs: &JsonValue) {
    let Some(attributes) = attrs.as_object() else {
        return;
    };
    let index = item.getString("인덱스");
    item.attr.clear();
    item.temp
        .retain(|key, _| !key.starts_with(JSON_ARRAY_MARKER_PREFIX));
    for (field, value) in attributes {
        set_item_json_field(item, field, value);
    }
    if !index.is_empty() && !item.attr.contains_key("인덱스") {
        item.set("인덱스", index);
    }
}

#[cfg(test)]
fn nonempty(value: &Value) -> bool {
    !matches!(value, Value::String(value) if value.is_empty())
}

/// Build the exact item record shape written by Python `Player.save()`.
#[cfg(test)]
pub(super) fn python_item_record(item: &Object, now: f64) -> Option<JsonValue> {
    let index = item.getString("인덱스");
    if index.is_empty() {
        return None;
    }

    let mut record = JsonMap::new();
    record.insert("인덱스".to_string(), JsonValue::String(index));
    record.insert("이름".to_string(), JsonValue::String(item.getName()));
    record.insert("반응이름".to_string(), item_field_to_json(item, "반응이름"));

    for field in ["공격력", "방어력", "기량"] {
        let value = item.get(field);
        if nonempty(&value) {
            record.insert(field.to_string(), plain_value_to_json(&value));
        }
    }
    if item.getBool("inUse") {
        record.insert("상태".to_string(), plain_value_to_json(&item.get("계층")));
    }
    for field in ["옵션", "아이템속성"] {
        let value = item.get(field);
        if nonempty(&value) {
            record.insert(field.to_string(), item_field_to_json(item, field));
        }
    }
    for field in ["확장 이름", "고유번호"] {
        let value = item.get(field);
        if nonempty(&value) {
            record.insert(field.to_string(), plain_value_to_json(&value));
        }
    }
    if item.checkAttr("아이템속성", "단일아이템") {
        record.insert(
            "시간".to_string(),
            serde_json::Number::from_f64(now)
                .map(JsonValue::Number)
                .unwrap_or_else(|| JsonValue::Number(0.into())),
        );
    }
    if item.getString("종류") == "호위" {
        record.insert("체력".to_string(), plain_value_to_json(&item.get("체력")));
    }

    Some(JsonValue::Object(record))
}

/// Compact pristine template copies while retaining complete state for every
/// object that differs from its item JSON template.
pub(crate) fn compact_object_records(inventory: &Object) -> Vec<JsonValue> {
    let mut records = Vec::<JsonValue>::new();
    let mut pristine_positions = std::collections::HashMap::<String, usize>::new();
    let mut template_attributes = std::collections::HashMap::<String, Option<JsonValue>>::new();

    let mut runtime_stacks = positive_stack_entries(&inventory.inv_stack);
    for (index, count) in runtime_stacks.drain(..) {
        let position = records.len();
        records.push(serde_json::json!({"인덱스": index, "수량": count}));
        pristine_positions.insert(index, position);
    }

    for item in &inventory.objs {
        let Ok(item) = item.lock() else { continue };
        let index = item.getString("인덱스");
        if index.is_empty() {
            continue;
        }
        let current = item_attributes_to_json(&item);
        let template = template_attributes.entry(index.clone()).or_insert_with(|| {
            super::object_from_item_json(&index).and_then(|(template, _)| {
                template
                    .lock()
                    .ok()
                    .map(|template| item_attributes_to_json(&template))
            })
        });
        if template.as_ref() == Some(&current) && is_pristine_template_object(&item) {
            if let Some(position) = pristine_positions.get(&index).copied() {
                if let Some(record) = records.get_mut(position).and_then(JsonValue::as_object_mut) {
                    let count = record.get("수량").and_then(JsonValue::as_i64).unwrap_or(1);
                    record.insert("수량".into(), JsonValue::Number((count + 1).into()));
                }
            } else {
                let position = records.len();
                records.push(serde_json::json!({"인덱스": index, "수량": 1}));
                pristine_positions.insert(index, position);
            }
            continue;
        }

        let current_map = current.as_object().cloned().unwrap_or_default();
        let template_map = template
            .as_ref()
            .and_then(JsonValue::as_object)
            .cloned()
            .unwrap_or_default();
        let changed = current_map
            .iter()
            .filter(|(field, value)| match template_map.get(*field) {
                Some(template_value) => template_value != *value,
                None => !is_missing_attribute_default(value),
            })
            .map(|(field, value)| (field.clone(), value.clone()))
            .collect::<JsonMap<_, _>>();
        let removed = template_map
            .keys()
            .filter(|field| !current_map.contains_key(*field))
            .cloned()
            .map(JsonValue::String)
            .collect::<Vec<_>>();
        let mut record = JsonMap::new();
        record.insert("인덱스".into(), JsonValue::String(index));
        record.insert("수량".into(), JsonValue::Number(1.into()));
        if !changed.is_empty() {
            record.insert("변경".into(), JsonValue::Object(changed));
        }
        if !removed.is_empty() {
            record.insert("제거".into(), JsonValue::Array(removed));
        }
        records.push(JsonValue::Object(record));
    }
    records
}

pub(super) fn compact_inventory_records(body: &Body) -> Vec<JsonValue> {
    compact_object_records(&body.object)
}

fn item_index(record: &JsonMap<String, JsonValue>) -> Option<String> {
    match record.get("인덱스")? {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn item_records(root: &JsonMap<String, JsonValue>) -> Vec<&JsonMap<String, JsonValue>> {
    match root.get("아이템") {
        Some(JsonValue::Array(records)) => {
            records.iter().filter_map(JsonValue::as_object).collect()
        }
        // Python explicitly accepts the historical singleton-dict shape.
        Some(JsonValue::Object(record)) => vec![record],
        _ => Vec::new(),
    }
}

fn is_compact_pristine_record(record: &JsonMap<String, JsonValue>) -> bool {
    record
        .keys()
        .all(|field| matches!(field.as_str(), "인덱스" | "수량"))
}

/// Load Python `아이템` records. Exact `{인덱스, 수량}` records remain counted
/// at runtime without constructing one object per copy. Stateful records still
/// call `insert(0, item)`, exactly mirroring `Player.load()`'s reversed order.
pub(super) fn load_python_inventory(body: &mut Body, root: &JsonMap<String, JsonValue>) {
    reset_body_derived_state(body);
    body.object.objs.clear();
    body.object.inv_stack.clear();
    let mut loaded_single_items = std::collections::HashSet::<String>::new();
    for record in item_records(root) {
        let Some(index) = item_index(record) else {
            continue;
        };
        let single_item = is_single_item_template(&index);
        if single_item && !loaded_single_items.insert(index.clone()) {
            continue;
        }
        let count = record
            .get("수량")
            .and_then(JsonValue::as_i64)
            .unwrap_or(1)
            .clamp(1, 100_000);
        let count = if single_item { 1 } else { count };
        if !single_item && is_compact_pristine_record(record) && super::is_stackable(&index) {
            *body.object.inv_stack.entry(index).or_insert(0) += count;
            continue;
        }
        for _ in 0..count {
            let has_delta = record.get("변경").and_then(JsonValue::as_object).is_some()
                || record.get("제거").and_then(JsonValue::as_array).is_some();
            let item = if let Some((item, _)) = super::object_from_item_json(&index) {
                item
            } else if record.get("속성").and_then(JsonValue::as_object).is_some() || has_delta {
                let mut item = Object::new();
                item.set("인덱스", index.clone());
                Arc::new(Mutex::new(item))
            } else {
                continue;
            };
            let mut equipped_stats = None;
            if let Ok(mut item) = item.lock() {
                if let Some(attributes) = record.get("속성") {
                    replace_item_attributes_from_json(&mut item, attributes);
                } else if has_delta {
                    if let Some(removed) = record.get("제거").and_then(JsonValue::as_array) {
                        for field in removed.iter().filter_map(JsonValue::as_str) {
                            if field != "인덱스" {
                                item.attr.remove(field);
                                item.temp.remove(&json_array_marker(field));
                            }
                        }
                    }
                    if let Some(changed) = record.get("변경").and_then(JsonValue::as_object) {
                        for (field, value) in changed {
                            if field != "인덱스" {
                                set_item_json_field(&mut item, field, value);
                            }
                        }
                    }
                } else {
                    for field in [
                        "이름",
                        "반응이름",
                        "고유번호",
                        "공격력",
                        "방어력",
                        "기량",
                        "아이템속성",
                        "옵션",
                        "확장 이름",
                        "체력",
                    ] {
                        if let Some(value) = record.get(field) {
                            if field == "반응이름" {
                                if let JsonValue::String(value) = value {
                                    set_item_json_field(
                                        &mut item,
                                        field,
                                        &JsonValue::Array(vec![JsonValue::String(value.clone())]),
                                    );
                                    continue;
                                }
                            }
                            set_item_json_field(&mut item, field, value);
                        }
                    }
                    if record.contains_key("상태") {
                        item.set("inUse", 1_i64);
                    }
                }
                if item.getBool("inUse") {
                    equipped_stats = Some((
                        item.getInt("방어력") as i32,
                        item.getInt("공격력") as i32,
                        item.getString("종류") == "무기",
                        (record.contains_key("속성") || has_delta || record.contains_key("옵션"))
                            .then(|| item.get_option())
                            .flatten(),
                    ));
                }
            }

            if let Some((armor, attack, is_weapon, options)) = equipped_stats {
                body.armor += armor;
                body.attpower += attack;
                if is_weapon {
                    body.weapon_item = Some(Arc::downgrade(&item));
                }
                if let Some(options) = options {
                    for (name, value) in options {
                        let value = value as i32;
                        match name.as_str() {
                            "힘" => body._str += value,
                            "민첩성" => body._dex += value,
                            "맷집" => body._arm += value,
                            "체력" => body._maxhp += value,
                            "내공" => body._maxmp += value,
                            "필살" => body._critical += value,
                            "운" => body._critical_chance += value,
                            "회피" => body._miss += value,
                            "명중" => body._hit += value,
                            "경험치" => body._exp += value,
                            "마법발견" => body._magic_chance += value,
                            _ => {}
                        }
                    }
                }
            }
            let stack_key = item
                .lock()
                .ok()
                .filter(|item| !item.getBool("inUse") && is_pristine_template_object(item))
                .map(|item| item.getString("인덱스"));
            if let Some(stack_key) = stack_key {
                *body.object.inv_stack.entry(stack_key).or_insert(0) += 1;
            } else {
                body.object.objs.insert(0, item);
            }
        }
    }
}

fn positive_stack_entries(stack: &std::collections::HashMap<String, i64>) -> Vec<(String, i64)> {
    let mut entries = stack
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(key, count)| (key.clone(), *count))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

/// Resolve a displayed name only among keys that are actually present in a
/// counted inventory. This avoids choosing an unrelated template when two
/// item definitions share the same name or reaction alias.
pub(crate) fn find_counted_item_key(
    stack: &std::collections::HashMap<String, i64>,
    name: &str,
) -> Option<String> {
    counted_item_at(stack, name, 1).map(|(key, _)| key)
}

/// Return every positive counted group matching a displayed name or reaction
/// alias in the same deterministic order used by [`counted_item_at`].
pub(crate) fn counted_item_keys(
    stack: &std::collections::HashMap<String, i64>,
    name: &str,
) -> Vec<String> {
    let keys = positive_stack_entries(stack)
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    for exact_name in [true, false] {
        for key in &keys {
            let Some((item, _)) = super::object_from_item_json(key) else {
                continue;
            };
            let matches = item.lock().ok().is_some_and(|item| {
                if exact_name {
                    item.getName() == name
                } else {
                    item.getName() != name
                        && super::reaction_names(&item.getString("반응이름"))
                            .iter()
                            .any(|alias| alias == name)
                }
            });
            if matches {
                result.push(key.clone());
            }
        }
    }
    result
}

/// Resolve a one-based occurrence across all counted template groups that
/// match a display name or reaction alias. Exact display names precede aliases,
/// and keys are stable-sorted inside each class.
pub(crate) fn counted_item_at(
    stack: &std::collections::HashMap<String, i64>,
    name: &str,
    order: i64,
) -> Option<(String, i64)> {
    if order < 1 {
        return None;
    }
    let keys = counted_item_keys(stack, name);
    let mut remaining = order;
    for key in keys {
        let count = stack.get(&key).copied().unwrap_or(0).max(0);
        if remaining <= count {
            return Some((key, remaining));
        }
        remaining -= count;
    }
    None
}

/// Normalize old runtime state before save: valid pristine items stay counted,
/// old stacks of identity-bearing items become objects, and pristine objects
/// created by legacy paths are folded back into counts.
pub(crate) fn materialize_stacks_for_save(body: &mut Body) -> Result<(), Vec<String>> {
    body.object.inv_stack.retain(|_, count| *count > 0);
    let non_stackable = positive_stack_entries(&body.object.inv_stack)
        .into_iter()
        .filter(|(key, _)| !super::is_stackable(key))
        .collect::<Vec<_>>();
    for (key, count) in non_stackable {
        let Some((template, _)) = super::object_from_item_json(&key) else {
            continue;
        };
        body.object.inv_stack.remove(&key);
        let count = if is_single_item_template(&key) {
            i64::from(!inventory_contains_index(&body.object, &key))
        } else {
            count
        };
        for _ in 0..count {
            let item = template
                .lock()
                .ok()
                .map(|template| Arc::new(Mutex::new(template.deepclone())));
            if let Some(item) = item {
                body.object.objs.push(item);
            }
        }
    }
    let pristine_objects = body
        .object
        .objs
        .iter()
        .filter(|item| {
            item.lock()
                .ok()
                .is_some_and(|item| is_pristine_template_object(&item))
        })
        .cloned()
        .collect::<Vec<_>>();
    for item in pristine_objects {
        let key = item
            .lock()
            .ok()
            .map(|item| item.getString("인덱스"))
            .unwrap_or_default();
        if add_pristine_count(&mut body.object, &key, 1) {
            body.object.remove(&item);
        }
    }
    Ok(())
}

/// One-way migration for files written by older Rust builds. Existing
/// `아이템` objects retain their Python runtime order; legacy stack groups are
/// appended in the same key-sorted order formerly used by the inventory view.
/// The original mixed acquisition order was never stored and cannot be
/// reconstructed.
pub(super) fn load_legacy_stacks(body: &mut Body, root: &JsonMap<String, JsonValue>) {
    let Some(stack) = root.get("소지품_수량").and_then(JsonValue::as_object) else {
        return;
    };
    let mut entries = stack
        .iter()
        .filter_map(|(key, count)| count.as_i64().map(|count| (key.clone(), count)))
        .filter(|(_, count)| *count > 0)
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    for (key, count) in entries {
        if super::is_stackable(&key) {
            *body.object.inv_stack.entry(key).or_insert(0) += count;
            continue;
        }
        let Some((template, _)) = super::object_from_item_json(&key) else {
            body.object.inv_stack.insert(key, count);
            continue;
        };
        let count = if is_single_item_template(&key) {
            i64::from(!inventory_contains_index(&body.object, &key))
        } else {
            count
        };
        for _ in 0..count {
            if let Ok(template) = template.lock() {
                body.object
                    .objs
                    .push(Arc::new(Mutex::new(template.deepclone())));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_json_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "muc_inventory_{label}_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn inventory_indexes(body: &Body) -> Vec<String> {
        body.object
            .objs
            .iter()
            .map(|item| item.lock().unwrap().getString("인덱스"))
            .collect()
    }

    #[test]
    fn catalogue_attribute_replacement_matches_python_item_attr_assignment() {
        let mut item = Object::new();
        item.set("인덱스", "시험검");
        item.set("이름", "템플릿검");
        item.set("옵션", "힘 1");
        item.temp.insert(json_array_marker("옵션"), Value::Int(1));

        let attributes = serde_json::json!({
            "이름": "등록검",
            "공격력": 321,
            "반응이름": ["등록검", "등록"],
            "옵션": ["힘 7", "운 3"],
            "아이템속성": "줄수없음"
        });
        replace_item_attributes_from_json(&mut item, &attributes);

        assert_eq!(item.getString("인덱스"), "시험검");
        assert_eq!(item.getString("이름"), "등록검");
        assert_eq!(item.getInt("공격력"), 321);
        assert_eq!(item.getString("종류"), "");
        assert_eq!(
            item_attributes_to_json(&item),
            serde_json::json!({
                "이름": "등록검",
                "공격력": 321,
                "반응이름": ["등록검", "등록"],
                "옵션": ["힘 7", "운 3"],
                "아이템속성": "줄수없음"
            })
        );
    }

    #[test]
    fn authoritative_python_source_reads_compact_records_but_keeps_legacy_save_shape() {
        let player = include_str!("../../objs/player.py");
        let save = player
            .split("    def save(self, mode = True):")
            .nth(1)
            .unwrap();
        let save = save.split("    def saveItems(self):").next().unwrap();
        assert!(save.contains("for item in self.objs:"));
        assert!(save.contains("o['아이템'] = items"));
        assert!(!save.contains("소지품_수량"));

        let load = player.split("    def load(self, path):").nth(1).unwrap();
        let load = load
            .split("    def save(self, mode = True):")
            .next()
            .unwrap();
        assert!(load.contains("if type(items) == dict:"));
        assert!(load.contains("item.get('수량', 1)"));
        assert!(load.contains("item.get('변경', {})"));
        assert!(load.contains("item.get('제거', [])"));
        assert!(load.contains("self.insert(obj)"));
        assert!(!load.contains("소지품_수량"));
    }

    #[test]
    fn python_array_load_reverses_order_and_ignores_saved_oneitem_time() {
        let root = serde_json::json!({
            "아이템": [
                {"인덱스": "1000", "이름": "첫째"},
                {"인덱스": "1001", "이름": "둘째"},
                {"인덱스": "64", "이름": "셋째", "시간": 123.5}
            ]
        });
        let mut body = Body::new();
        load_python_inventory(&mut body, root.as_object().unwrap());
        assert_eq!(inventory_indexes(&body), ["64", "1001", "1000"]);
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "셋째");
        assert_eq!(body.object.objs[0].lock().unwrap().getString("시간"), "");
    }

    #[test]
    fn singleton_item_object_is_accepted_like_python() {
        let root = serde_json::json!({"아이템": {"인덱스": 1000, "이름": "한개"}});
        let mut body = Body::new();
        load_python_inventory(&mut body, root.as_object().unwrap());
        assert_eq!(inventory_indexes(&body), ["1000"]);
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "한개");
    }

    #[test]
    fn item_record_preserves_array_fields_and_python_oneitem_shape() {
        let (item, _) = super::super::object_from_item_json("64").unwrap();
        let mut item = item.lock().unwrap();
        set_item_json_field(
            &mut item,
            "반응이름",
            &serde_json::json!(["성황신검", "성황"]),
        );
        set_item_json_field(&mut item, "옵션", &serde_json::json!(["힘 10", "민첩성 5"]));
        let record = python_item_record(&item, 42.25).unwrap();
        assert_eq!(record["반응이름"], serde_json::json!(["성황신검", "성황"]));
        assert_eq!(record["옵션"], serde_json::json!(["힘 10", "민첩성 5"]));
        assert_eq!(record["시간"], serde_json::json!(42.25));
    }

    #[test]
    fn pristine_consumables_compact_and_modified_items_keep_only_template_delta() {
        let mut body = Body::new();
        for _ in 0..2 {
            let (item, _) = super::super::object_from_item_json("비황석").unwrap();
            body.object.objs.push(item);
        }
        let (modified, _) = super::super::object_from_item_json("비황석").unwrap();
        {
            let mut modified = modified.lock().unwrap();
            modified.set("고유번호", "암기-uuid-1");
            modified.attr.remove("판매가격");
        }
        body.object.objs.push(modified);

        let records = compact_inventory_records(&body);
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0],
            serde_json::json!({"인덱스": "비황석", "수량": 2})
        );
        assert_eq!(records[1]["인덱스"], "비황석");
        assert_eq!(records[1]["수량"], 1);
        assert_eq!(
            records[1]["변경"],
            serde_json::json!({"고유번호": "암기-uuid-1"})
        );
        assert_eq!(records[1]["제거"], serde_json::json!(["판매가격"]));

        let root = serde_json::json!({"아이템": records});
        let mut loaded = Body::new();
        load_python_inventory(&mut loaded, root.as_object().unwrap());
        assert_eq!(loaded.object.objs.len(), 1);
        assert_eq!(loaded.object.inv_stack.get("비황석"), Some(&2));
        assert_eq!(
            loaded
                .object
                .objs
                .iter()
                .filter(|item| item.lock().unwrap().getString("고유번호") == "암기-uuid-1")
                .count(),
            1
        );
        let modified = loaded
            .object
            .objs
            .iter()
            .find(|item| item.lock().unwrap().getString("고유번호") == "암기-uuid-1")
            .unwrap()
            .lock()
            .unwrap();
        assert!(!modified.attr.contains_key("판매가격"));
    }

    #[test]
    fn template_missing_scalar_defaults_are_not_persisted_as_changes() {
        let mut body = Body::new();
        let (item, _) = super::super::object_from_item_json("비황석").unwrap();
        {
            let mut item = item.lock().unwrap();
            item.set("고유번호", "");
            item.set("사용횟수", 0_i64);
            item.set("확장 이름", "빙백");
        }
        body.object.objs.push(item);

        let records = compact_inventory_records(&body);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["변경"], serde_json::json!({"확장 이름": "빙백"}));
    }

    #[test]
    fn compact_pristine_load_keeps_large_quantity_without_runtime_objects() {
        let root = serde_json::json!({
            "아이템": [{"인덱스": "비황석", "수량": 50}]
        });
        let mut body = Body::new();

        load_python_inventory(&mut body, root.as_object().unwrap());

        assert!(body.object.objs.is_empty());
        assert_eq!(body.object.inv_stack.get("비황석"), Some(&50));
    }

    #[test]
    fn counted_occurrence_does_not_count_exact_name_again_as_its_alias() {
        let stack = std::collections::HashMap::from([("비황석".to_string(), 2)]);
        assert_eq!(
            counted_item_at(&stack, "비황석", 2),
            Some(("비황석".to_string(), 2))
        );
        assert_eq!(counted_item_at(&stack, "비황석", 3), None);
    }

    #[test]
    fn delta_quantity_loads_as_individual_stateful_objects() {
        let root = serde_json::json!({
            "아이템": [{
                "인덱스": "비황석",
                "수량": 2,
                "변경": {"확장 이름": "빙백"}
            }]
        });
        let mut body = Body::new();

        load_python_inventory(&mut body, root.as_object().unwrap());

        assert_eq!(body.object.objs.len(), 2);
        assert!(body.object.inv_stack.is_empty());
        assert!(body
            .object
            .objs
            .iter()
            .all(|item| { item.lock().unwrap().getString("확장 이름") == "빙백" }));
    }

    #[test]
    fn legacy_stack_and_pristine_objects_compact_into_runtime_counts() {
        let mut body = Body::new();
        let (existing, _) = super::super::object_from_item_json("1002").unwrap();
        body.object.objs.push(existing);
        body.object.inv_stack.insert("1001".to_string(), 2);
        body.object.inv_stack.insert("1000".to_string(), 3);

        materialize_stacks_for_save(&mut body).unwrap();
        assert!(body.object.objs.is_empty());
        assert_eq!(body.object.inv_stack.get("1000"), Some(&3));
        assert_eq!(body.object.inv_stack.get("1001"), Some(&2));
        assert_eq!(body.object.inv_stack.get("1002"), Some(&1));
    }

    #[test]
    fn legacy_file_groups_follow_python_items_without_claiming_lost_interleaving() {
        let root = serde_json::json!({
            "아이템": [
                {"인덱스": "1000"},
                {"인덱스": "1001"}
            ],
            "소지품_수량": {"1003": 1, "1002": 2}
        });
        let mut body = Body::new();
        load_python_inventory(&mut body, root.as_object().unwrap());
        load_legacy_stacks(&mut body, root.as_object().unwrap());
        assert!(body.object.objs.is_empty());
        assert_eq!(body.object.inv_stack.get("1000"), Some(&1));
        assert_eq!(body.object.inv_stack.get("1001"), Some(&1));
        assert_eq!(body.object.inv_stack.get("1002"), Some(&2));
        assert_eq!(body.object.inv_stack.get("1003"), Some(&1));
    }

    #[test]
    fn unknown_legacy_stack_is_not_silently_discarded() {
        let root = serde_json::json!({"소지품_수량": {"존재하지않는키": 2}});
        let mut body = Body::new();
        load_legacy_stacks(&mut body, root.as_object().unwrap());
        assert_eq!(body.object.inv_stack.get("존재하지않는키"), Some(&2));
        assert!(materialize_stacks_for_save(&mut body).is_ok());
        assert_eq!(body.object.inv_stack.get("존재하지않는키"), Some(&2));
    }

    #[test]
    fn python_rust_round_trip_uses_only_item_array_and_preserves_reversal() {
        let path = temp_json_path("round_trip");
        let source = serde_json::json!({
            "사용자오브젝트": {"이름": "순서검사", "은전": 0},
            "아이템": [
                {"인덱스": "1000", "이름": "첫째", "반응이름": "첫반응"},
                {"인덱스": "1001", "이름": "둘째", "옵션": ["힘 3", "명중 4"]},
                {"인덱스": "64", "이름": "셋째", "시간": 1.25}
            ]
        });
        std::fs::write(&path, serde_json::to_string(&source).unwrap()).unwrap();

        let mut first_load = Body::new();
        assert!(super::super::load_body_from_json(
            &mut first_load,
            &path.to_string_lossy()
        ));
        assert_eq!(inventory_indexes(&first_load), ["64", "1001", "1000"]);

        assert!(super::super::save_body_to_json_without_timestamp(
            &mut first_load,
            &path.to_string_lossy()
        ));
        let saved: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(saved.get("소지품_수량").is_none());
        let names = saved["아이템"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["변경"]["이름"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(names, ["셋째", "둘째", "첫째"]);
        assert_eq!(
            saved["아이템"][2]["변경"]["반응이름"],
            serde_json::json!(["첫반응"])
        );
        assert_eq!(
            saved["아이템"][1]["변경"]["옵션"],
            serde_json::json!(["힘 3", "명중 4"])
        );

        // A Python load of the just-saved array also prepends every item. A
        // second Rust load therefore returns to the original A/B/C order.
        let mut second_load = Body::new();
        assert!(super::super::load_body_from_json(
            &mut second_load,
            &path.to_string_lossy()
        ));
        assert_eq!(inventory_indexes(&second_load), ["1000", "1001", "64"]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_stack_save_keeps_runtime_counts_and_writes_compact_records() {
        let path = temp_json_path("legacy_expand");
        let mut body = Body::new();
        body.set("이름", "레거시확장");
        let (existing, _) = super::super::object_from_item_json("1002").unwrap();
        body.object.objs.push(existing);
        body.object.inv_stack.insert("1000".to_string(), 2);

        assert!(super::super::save_body_to_json_without_timestamp(
            &mut body,
            &path.to_string_lossy()
        ));
        let saved: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(saved.get("소지품_수량").is_none());
        let items = saved["아이템"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], serde_json::json!({"인덱스": "1000", "수량": 2}));
        assert_eq!(items[1], serde_json::json!({"인덱스": "1002", "수량": 1}));
        assert_eq!(body.object.inv_stack.get("1000"), Some(&2));
        assert_eq!(body.object.inv_stack.get("1002"), Some(&1));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn duplicate_single_item_records_load_as_exactly_one_unique_object() {
        let root = serde_json::json!({
            "아이템": [
                {"인덱스": "64", "수량": 3},
                {"인덱스": "64"}
            ],
            "소지품_수량": {"64": 2}
        });
        let mut body = Body::new();
        load_python_inventory(&mut body, root.as_object().unwrap());
        load_legacy_stacks(&mut body, root.as_object().unwrap());

        assert_eq!(inventory_indexes(&body), ["64"]);
        assert!(!body.object.inv_stack.contains_key("64"));
    }

    #[test]
    fn equipped_item_derived_state_rebuilds_once_and_keeps_template_layer() {
        let path = temp_json_path("equipped");
        let source = serde_json::json!({
            "사용자오브젝트": {
                "이름": "장착복원",
                "힘": 10,
                "민첩성": 20,
                "맷집": 30,
                "최고체력": 100,
                "최고내공": 200,
                "필살": 1,
                "운": 2,
                "회피": 3,
                "명중": 4
            },
            "아이템": [{
                "인덱스": "무황성도-1",
                "공격력": 17,
                "방어력": 19,
                "상태": "저장값은계층을덮지않음",
                "옵션": [
                    "힘 1", "민첩성 2", "맷집 3", "체력 4", "내공 5",
                    "필살 6", "운 7", "회피 8", "명중 9", "경험치 10",
                    "마법발견 11", "알수없음 99", "힘 999 여분"
                ]
            }]
        });
        std::fs::write(&path, serde_json::to_string(&source).unwrap()).unwrap();

        let mut body = Body::new();
        body.attpower = 500;
        body.armor = 500;
        body._str = 500;
        assert!(super::super::load_body_from_json(
            &mut body,
            &path.to_string_lossy()
        ));
        assert_eq!(body.attpower, 17);
        assert_eq!(body.armor, 19);
        assert_eq!(body._str, 1);
        assert_eq!(body._dex, 2);
        assert_eq!(body._arm, 3);
        assert_eq!(body._maxhp, 4);
        assert_eq!(body._maxmp, 5);
        assert_eq!(body._critical, 6);
        assert_eq!(body._critical_chance, 7);
        assert_eq!(body._miss, 8);
        assert_eq!(body._hit, 9);
        assert_eq!(body._exp, 10);
        assert_eq!(body._magic_chance, 11);
        let weapon = body.weapon_item.as_ref().unwrap().upgrade().unwrap();
        assert_eq!(weapon.lock().unwrap().getString("계층"), "무기");

        // Reusing the same Body must not double equipment bonuses.
        assert!(super::super::load_body_from_json(
            &mut body,
            &path.to_string_lossy()
        ));
        assert_eq!(body.attpower, 17);
        assert_eq!(body.armor, 19);
        assert_eq!(body._str, 1);
        assert_eq!(body._magic_chance, 11);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn persisted_defense_skill_is_reapplied_after_equipment_reset_without_duplication() {
        let path = temp_json_path("equipment_and_defense_skill");
        let source = serde_json::json!({
            "사용자오브젝트": {
                "이름": "장비방어무공복원",
                "방어무공시전": ["금강불괴 35"]
            },
            "아이템": [{
                "인덱스": "무황성도-1",
                "상태": "무기",
                "옵션": ["힘 1", "맷집 3"]
            }]
        });
        std::fs::write(&path, serde_json::to_string(&source).unwrap()).unwrap();
        let skill = crate::world::get_skill("금강불괴").unwrap();

        let mut body = Body::new();
        for _ in 0..2 {
            assert!(super::super::load_body_from_json(
                &mut body,
                &path.to_string_lossy()
            ));
            assert_eq!(body.active_skills.len(), 1);
            assert_eq!(body.active_skills[0].name, "금강불괴");
            assert_eq!(body.active_skills[0].start_time, 35);
            assert_eq!(body._str, 1 + skill.str_bonus as i32);
            assert_eq!(body._dex, skill.dex_bonus as i32);
            assert_eq!(body._arm, 3 + skill.arm_bonus as i32);
            assert_eq!(body._mp, skill.mp_bonus as i32);
            assert_eq!(body._maxmp, skill.max_mp_bonus as i32);
            assert_eq!(body._hp, skill.hp_bonus as i32);
            assert_eq!(body._maxhp, skill.max_hp_bonus as i32);
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn inventory_view_and_selected_drop_follow_python_loaded_object_order() {
        let root = serde_json::json!({
            "아이템": [
                {"인덱스": "1000", "이름": "첫이름", "반응이름": ["공통"]},
                {"인덱스": "1001", "이름": "둘째이름", "반응이름": ["공통"]},
                {"인덱스": "1002", "이름": "둘째이름", "반응이름": ["공통"]}
            ]
        });
        let mut body = Body::new();
        body.set("이름", "소지품순서검사");
        body.set("은전", 0_i64);
        load_python_inventory(&mut body, root.as_object().unwrap());
        assert_eq!(inventory_indexes(&body), ["1002", "1001", "1000"]);

        let storage = super::super::ScriptStorage::default();
        let (output, special) = storage
            .execute("소지품", &mut body, "", None, None, None)
            .unwrap();
        assert!(special.is_none());
        let grouped = output
            .iter()
            .filter(|line| line.starts_with("\x1b[36m"))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            grouped,
            [
                "\x1b[36m둘째이름 \x1b[36m2개\x1b[37m",
                "\x1b[36m첫이름\x1b[37m"
            ]
        );

        let zone = format!("소지품순서존-{}", std::process::id());
        let room = "1".to_string();
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.get_room_objs_mut(&zone, &room).clear();
            world.get_room_objs_stack_mut(&zone, &room).clear();
            world.set_player_position(
                "소지품순서검사",
                crate::world::PlayerPosition::new(zone.clone(), room.clone()),
            );
        }
        let (_, special) = storage
            .execute("버려", &mut body, "2공통", None, None, None)
            .unwrap();
        assert!(special.is_none());
        // Runtime order was 1002,1001,1000, so Python's second match is 1001.
        assert_eq!(inventory_indexes(&body), ["1002", "1000"]);
        let floor = crate::world::get_world_state()
            .read()
            .unwrap()
            .get_room_objs(&zone, &room);
        assert_eq!(floor.len(), 1);
        assert_eq!(floor[0].lock().unwrap().getString("인덱스"), "1001");

        let mut world = crate::world::get_world_state().write().unwrap();
        world.remove_player_position("소지품순서검사");
        world.get_room_objs_mut(&zone, &room).clear();
        world.get_room_objs_stack_mut(&zone, &room).clear();
    }
}
