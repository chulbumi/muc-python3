//! Python `Player.objs` inventory persistence compatibility.
//!
//! The authoritative Python runtime has no stack inventory. Every owned item
//! is an individual `Item` in `Player.objs`; `Player.save()` writes that list
//! to the `아이템` JSON array, and `Player.load()` calls `insert()` for every
//! array element (therefore reversing the JSON array in memory).  Older Rust
//! builds introduced `Object::inv_stack` and `소지품_수량`.  The helpers in
//! this module expand that lossy representation back into individual objects
//! at the persistence boundary.  A mixed `objs`/`inv_stack` acquisition order
//! cannot be reconstructed after it has already been compressed.

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

fn nonempty(value: &Value) -> bool {
    !matches!(value, Value::String(value) if value.is_empty())
}

/// Build the exact item record shape written by Python `Player.save()`.
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

/// Serialize current runtime inventory order. `materialize_stacks_for_save`
/// must have succeeded before this is called.
pub(super) fn python_inventory_records(body: &Body, now: f64) -> Vec<JsonValue> {
    body.object
        .objs
        .iter()
        .filter_map(|item| {
            item.lock()
                .ok()
                .and_then(|item| python_item_record(&item, now))
        })
        .collect()
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

/// Load Python `아이템` records. Calling `insert(0, item)` rather than push is
/// observable: it exactly mirrors `Player.load()` and reverses JSON order.
pub(super) fn load_python_inventory(body: &mut Body, root: &JsonMap<String, JsonValue>) {
    reset_body_derived_state(body);
    body.object.objs.clear();
    for record in item_records(root) {
        let Some(index) = item_index(record) else {
            continue;
        };
        let Some((item, _)) = super::object_from_item_json(&index) else {
            continue;
        };
        let mut equipped_stats = None;
        if let Ok(mut item) = item.lock() {
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
                    // Player.load converts a legacy string reaction name into
                    // a one-element list before assigning it to the Item.
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
            // Python checks only for key presence and retains the template's
            // 계층; it does not copy the JSON 상태 value into 계층.
            if record.contains_key("상태") {
                item.set("inUse", 1_i64);
                equipped_stats = Some((
                    item.getInt("방어력") as i32,
                    item.getInt("공격력") as i32,
                    item.getString("종류") == "무기",
                    // Python applies options here only when the persisted
                    // record itself contains 옵션, not merely when the item
                    // template has a default option.
                    record
                        .contains_key("옵션")
                        .then(|| item.get_option())
                        .flatten(),
                ));
            }
            // Python deliberately ignores the saved 단일아이템 `시간` field.
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
        body.object.objs.insert(0, item);
    }
}

fn expanded_stack_objects(
    entries: &[(String, i64)],
) -> Result<Vec<Arc<Mutex<Object>>>, Vec<String>> {
    let mut expanded = Vec::new();
    let mut missing = Vec::new();
    for (key, count) in entries {
        if *count <= 0 {
            continue;
        }
        let Some((template, _)) = super::object_from_item_json(key) else {
            missing.push(key.clone());
            continue;
        };
        for _ in 0..*count {
            let cloned = match template.lock() {
                Ok(template) => Arc::new(Mutex::new(template.deepclone())),
                Err(_) => {
                    missing.push(key.clone());
                    break;
                }
            };
            expanded.push(cloned);
        }
    }
    if missing.is_empty() {
        Ok(expanded)
    } else {
        missing.sort();
        missing.dedup();
        Err(missing)
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

/// Convert any runtime-created Rust stack into Python item objects before a
/// save or an order-sensitive inventory command. New acquisitions belong at
/// the front because Python uses `ob.insert(item)`. With more than one legacy
/// stack key their original interleaving is already unknowable; sorted keys
/// provide only a deterministic migration, not a claim of recovered order.
pub(crate) fn materialize_stacks_for_save(body: &mut Body) -> Result<(), Vec<String>> {
    let entries = positive_stack_entries(&body.object.inv_stack);
    if entries.is_empty() {
        body.object.inv_stack.retain(|_, count| *count > 0);
        return Ok(());
    }
    let mut expanded = expanded_stack_objects(&entries)?;
    expanded.append(&mut body.object.objs);
    body.object.objs = expanded;
    body.object.inv_stack.clear();
    Ok(())
}

/// One-way migration for files written by older Rust builds. Existing
/// `아이템` objects retain their Python runtime order; legacy stack groups are
/// appended in the same key-sorted order formerly used by the inventory view.
/// The original mixed acquisition order was never stored and cannot be
/// reconstructed.
pub(super) fn load_legacy_stacks(body: &mut Body, root: &JsonMap<String, JsonValue>) {
    body.object.inv_stack.clear();
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
        match expanded_stack_objects(&[(key.clone(), count)]) {
            Ok(mut expanded) => body.object.objs.append(&mut expanded),
            Err(_) => {
                // Do not silently destroy an unknown legacy key. It remains
                // unreadable by Python, and a later save will fail preflight.
                body.object.inv_stack.insert(key, count);
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
    fn authoritative_python_source_has_only_individual_item_records() {
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
    fn legacy_stack_materializes_as_individual_objects_without_losing_count() {
        let mut body = Body::new();
        let (existing, _) = super::super::object_from_item_json("1002").unwrap();
        body.object.objs.push(existing);
        body.object.inv_stack.insert("1001".to_string(), 2);
        body.object.inv_stack.insert("1000".to_string(), 3);

        materialize_stacks_for_save(&mut body).unwrap();
        assert!(body.object.inv_stack.is_empty());
        assert_eq!(
            inventory_indexes(&body),
            ["1000", "1000", "1000", "1001", "1001", "1002"]
        );
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
        assert_eq!(
            inventory_indexes(&body),
            ["1001", "1000", "1002", "1002", "1003"]
        );
        assert!(body.object.inv_stack.is_empty());
    }

    #[test]
    fn unknown_legacy_stack_is_not_silently_discarded() {
        let root = serde_json::json!({"소지품_수량": {"존재하지않는키": 2}});
        let mut body = Body::new();
        load_legacy_stacks(&mut body, root.as_object().unwrap());
        assert_eq!(body.object.inv_stack.get("존재하지않는키"), Some(&2));
        assert!(materialize_stacks_for_save(&mut body).is_err());
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
            .map(|item| item["이름"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(names, ["셋째", "둘째", "첫째"]);
        assert_eq!(
            saved["아이템"][2]["반응이름"],
            serde_json::json!(["첫반응"])
        );
        assert_eq!(
            saved["아이템"][1]["옵션"],
            serde_json::json!(["힘 3", "명중 4"])
        );
        assert!(saved["아이템"][0]["시간"].as_f64().is_some());

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
    fn legacy_stack_save_expands_to_python_item_records_in_temp_file() {
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
        let indexes = saved["아이템"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["인덱스"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(indexes, ["1000", "1000", "1002"]);
        assert!(body.object.inv_stack.is_empty());
        let _ = std::fs::remove_file(path);
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
