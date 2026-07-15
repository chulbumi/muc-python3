//! Python `cmds/넣어.py`, `cmds/꺼내.py`, and `objs/box.py` parity.
//!
//! `Box` is a Python object with an ordered `objs` list.  Rust keeps the
//! equivalent box attributes and children in `Object` hash maps / `objs` and
//! marks the runtime type in `temp`.  User-visible text is deliberately not
//! stored here; the two Rhai commands render every message and ANSI byte.

use crate::object::{Object, Value};
use crate::player::Body;
use crate::world::get_world_state;
use rhai::{Dynamic, Engine};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const BOX_TYPE_MARKER: &str = "_python_box";
const BOX_LOAD_SUPPORTED_KEY: &str = "_python_box_load_supported";
const BOX_INDEX_KEY: &str = "_python_box_index";
const BOX_PATH_KEY: &str = "_python_box_path";
const ARRAY_MARKER_PREFIX: &str = "_python_json_array:";
const BOX_DELIVERY_REQUESTS: &str = "_box_delivery_requests";

thread_local! {
    static PRECOMPUTED_BOX_CONTEXT: RefCell<Option<rhai::Map>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub(crate) struct BoxDelivery {
    pub connection_id: String,
    pub raw_text: String,
}

pub(crate) fn build_box_observer_snapshot(
    connection_id: String,
    body: &Body,
    interactive: i32,
) -> Dynamic {
    let config = super::parse_config_string(&body.get_string("설정상태"));
    let mut observer = rhai::Map::new();
    observer.insert("id".into(), Dynamic::from(connection_id));
    observer.insert("name".into(), Dynamic::from(body.get_name()));
    observer.insert(
        "reaction_names".into(),
        Dynamic::from(
            super::reaction_names(&body.get_string("반응이름"))
                .into_iter()
                .map(Dynamic::from)
                .collect::<rhai::Array>(),
        ),
    );
    observer.insert(
        "transparent".into(),
        Dynamic::from(body.get_int("투명상태") == 1),
    );
    observer.insert("hp".into(), Dynamic::from(body.get_hp()));
    observer.insert("max_hp".into(), Dynamic::from(body.get_max_hp()));
    observer.insert("mp".into(), Dynamic::from(body.get_mp()));
    observer.insert("max_mp".into(), Dynamic::from(body.get_max_mp()));
    observer.insert(
        "show_prompt".into(),
        Dynamic::from(interactive == 1 && config.get("엘피출력").map(String::as_str) != Some("1")),
    );
    Dynamic::from(observer)
}

pub(crate) fn set_precomputed_box_context(self_id: String, observers: rhai::Array) {
    let mut context = rhai::Map::new();
    context.insert("self_id".into(), Dynamic::from(self_id));
    context.insert("players".into(), Dynamic::from(observers));
    PRECOMPUTED_BOX_CONTEXT.with(|slot| *slot.borrow_mut() = Some(context));
}

pub(crate) fn clear_precomputed_box_context() {
    PRECOMPUTED_BOX_CONTEXT.with(|slot| *slot.borrow_mut() = None);
}

pub(crate) fn take_box_deliveries(body: &mut Body) -> Vec<BoxDelivery> {
    body.temp_mut()
        .remove(BOX_DELIVERY_REQUESTS)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

fn box_context_knows(connection_id: &str) -> bool {
    if connection_id.is_empty() {
        return false;
    }
    PRECOMPUTED_BOX_CONTEXT.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|context| context.get("players"))
            .and_then(|players| players.clone().try_cast::<rhai::Array>())
            .unwrap_or_default()
            .iter()
            .any(|player| {
                player
                    .clone()
                    .try_cast::<rhai::Map>()
                    .and_then(|player| player.get("id").cloned())
                    .and_then(|id| id.into_string().ok())
                    .is_some_and(|id| id == connection_id)
            })
    })
}

fn connected_player_occurrences(name: &str, query: &str) -> i64 {
    PRECOMPUTED_BOX_CONTEXT.with(|slot| {
        let player = slot
            .borrow()
            .as_ref()
            .and_then(|context| context.get("players"))
            .and_then(|players| players.clone().try_cast::<rhai::Array>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|player| player.try_cast::<rhai::Map>())
            .find(|player| {
                player
                    .get("name")
                    .and_then(|value| value.clone().into_string().ok())
                    .as_deref()
                    == Some(name)
            });
        let Some(player) = player else {
            return i64::from(name == query);
        };
        if player
            .get("transparent")
            .and_then(|value| value.as_bool().ok())
            .unwrap_or(false)
        {
            return 0;
        }
        let aliases = player
            .get("reaction_names")
            .and_then(|value| value.clone().try_cast::<rhai::Array>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.into_string().ok())
            .collect::<Vec<_>>();
        if name == query || aliases.iter().any(|alias| alias == query) {
            1
        } else {
            aliases
                .iter()
                .filter(|alias| alias.starts_with(query))
                .count() as i64
        }
    })
}

fn array_marker(field: &str) -> String {
    format!("{ARRAY_MARKER_PREFIX}{field}")
}

fn has_array_shape(object: &Object, field: &str) -> bool {
    object.temp.contains_key(&array_marker(field))
}

fn python_sequence(object: &Object, field: &str) -> Vec<String> {
    let raw = object.getString(field);
    if has_array_shape(object, field) {
        raw.split('\n').map(str::to_string).collect()
    } else {
        raw.chars().map(|character| character.to_string()).collect()
    }
}

fn python_contains(object: &Object, field: &str, value: &str) -> bool {
    let raw = object.getString(field);
    if has_array_shape(object, field) {
        raw.split('\n').any(|entry| entry == value)
    } else {
        raw.contains(value)
    }
}

fn python_nonempty(value: &Value) -> bool {
    !matches!(value, Value::String(value) if value.is_empty())
}

fn direct_integer(object: &Object, field: &str) -> Option<i64> {
    match object.get(field) {
        Value::Int(value) => Some(value),
        Value::Float(value) if value.is_finite() && value.fract() == 0.0 => Some(value as i64),
        // Python arithmetic/comparison against an empty or numeric string
        // raises TypeError.  Do not silently coerce it into a new branch.
        Value::Float(_) | Value::String(_) => None,
    }
}

fn parse_int_prefix(value: &str) -> i64 {
    if value.is_empty() {
        return 0;
    }
    if let Ok(value) = value.parse::<i64>() {
        return value;
    }
    let mut digits = String::new();
    for character in value.chars() {
        if let Some(digit) = character.to_digit(10) {
            digits.push(char::from_digit(digit, 10).unwrap_or('0'));
        } else {
            break;
        }
    }
    if digits.is_empty() {
        0
    } else {
        digits.parse().unwrap_or(0)
    }
}

fn python_is_digit_string(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

fn python_name_order(value: &str) -> (String, i64) {
    let order = parse_int_prefix(value);
    if order == 0 {
        return (value.to_string(), 1);
    }
    for (byte_index, character) in value.char_indices() {
        if character.to_digit(10).is_none() {
            return (value[byte_index..].to_string(), order);
        }
    }
    // Python leaves a digit-only name unchanged when the loop never reaches
    // a non-digit character.
    (value.to_string(), order)
}

#[derive(Clone, Debug)]
struct ItemDetails {
    name: String,
    particle_source: String,
    ansi: String,
    index: String,
    kind: String,
    purchase_name: String,
    weight: Option<i64>,
    in_use: bool,
    hidden: bool,
    one_item: bool,
    cannot_store: bool,
    public_blocked: bool,
    has_option: bool,
}

fn item_details(item: &Object) -> ItemDetails {
    let name = item.getName();
    let reaction_sequence = python_sequence(item, "반응이름");
    let particle_source = if crate::hangul::is_han(&name) {
        name.clone()
    } else {
        reaction_sequence
            .first()
            .cloned()
            .unwrap_or_else(|| name.clone())
    };
    ItemDetails {
        name,
        particle_source,
        ansi: item.getString("안시"),
        index: item.getString("인덱스"),
        kind: item.getString("종류"),
        purchase_name: item.getString("구매이름"),
        weight: direct_integer(item, "무게"),
        in_use: item.getBool("inUse"),
        hidden: python_contains(item, "아이템속성", "출력안함"),
        one_item: python_contains(item, "아이템속성", "단일아이템"),
        cannot_store: python_contains(item, "아이템속성", "보관못함"),
        public_blocked: ["줄수없음", "버리지못함", "팔지못함", "부수지못함"]
            .iter()
            .any(|attribute| python_contains(item, "아이템속성", attribute)),
        has_option: python_nonempty(&item.get("옵션")),
    }
}

fn matches_item_name(item: &Object, name: &str) -> bool {
    item.getName() == name || python_contains(item, "반응이름", name)
}

fn prefix_matches_item(item: &Object, name: &str) -> bool {
    python_sequence(item, "반응이름")
        .iter()
        .any(|alias| alias.starts_with(name))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TransferGroup {
    name: String,
    particle_source: String,
    ansi: String,
    count: i64,
}

fn increment_group(groups: &mut Vec<TransferGroup>, details: &ItemDetails) {
    if let Some(group) = groups.iter_mut().find(|group| group.name == details.name) {
        group.count += 1;
    } else {
        groups.push(TransferGroup {
            name: details.name.clone(),
            particle_source: details.particle_source.clone(),
            ansi: details.ansi.clone(),
            count: 1,
        });
    }
}

fn increment_group_count(groups: &mut Vec<TransferGroup>, details: &ItemDetails, count: i64) {
    if count <= 0 {
        return;
    }
    if let Some(group) = groups.iter_mut().find(|group| group.name == details.name) {
        group.count += count;
    } else {
        groups.push(TransferGroup {
            name: details.name.clone(),
            particle_source: details.particle_source.clone(),
            ansi: details.ansi.clone(),
            count,
        });
    }
}

fn stack_details(key: &str) -> Option<ItemDetails> {
    let (item, _) = super::object_from_item_json(key)?;
    let item = item.lock().ok()?;
    Some(item_details(&item))
}

fn inventory_unit_count(inventory: &Object) -> i64 {
    inventory.objs.len() as i64
        + inventory
            .inv_stack
            .values()
            .filter(|count| **count > 0)
            .sum::<i64>()
}

fn groups_to_dynamic(groups: Vec<TransferGroup>) -> rhai::Array {
    groups
        .into_iter()
        .map(|group| {
            let mut value = rhai::Map::new();
            value.insert("name".into(), Dynamic::from(group.name));
            value.insert(
                "particle_source".into(),
                Dynamic::from(group.particle_source),
            );
            value.insert("ansi".into(), Dynamic::from(group.ansi));
            value.insert("count".into(), Dynamic::from(group.count));
            Dynamic::from(value)
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransferStatus {
    Ok,
    NoBox,
    Unsupported,
    NotExpandable,
    NotEnoughMoney,
    BoxFull,
    Nothing,
    ItemNotFound,
    CannotStore,
    TooHeavy,
    ItemLimit,
}

impl TransferStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NoBox => "no_box",
            Self::Unsupported => "unsupported",
            Self::NotExpandable => "not_expandable",
            Self::NotEnoughMoney => "not_enough_money",
            Self::BoxFull => "box_full",
            Self::Nothing => "nothing",
            Self::ItemNotFound => "item_not_found",
            Self::CannotStore => "cannot_store",
            Self::TooHeavy => "too_heavy",
            Self::ItemLimit => "item_limit",
        }
    }
}

#[derive(Clone, Debug)]
struct TransferResult {
    status: TransferStatus,
    box_name: String,
    money: i64,
    groups: Vec<TransferGroup>,
}

impl TransferResult {
    fn status(status: TransferStatus) -> Self {
        Self {
            status,
            box_name: String::new(),
            money: 0,
            groups: Vec::new(),
        }
    }

    fn for_box(status: TransferStatus, box_name: String) -> Self {
        Self {
            status,
            box_name,
            money: 0,
            groups: Vec::new(),
        }
    }

    fn into_dynamic(self) -> Dynamic {
        let mut result = rhai::Map::new();
        result.insert("status".into(), Dynamic::from(self.status.as_str()));
        result.insert("box_name".into(), Dynamic::from(self.box_name));
        result.insert("money".into(), Dynamic::from(self.money));
        result.insert(
            "groups".into(),
            Dynamic::from(groups_to_dynamic(self.groups)),
        );
        Dynamic::from(result)
    }
}

trait OneItemActions {
    fn keep(&mut self, index: &str, location: &str);
    fn have(&mut self, index: &str, owner: &str);
}

struct PersistentOneItemActions;

impl OneItemActions for PersistentOneItemActions {
    fn keep(&mut self, index: &str, location: &str) {
        let _ = crate::oneitem::oneitem_keep(index, location);
    }

    fn have(&mut self, index: &str, owner: &str) {
        let _ = crate::oneitem::oneitem_have(index, owner);
    }
}

fn mark_box(object: &mut Object, index: &str, path: &Path) {
    object
        .temp
        .insert(BOX_TYPE_MARKER.to_string(), Value::Int(1));
    object
        .temp
        .insert(BOX_INDEX_KEY.to_string(), Value::String(index.to_string()));
    object.temp.insert(
        BOX_PATH_KEY.to_string(),
        Value::String(path.to_string_lossy().to_string()),
    );
    object
        .temp
        .insert(BOX_LOAD_SUPPORTED_KEY.to_string(), Value::Int(1));
}

fn is_box(object: &Object) -> bool {
    object.getTemp(BOX_TYPE_MARKER) == Value::Int(1)
}

fn box_load_supported(object: &Object) -> bool {
    object.getTemp(BOX_LOAD_SUPPORTED_KEY) == Value::Int(1)
}

fn box_path(object: &Object) -> Option<PathBuf> {
    match object.getTemp(BOX_PATH_KEY) {
        Value::String(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => None,
    }
}

fn json_item_index(record: &JsonMap<String, JsonValue>) -> Option<String> {
    match record.get("인덱스")? {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn box_item_records(
    root: &JsonMap<String, JsonValue>,
) -> Result<Vec<&JsonMap<String, JsonValue>>, ()> {
    match root.get("아이템") {
        Some(JsonValue::Array(records)) => records
            .iter()
            .map(JsonValue::as_object)
            .collect::<Option<Vec<_>>>()
            .ok_or(()),
        Some(JsonValue::Object(record)) => Ok(vec![record]),
        None => Ok(Vec::new()),
        Some(_) => Err(()),
    }
}

fn load_box_with<F>(index: &str, path: &Path, mut load_item: F) -> Object
where
    F: FnMut(&str) -> Option<Arc<Mutex<Object>>>,
{
    let mut box_object = Object::new();
    mark_box(&mut box_object, index, path);

    let Some(root) = std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<JsonValue>(&content).ok())
        .and_then(|value| value.as_object().cloned())
    else {
        // Room.create() ignores Box.create()'s false return and still inserts
        // the empty Box instance into room.objs.
        return box_object;
    };
    let Some(attributes) = root.get("상자정보").and_then(JsonValue::as_object) else {
        return box_object;
    };
    for (field, value) in attributes {
        super::inventory_compat::set_item_json_field(&mut box_object, field, value);
    }

    let records = match box_item_records(&root) {
        Ok(records) => records,
        Err(()) => {
            box_object
                .temp
                .insert(BOX_LOAD_SUPPORTED_KEY.to_string(), Value::Int(0));
            return box_object;
        }
    };
    for record in records {
        let Some(index) = json_item_index(record) else {
            box_object
                .temp
                .insert(BOX_LOAD_SUPPORTED_KEY.to_string(), Value::Int(0));
            return box_object;
        };
        let count = record
            .get("수량")
            .and_then(JsonValue::as_i64)
            .unwrap_or(1)
            .clamp(1, 100_000);
        for _ in 0..count {
            let Some(item) = load_item(&index) else {
                continue;
            };
            let Ok(mut item_value) = item.lock() else {
                box_object
                    .temp
                    .insert(BOX_LOAD_SUPPORTED_KEY.to_string(), Value::Int(0));
                return box_object;
            };
            if let Some(removed) = record.get("제거").and_then(JsonValue::as_array) {
                for field in removed.iter().filter_map(JsonValue::as_str) {
                    if field != "인덱스" {
                        item_value.attr.remove(field);
                    }
                }
            }
            if let Some(changed) = record.get("변경").and_then(JsonValue::as_object) {
                for (field, value) in changed {
                    if field != "인덱스" {
                        super::inventory_compat::set_item_json_field(&mut item_value, field, value);
                    }
                }
            } else {
                for field in [
                    "확장 이름",
                    "이름",
                    "고유번호",
                    "반응이름",
                    "공격력",
                    "방어력",
                    "기량",
                    "옵션",
                    "아이템속성",
                    "시간",
                ] {
                    if let Some(value) = record.get(field) {
                        super::inventory_compat::set_item_json_field(&mut item_value, field, value);
                    }
                }
            }
            drop(item_value);
            let _ = super::inventory_compat::store_acquired_object(&mut box_object, item, true);
        }
    }
    box_object
}

fn runtime_load_item(index: &str) -> Option<Arc<Mutex<Object>>> {
    super::object_from_item_json(index).map(|(item, _)| item)
}

/// Create the usable fixed-room Box when its runtime save file does not yet
/// exist. The installation name selects an exact item template when present
/// (for example 무기보관함), otherwise the shared 보관함 template supplies
/// the capacity/expansion defaults.
fn load_installed_box(index: &str, installation_name: &str, box_root: &Path) -> Object {
    let path = box_root.join(format!("{index}.json"));
    let loaded = load_box_with(index, &path, runtime_load_item);
    if !loaded.getName().is_empty() {
        return loaded;
    }

    let mut box_object = Object::new();
    for template_name in [installation_name, "보관함"] {
        let Some((template, _)) = super::object_from_item_json(template_name) else {
            continue;
        };
        let Ok(template) = template.lock() else {
            continue;
        };
        box_object.attr = template.attr.clone();
        box_object.temp = template.temp.clone();
        break;
    }
    box_object.set("이름", installation_name);
    if installation_name.contains("무기") {
        box_object.set("보관종류", "무기");
    } else if installation_name.contains("방어구") {
        box_object.set("보관종류", "방어구");
    }
    mark_box(&mut box_object, index, &path);
    box_object
}

fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Int(value) => JsonValue::Number((*value).into()),
        Value::Float(value) => serde_json::Number::from_f64(*value)
            .map(JsonValue::Number)
            .unwrap_or_else(|| JsonValue::Number(0.into())),
        Value::String(value) => JsonValue::String(value.clone()),
    }
}

fn box_attributes_to_json(box_object: &Object) -> JsonValue {
    let mut attributes = JsonMap::new();
    for (field, value) in &box_object.attr {
        let value = if has_array_shape(box_object, field) {
            super::inventory_compat::item_field_to_json(box_object, field)
        } else {
            value_to_json(value)
        };
        attributes.insert(field.clone(), value);
    }
    JsonValue::Object(attributes)
}

fn box_item_record(item: &Object) -> Option<JsonValue> {
    let index = item.getString("인덱스");
    if index.is_empty() {
        return None;
    }
    let mut record = JsonMap::new();
    record.insert("인덱스".to_string(), JsonValue::String(index));
    record.insert(
        "이름".to_string(),
        JsonValue::String(item.getString("이름")),
    );
    record.insert(
        "반응이름".to_string(),
        super::inventory_compat::item_field_to_json(item, "반응이름"),
    );
    for field in ["공격력", "방어력", "기량"] {
        let value = item.get(field);
        if python_nonempty(&value) {
            record.insert(field.to_string(), value_to_json(&value));
        }
    }
    for field in ["옵션", "아이템속성"] {
        let value = item.get(field);
        if python_nonempty(&value) {
            record.insert(
                field.to_string(),
                super::inventory_compat::item_field_to_json(item, field),
            );
        }
    }
    for field in ["확장 이름", "시간", "고유번호"] {
        let value = item.get(field);
        if python_nonempty(&value) {
            record.insert(field.to_string(), value_to_json(&value));
        }
    }
    Some(JsonValue::Object(record))
}

fn box_records(box_object: &Object) -> Option<Vec<JsonValue>> {
    box_object
        .objs
        .iter()
        .all(recordable_item)
        .then(|| super::inventory_compat::compact_object_records(box_object))
}

fn recordable_item(item: &Arc<Mutex<Object>>) -> bool {
    item.lock()
        .ok()
        .and_then(|item| box_item_record(&item))
        .is_some()
}

fn can_save_after_put(box_object: &Object, selected: &[Arc<Mutex<Object>>]) -> bool {
    box_path(box_object).is_some()
        && box_object.objs.iter().all(recordable_item)
        && selected.iter().all(recordable_item)
}

fn can_save_after_take(box_object: &Object, selected: &[Arc<Mutex<Object>>]) -> bool {
    box_path(box_object).is_some()
        && box_object
            .objs
            .iter()
            .filter(|item| {
                !selected
                    .iter()
                    .any(|selected_item| Arc::ptr_eq(item, selected_item))
            })
            .all(recordable_item)
}

fn save_box(box_object: &Object) -> bool {
    let Some(path) = box_path(box_object) else {
        return false;
    };
    // Python appends another `.json` to an already suffixed path, making a
    // newly installed box impossible to reload. Rust intentionally repairs
    // that source bug and writes the path that the loader actually reads.
    let output_path = path;
    let Some(items) = box_records(box_object) else {
        // Never turn an unrepresentable child into silent persistent loss.
        return false;
    };
    let mut root = JsonMap::new();
    root.insert("상자정보".to_string(), box_attributes_to_json(box_object));
    root.insert("아이템".to_string(), JsonValue::Array(items));
    serde_json::to_string_pretty(&JsonValue::Object(root))
        .ok()
        .is_some_and(|serialized| std::fs::write(output_path, serialized).is_ok())
}

pub(crate) fn prepare_installed_box(box_object: &mut Object, owner: &str, name: &str) -> bool {
    let index = format!("{owner}_{name}");
    let path = Path::new("data/box").join(format!("{index}.json"));
    mark_box(box_object, &index, &path);
    box_object.set("인덱스", index);
    box_object.set("경로", path.to_string_lossy().to_string());
    save_box(box_object)
}

pub(crate) fn register_installed_box(zone: &str, room: &str, object: Arc<Mutex<Object>>) {
    if let Ok(mut registry) = registry().lock() {
        registry.ensure_room(zone, room);
        registry
            .rooms
            .entry(format!("{zone}:{room}"))
            .or_default()
            .insert(0, object.clone());
    }
    if let Ok(mut world) = get_world_state().write() {
        world.record_box(zone, room, &object);
    }
}

#[derive(Default)]
struct BoxRegistry {
    rooms: HashMap<String, Vec<Arc<Mutex<Object>>>>,
    loaded_rooms: HashSet<String>,
}

impl BoxRegistry {
    fn ensure_room(&mut self, zone: &str, room: &str) {
        let room_key = format!("{zone}:{room}");
        if self.loaded_rooms.contains(&room_key) {
            return;
        }
        self.loaded_rooms.insert(room_key.clone());

        // Python skips every installed Box in difficulty zones whose zone
        // name ends in a digit.
        if zone
            .chars()
            .last()
            .is_some_and(|value| value.is_ascii_digit())
        {
            self.rooms.entry(room_key).or_default();
            return;
        }

        let path = Path::new("data/map")
            .join(zone)
            .join(format!("{room}.json"));
        let Some(info) = std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str::<JsonValue>(&content).ok())
            .and_then(|root| root.get("맵정보").and_then(JsonValue::as_object).cloned())
        else {
            self.rooms.entry(room_key).or_default();
            return;
        };

        let guild_owner = info
            .get("방파주인")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        let owner = if guild_owner.is_empty() {
            info.get("주인").and_then(JsonValue::as_str).unwrap_or("")
        } else {
            guild_owner
        };
        let installation_names = python_installation_names(info.get("설치리스트"));

        let boxes = self.rooms.entry(room_key).or_default();
        for name in installation_names {
            let index = format!("{owner}_{name}");
            let box_object = Arc::new(Mutex::new(load_installed_box(
                &index,
                &name,
                Path::new("data/box"),
            )));
            // Python room.insert(box) prepends on every iteration.
            boxes.insert(0, box_object);
        }
    }
}

fn python_installation_names(value: Option<&JsonValue>) -> Vec<String> {
    match value {
        Some(JsonValue::Array(values)) => values
            .iter()
            .filter_map(JsonValue::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>(),
        // Room.create() directly iterates a string, yielding characters.
        Some(JsonValue::String(value)) => value
            .chars()
            .map(|character| character.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

fn registry() -> &'static Mutex<BoxRegistry> {
    static REGISTRY: OnceLock<Mutex<BoxRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(BoxRegistry::default()))
}

/// Ordered installed Box objects for another room-object resolver. Empty Box
/// instances whose Python load failed are retained exactly as Room.create()
/// retains them; callers naturally will not match their empty name/aliases.
pub(super) fn installed_boxes_for_room(zone: &str, room: &str) -> Option<Vec<Arc<Mutex<Object>>>> {
    let boxes = {
        let mut registry = registry().lock().ok()?;
        registry.ensure_room(zone, room);
        registry
            .rooms
            .get(&format!("{zone}:{room}"))
            .cloned()
            .unwrap_or_default()
    };
    Some(boxes)
}

#[derive(Clone, Debug)]
struct NamedRoomQuery {
    name: String,
    order: i64,
}

fn named_room_query(raw: &str) -> Option<NamedRoomQuery> {
    let raw = if raw.trim() == "." { "1" } else { raw };
    // Room.findObjName's digit-only branch can return only a live non-type-7
    // mob.  It can never return a Box.
    if python_is_digit_string(raw) {
        return None;
    }
    let (name, order) = python_name_order(raw);
    Some(NamedRoomQuery { name, order })
}

fn object_matches_named_query(object: &Object, query: &NamedRoomQuery) -> bool {
    if object.getInt("투명상태") == 1 {
        return false;
    }
    matches_item_name(object, &query.name) || prefix_matches_item(object, &query.name)
}

fn box_has_any_match(boxes: &[Arc<Mutex<Object>>], query: &NamedRoomQuery) -> bool {
    boxes.iter().any(|candidate| {
        candidate.lock().ok().is_some_and(|candidate| {
            is_box(&candidate) && object_matches_named_query(&candidate, query)
        })
    })
}

fn find_box_without_competitors(
    boxes: &[Arc<Mutex<Object>>],
    query: &NamedRoomQuery,
) -> Option<Arc<Mutex<Object>>> {
    let mut exact_count = 0_i64;
    let mut prefix_count = 0_i64;
    for candidate in boxes {
        let Ok(candidate_value) = candidate.lock() else {
            return None;
        };
        if !is_box(&candidate_value) || candidate_value.getInt("투명상태") == 1 {
            continue;
        }
        if matches_item_name(&candidate_value, &query.name) {
            exact_count += 1;
            if exact_count == query.order {
                return Some(candidate.clone());
            }
        } else {
            for alias in python_sequence(&candidate_value, "반응이름") {
                if alias.starts_with(&query.name) {
                    prefix_count += 1;
                    if prefix_count == query.order {
                        return Some(candidate.clone());
                    }
                }
            }
        }
    }
    None
}

fn reaction_list_matches(reactions: &[String], name: &str) -> bool {
    reactions
        .iter()
        .any(|reaction| reaction == name || reaction.starts_with(name))
}

fn room_has_non_box_competitor(body: &Body, query: &NamedRoomQuery) -> bool {
    let Ok(world) = get_world_state().read() else {
        return true;
    };
    let Some(position) = world.get_player_position(&body.get_name()) else {
        return true;
    };

    if world
        .get_players_in_room(&position.zone, &position.room)
        .iter()
        .any(|player_name| player_name == &query.name)
    {
        return true;
    }

    for item in world.get_room_objs(&position.zone, &position.room) {
        let Ok(item) = item.lock() else {
            return true;
        };
        if object_matches_named_query(&item, query) {
            return true;
        }
    }
    for (key, count) in world.get_room_objs_stack(&position.zone, &position.room) {
        if count <= 0 {
            continue;
        }
        let Some((item, _)) = super::object_from_item_json(&key) else {
            // A compressed floor object has no recoverable unified order.
            return true;
        };
        let Ok(item) = item.lock() else {
            return true;
        };
        if object_matches_named_query(&item, query) {
            return true;
        }
    }

    for mob in world
        .mob_cache
        .get_all_mobs_in_room(&position.zone, &position.room)
    {
        let eligible = if query.name == "시체" {
            !mob.alive
        } else {
            mob.alive
        };
        if !eligible {
            continue;
        }
        let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
            return true;
        };
        if mob.name == query.name || reaction_list_matches(&data.reaction_names, &query.name) {
            return true;
        }
    }
    false
}

fn resolve_box(body: &Body, raw_name: &str) -> Result<Arc<Mutex<Object>>, TransferStatus> {
    let Some(query) = named_room_query(raw_name) else {
        return Err(TransferStatus::NoBox);
    };
    let (zone, room) = {
        let world = get_world_state()
            .read()
            .map_err(|_| TransferStatus::Unsupported)?;
        let position = world
            .get_player_position(&body.get_name())
            .ok_or(TransferStatus::NoBox)?;
        (position.zone.clone(), position.room.clone())
    };
    let boxes = {
        let mut registry = registry().lock().map_err(|_| TransferStatus::Unsupported)?;
        registry.ensure_room(&zone, &room);
        registry
            .rooms
            .get(&format!("{zone}:{room}"))
            .cloned()
            .unwrap_or_default()
    };

    let ordered_resolution = {
        let world = get_world_state()
            .read()
            .map_err(|_| TransferStatus::Unsupported)?;
        let ordered = world.get_room_object_order(&zone, &room);
        if ordered.is_empty() {
            None
        } else {
            let floor = world.get_room_objs(&zone, &room);
            let mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
            let mut remaining = query.order;
            let occurrences = |object: &Object| -> i64 {
                if object.getInt("투명상태") == 1 {
                    return 0;
                }
                if matches_item_name(object, &query.name) {
                    1
                } else {
                    python_sequence(object, "반응이름")
                        .iter()
                        .filter(|alias| alias.starts_with(&query.name))
                        .count() as i64
                }
            };
            let mut resolved = None;
            let mut saw_match = false;
            for object in ordered {
                let (count, selected_box) = match object {
                    crate::world::RoomObjectRef::InstalledBox(ordinal) => {
                        let Some(candidate) = boxes.get(ordinal).cloned() else {
                            continue;
                        };
                        let count = candidate
                            .lock()
                            .map_err(|_| TransferStatus::Unsupported)
                            .map(|candidate| occurrences(&candidate))?;
                        (count, Some(candidate))
                    }
                    crate::world::RoomObjectRef::Box(pointer) => {
                        let Some(candidate) = boxes
                            .iter()
                            .find(|candidate| Arc::as_ptr(candidate) as usize == pointer)
                            .cloned()
                        else {
                            continue;
                        };
                        let count = candidate
                            .lock()
                            .map_err(|_| TransferStatus::Unsupported)
                            .map(|candidate| occurrences(&candidate))?;
                        (count, Some(candidate))
                    }
                    crate::world::RoomObjectRef::FloorItem(pointer) => {
                        let Some(item) = floor
                            .iter()
                            .find(|item| Arc::as_ptr(item) as usize == pointer)
                        else {
                            continue;
                        };
                        let count = item
                            .lock()
                            .map_err(|_| TransferStatus::Unsupported)
                            .map(|item| occurrences(&item))?;
                        (count, None)
                    }
                    crate::world::RoomObjectRef::Mob(id) => {
                        let Some(mob) = mobs.iter().find(|mob| mob.instance_id == id) else {
                            continue;
                        };
                        let eligible = if query.name == "시체" {
                            !mob.alive
                        } else {
                            mob.alive
                        };
                        if !eligible {
                            continue;
                        }
                        let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                            return Err(TransferStatus::Unsupported);
                        };
                        let count = if mob.name == query.name || data.name == query.name {
                            1
                        } else {
                            data.reaction_names
                                .iter()
                                .filter(|alias| alias.starts_with(&query.name))
                                .count() as i64
                        };
                        (count, None)
                    }
                    crate::world::RoomObjectRef::Player(name) => {
                        (connected_player_occurrences(&name, &query.name), None)
                    }
                    crate::world::RoomObjectRef::SummonedUser(id) => {
                        let Some(user) = world.summoned_users().iter().find(|user| user.id == id)
                        else {
                            continue;
                        };
                        if user.body.get_int("투명상태") == 1 {
                            continue;
                        }
                        let aliases = super::reaction_names(&user.body.get_string("반응이름"));
                        let count = if user.body.get_name() == query.name
                            || aliases.iter().any(|alias| alias == &query.name)
                        {
                            1
                        } else {
                            aliases
                                .iter()
                                .filter(|alias| alias.starts_with(&query.name))
                                .count() as i64
                        };
                        (count, None)
                    }
                    crate::world::RoomObjectRef::Fixture(id) => {
                        let count = world
                            .get_fixture(id)
                            .map(|fixture| {
                                let (exact, prefixes) = fixture.match_counts(&query.name);
                                if exact {
                                    1
                                } else {
                                    prefixes as i64
                                }
                            })
                            .unwrap_or(0);
                        (count, None)
                    }
                };
                if count <= 0 {
                    continue;
                }
                saw_match = true;
                if remaining <= count {
                    resolved = Some(match selected_box {
                        Some(selected) => Ok(selected),
                        None => Err(TransferStatus::NoBox),
                    });
                    break;
                }
                remaining -= count;
            }
            if resolved.is_none() && saw_match {
                Some(Err(TransferStatus::NoBox))
            } else {
                resolved
            }
        }
    };

    if let Some(result) = ordered_resolution {
        let selected = result?;
        let supported = selected
            .lock()
            .map(|box_object| box_load_supported(&box_object))
            .map_err(|_| TransferStatus::Unsupported)?;
        return supported
            .then_some(selected)
            .ok_or(TransferStatus::Unsupported);
    }

    if !box_has_any_match(&boxes, &query) {
        return Err(TransferStatus::NoBox);
    }
    // Rust does not yet retain Python's single room.objs order across player,
    // mob, floor item and Box.  A matching non-Box makes first-object
    // selection unknowable, so reject before changing either container.
    if room_has_non_box_competitor(body, &query) {
        return Err(TransferStatus::Unsupported);
    }
    let selected = find_box_without_competitors(&boxes, &query).ok_or(TransferStatus::NoBox)?;
    let supported = selected
        .lock()
        .map(|box_object| box_load_supported(&box_object))
        .map_err(|_| TransferStatus::Unsupported)?;
    if !supported {
        return Err(TransferStatus::Unsupported);
    }
    Ok(selected)
}

fn box_capacity(box_object: &Object) -> Option<i64> {
    direct_integer(box_object, "보관수량")
}

fn box_is_public(box_object: &Object) -> bool {
    python_contains(box_object, "아이템속성", "공용보관함")
}

fn box_accepts_kind(box_object: &Object, kind: &str) -> bool {
    python_contains(box_object, "보관종류", kind)
}

fn item_arc_details(item: &Arc<Mutex<Object>>) -> Result<ItemDetails, TransferStatus> {
    item.lock()
        .map(|item| item_details(&item))
        .map_err(|_| TransferStatus::Unsupported)
}

fn item_arc_matches(item: &Arc<Mutex<Object>>, name: &str) -> Result<bool, TransferStatus> {
    item.lock()
        .map(|item| matches_item_name(&item, name))
        .map_err(|_| TransferStatus::Unsupported)
}

fn find_inventory_item(
    inventory: &[Arc<Mutex<Object>>],
    name: &str,
    order: i64,
    skip_in_use: bool,
    skip_hidden: bool,
) -> Result<Option<Arc<Mutex<Object>>>, TransferStatus> {
    let mut found = 0_i64;
    for item in inventory {
        let item_value = item.lock().map_err(|_| TransferStatus::Unsupported)?;
        if !matches_item_name(&item_value, name) {
            continue;
        }
        let details = item_details(&item_value);
        if (skip_in_use && details.in_use) || (skip_hidden && details.hidden) {
            continue;
        }
        found += 1;
        if found == order {
            return Ok(Some(item.clone()));
        }
    }
    Ok(None)
}

fn count_inventory_items(
    inventory: &[Arc<Mutex<Object>>],
    name: &str,
    skip_in_use: bool,
    skip_hidden: bool,
) -> Result<i64, TransferStatus> {
    let mut found = 0_i64;
    for item in inventory {
        let item = item.lock().map_err(|_| TransferStatus::Unsupported)?;
        if !matches_item_name(&item, name) {
            continue;
        }
        let details = item_details(&item);
        if (skip_in_use && details.in_use) || (skip_hidden && details.hidden) {
            continue;
        }
        found += 1;
    }
    Ok(found)
}

fn selected_count(words: &[&str]) -> i64 {
    words
        .get(2)
        .map(|value| parse_int_prefix(value))
        .unwrap_or(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PutBulkMode {
    All,
    OptionItem,
    Herb,
    OptionArmor,
    OptionWeapon,
}

impl PutBulkMode {
    fn from_word(value: &str) -> Option<Self> {
        match value {
            "모두" => Some(Self::All),
            "속성아이템" => Some(Self::OptionItem),
            "약초" => Some(Self::Herb),
            "속성방어구" => Some(Self::OptionArmor),
            "속성무기" => Some(Self::OptionWeapon),
            _ => None,
        }
    }

    fn matches(self, item: &ItemDetails) -> bool {
        match self {
            Self::All => true,
            Self::OptionItem => item.has_option,
            Self::Herb => item.purchase_name == "약초",
            Self::OptionArmor => item.has_option && item.kind == "방어구",
            Self::OptionWeapon => item.has_option && item.kind == "무기",
        }
    }
}

fn plan_put_bulk(
    body: &Body,
    box_object: &Object,
    mode: PutBulkMode,
) -> Result<(Vec<Arc<Mutex<Object>>>, Vec<TransferGroup>), TransferStatus> {
    let capacity = box_capacity(box_object).ok_or(TransferStatus::Unsupported)?;
    let public = box_is_public(box_object);
    let mut box_count = inventory_unit_count(box_object);
    let mut selected = Vec::new();
    let mut groups = Vec::new();

    for item in body.object.objs.clone() {
        // Python checks isFull before every filter in each bulk branch.
        if box_count >= capacity {
            return if selected.is_empty() {
                Err(TransferStatus::BoxFull)
            } else {
                Ok((selected, groups))
            };
        }
        let details = item_arc_details(&item)?;
        if !box_accepts_kind(box_object, &details.kind)
            || details.cannot_store
            || (public && details.public_blocked)
            || details.in_use
            || !mode.matches(&details)
        {
            continue;
        }
        increment_group(&mut groups, &details);
        selected.push(item);
        box_count += 1;
    }
    if selected.is_empty() {
        Err(TransferStatus::Nothing)
    } else {
        Ok((selected, groups))
    }
}

fn put_bulk(
    body: &mut Body,
    box_object: &mut Object,
    mode: PutBulkMode,
    oneitems: &mut dyn OneItemActions,
) -> TransferResult {
    let box_name = box_object.getName();
    let (selected, mut groups) = match plan_put_bulk(body, box_object, mode) {
        Ok(value) => value,
        Err(TransferStatus::Nothing) => (Vec::new(), Vec::new()),
        Err(status) => return TransferResult::for_box(status, box_name),
    };

    // Python's single-herb output has one stray `%s` placeholder and raises
    // after moving the item.  Treat that as a source bug: the intended output
    // is the same one-item form used by every other bulk selector.
    if !can_save_after_put(box_object, &selected) {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    }

    let location = format!("{} {}", body.get_name(), box_name);
    for item in selected {
        let details = match item_arc_details(&item) {
            Ok(details) => details,
            Err(status) => return TransferResult::for_box(status, box_object.getName()),
        };
        body.object.remove(&item);
        box_object.insert(item);
        if details.one_item {
            oneitems.keep(&details.index, &location);
        }
    }
    let Some(capacity) = box_capacity(box_object) else {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    };
    let public = box_is_public(box_object);
    let mut stack_keys = body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
    stack_keys.sort();
    for key in stack_keys {
        let have = body.object.inv_stack.get(&key).copied().unwrap_or(0);
        let Some(details) = stack_details(&key) else {
            continue;
        };
        if !mode.matches(&details)
            || !box_accepts_kind(box_object, &details.kind)
            || details.cannot_store
            || (public && details.public_blocked)
        {
            continue;
        }
        let available = capacity.saturating_sub(inventory_unit_count(box_object));
        if available <= 0 {
            break;
        }
        let moved = have.min(available);
        if super::inventory_compat::remove_pristine_count(&mut body.object, &key, moved)
            && super::inventory_compat::add_pristine_count(box_object, &key, moved)
        {
            increment_group_count(&mut groups, &details, moved);
        }
    }
    if groups.is_empty() {
        return TransferResult::for_box(TransferStatus::Nothing, box_name);
    }
    let _ = save_box(box_object);
    TransferResult {
        status: TransferStatus::Ok,
        box_name: box_object.getName(),
        money: 0,
        groups,
    }
}

fn put_money(body: &mut Body, box_object: &mut Object, requested: i64) -> TransferResult {
    let box_name = box_object.getName();
    let Some(capacity) = box_capacity(box_object) else {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    };
    let Some(max_capacity) = direct_integer(box_object, "보관최대수량") else {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    };
    if capacity == max_capacity {
        return TransferResult::for_box(TransferStatus::NotExpandable, box_name);
    }
    let amount = if requested <= 0 { 1 } else { requested };
    let player_money = body.get_int("은전");
    if player_money < amount {
        return TransferResult::for_box(TransferStatus::NotEnoughMoney, box_name);
    }
    let box_money = match box_object.get("은전") {
        Value::String(value) if value.is_empty() => 0,
        Value::Int(value) => value,
        Value::Float(value) if value.is_finite() && value.fract() == 0.0 => value as i64,
        _ => return TransferResult::for_box(TransferStatus::Unsupported, box_name),
    };
    let Some(requirement) = direct_integer(box_object, "보관증가은전") else {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    };
    if requirement == 0 {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    }

    let Some(mut saved_money) = box_money.checked_add(amount) else {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    };
    let available_growth = match max_capacity.checked_sub(capacity) {
        Some(value) => value,
        None => return TransferResult::for_box(TransferStatus::Unsupported, box_name),
    };
    let mut growth = saved_money.div_euclid(requirement);
    let mut consumed = amount;
    let mut new_capacity = capacity;
    if growth != 0 {
        if growth > available_growth {
            growth = available_growth;
        }
        let Some(cost) = growth.checked_mul(requirement) else {
            return TransferResult::for_box(TransferStatus::Unsupported, box_name);
        };
        let Some(value) = saved_money.checked_sub(cost) else {
            return TransferResult::for_box(TransferStatus::Unsupported, box_name);
        };
        saved_money = value;
        let Some(value) = capacity.checked_add(growth) else {
            return TransferResult::for_box(TransferStatus::Unsupported, box_name);
        };
        new_capacity = value;
        if new_capacity == max_capacity && saved_money != 0 {
            let Some(value) = amount.checked_sub(saved_money) else {
                return TransferResult::for_box(TransferStatus::Unsupported, box_name);
            };
            consumed = value;
            saved_money = 0;
        }
    }
    let Some(new_player_money) = player_money.checked_sub(consumed) else {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    };
    if !can_save_after_put(box_object, &[]) {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    }

    box_object.set("은전", saved_money);
    box_object.set("보관수량", new_capacity);
    body.set("은전", new_player_money);
    let _ = save_box(box_object);
    TransferResult {
        status: TransferStatus::Ok,
        box_name: box_object.getName(),
        money: consumed,
        groups: Vec::new(),
    }
}

fn put_selected(
    body: &mut Body,
    box_object: &mut Object,
    words: &[&str],
    oneitems: &mut dyn OneItemActions,
) -> TransferResult {
    let box_name = box_object.getName();
    let inventory = body.object.objs.clone();
    let raw_name = words[1];
    let mut parsed_name = raw_name.to_string();
    let mut parsed_order = 1_i64;
    let mut fallback_item = false;

    let mut initial = match find_inventory_item(&inventory, raw_name, 1, true, false) {
        Ok(value) => value,
        Err(status) => return TransferResult::for_box(status, box_name),
    };
    if initial.is_none() {
        let (name, order) = python_name_order(raw_name);
        parsed_name = name;
        parsed_order = order;
        initial = match find_inventory_item(&inventory, &parsed_name, parsed_order, true, false) {
            Ok(value) => value,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        if initial.is_none() {
            let object_count = match count_inventory_items(&inventory, &parsed_name, true, false) {
                Ok(count) => count,
                Err(status) => return TransferResult::for_box(status, box_name),
            };
            let remaining = parsed_order.max(1).saturating_sub(object_count);
            let Some((key, offset)) = super::inventory_compat::counted_item_at(
                &body.object.inv_stack,
                &parsed_name,
                remaining,
            ) else {
                return TransferResult::for_box(TransferStatus::ItemNotFound, box_name);
            };
            let have = body.object.inv_stack.get(&key).copied().unwrap_or(0);
            let Some(details) = stack_details(&key) else {
                return TransferResult::for_box(TransferStatus::Unsupported, box_name);
            };
            if !box_accepts_kind(box_object, &details.kind)
                || details.cannot_store
                || (box_is_public(box_object) && details.public_blocked)
            {
                return TransferResult::for_box(TransferStatus::CannotStore, box_name);
            }
            let Some(capacity) = box_capacity(box_object) else {
                return TransferResult::for_box(TransferStatus::Unsupported, box_name);
            };
            let requested = if parsed_order > 1 {
                1
            } else {
                selected_count(words).max(1)
            };
            let available = capacity.saturating_sub(inventory_unit_count(box_object));
            if available <= 0 {
                return TransferResult::for_box(TransferStatus::BoxFull, box_name);
            }
            let moved = requested
                .min(have.saturating_sub(offset - 1))
                .min(available);
            if !super::inventory_compat::remove_pristine_count(&mut body.object, &key, moved)
                || !super::inventory_compat::add_pristine_count(box_object, &key, moved)
            {
                return TransferResult::for_box(TransferStatus::Unsupported, box_name);
            }
            let _ = save_box(box_object);
            let mut groups = Vec::new();
            increment_group_count(&mut groups, &details, moved);
            return TransferResult {
                status: TransferStatus::Ok,
                box_name: box_object.getName(),
                money: 0,
                groups,
            };
        }
        fallback_item = true;
    }
    let initial_details = match item_arc_details(initial.as_ref().unwrap()) {
        Ok(details) => details,
        Err(status) => return TransferResult::for_box(status, box_name),
    };
    if !box_accepts_kind(box_object, &initial_details.kind)
        || initial_details.cannot_store
        || (box_is_public(box_object) && initial_details.public_blocked)
    {
        return TransferResult::for_box(TransferStatus::CannotStore, box_name);
    }

    let mut count = selected_count(words);
    if fallback_item {
        count = 1;
    }
    let Some(capacity) = box_capacity(box_object) else {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    };
    let mut box_count = inventory_unit_count(box_object);
    let match_name = if fallback_item {
        parsed_name.as_str()
    } else {
        raw_name
    };
    let mut occurrence = 1_i64;
    let mut selected = Vec::new();
    let mut groups = Vec::new();

    for item in inventory {
        let matches = match item_arc_matches(&item, match_name) {
            Ok(value) => value,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        if !matches {
            continue;
        }
        if fallback_item && parsed_order != occurrence {
            occurrence += 1;
            continue;
        }
        if box_count >= capacity {
            if selected.is_empty() {
                return TransferResult::for_box(TransferStatus::BoxFull, box_name);
            }
            break;
        }
        let details = match item_arc_details(&item) {
            Ok(details) => details,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        // Python's selected loop intentionally omits the public-box four-attr
        // test after validating only the initially found item.
        if !box_accepts_kind(box_object, &details.kind) || details.cannot_store || details.in_use {
            continue;
        }
        increment_group(&mut groups, &details);
        selected.push(item);
        box_count += 1;
        if i64::try_from(selected.len()).ok() == Some(count) {
            break;
        }
    }
    let mut stack_moves = Vec::<(String, i64)>::new();
    let mut remaining = count.saturating_sub(selected.len() as i64);
    if !fallback_item && remaining > 0 {
        for key in super::inventory_compat::counted_item_keys(&body.object.inv_stack, match_name) {
            if box_count >= capacity {
                break;
            }
            let Some(details) = stack_details(&key) else {
                continue;
            };
            if !box_accepts_kind(box_object, &details.kind) || details.cannot_store {
                continue;
            }
            let have = body.object.inv_stack.get(&key).copied().unwrap_or(0);
            let available = capacity.saturating_sub(box_count);
            let moved = have.min(available).min(remaining);
            if moved <= 0 {
                continue;
            }
            increment_group_count(&mut groups, &details, moved);
            stack_moves.push((key, moved));
            box_count += moved;
            remaining -= moved;
            if remaining == 0 {
                break;
            }
        }
    }
    if selected.is_empty() && stack_moves.is_empty() {
        return TransferResult::for_box(TransferStatus::Nothing, box_name);
    }
    if !can_save_after_put(box_object, &selected) {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    }

    let location = format!("{} {}", box_object.getString("주인"), box_name);
    for item in selected {
        let details = match item_arc_details(&item) {
            Ok(details) => details,
            Err(status) => return TransferResult::for_box(status, box_object.getName()),
        };
        body.object.remove(&item);
        box_object.insert(item);
        if details.one_item {
            oneitems.keep(&details.index, &location);
        }
    }
    for (key, count) in stack_moves {
        if !super::inventory_compat::remove_pristine_count(&mut body.object, &key, count)
            || !super::inventory_compat::add_pristine_count(box_object, &key, count)
        {
            return TransferResult::for_box(TransferStatus::Unsupported, box_name);
        }
    }
    let _ = save_box(box_object);
    TransferResult {
        status: TransferStatus::Ok,
        box_name: box_object.getName(),
        money: 0,
        groups,
    }
}

fn execute_put(body: &mut Body, line: &str, oneitems: &mut dyn OneItemActions) -> TransferResult {
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.len() < 2 {
        return TransferResult::status(TransferStatus::Unsupported);
    }
    let box_arc = match resolve_box(body, words[0]) {
        Ok(value) => value,
        Err(status) => return TransferResult::status(status),
    };
    if super::inventory_compat::materialize_stacks_for_save(body).is_err() {
        return TransferResult::status(TransferStatus::Unsupported);
    }
    let Ok(mut box_object) = box_arc.lock() else {
        return TransferResult::status(TransferStatus::Unsupported);
    };
    if words[1] == "은전" {
        let amount = words
            .get(2)
            .map(|value| parse_int_prefix(value))
            .unwrap_or(1);
        return put_money(body, &mut box_object, amount);
    }
    if let Some(mode) = PutBulkMode::from_word(words[1]) {
        return put_bulk(body, &mut box_object, mode, oneitems);
    }
    put_selected(body, &mut box_object, &words, oneitems)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TakeBulkMode {
    All,
    Herb,
    OptionWeapon,
    OptionArmor,
}

impl TakeBulkMode {
    fn from_word(value: &str) -> Option<Self> {
        match value {
            "모두" => Some(Self::All),
            "약초" => Some(Self::Herb),
            "속성무기" => Some(Self::OptionWeapon),
            "속성방어구" => Some(Self::OptionArmor),
            _ => None,
        }
    }

    fn matches(self, item: &ItemDetails) -> bool {
        match self {
            Self::All => true,
            Self::Herb => item.purchase_name == "약초",
            Self::OptionWeapon => item.kind == "무기" && item.has_option,
            Self::OptionArmor => item.kind == "방어구" && item.has_option,
        }
    }
}

fn max_inventory_count() -> Option<i64> {
    std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|content| serde_json::from_str::<JsonValue>(&content).ok())
        .and_then(|root| root.get("메인설정").cloned())
        .and_then(|config| config.get("사용자아이템갯수").cloned())
        .and_then(|value| match value {
            JsonValue::Number(number) => number.as_i64(),
            JsonValue::String(value) => Some(parse_int_prefix(&value)),
            _ => None,
        })
}

fn current_inventory_load(body: &Body) -> Result<(i64, i64), TransferStatus> {
    let mut weight = 0_i64;
    let mut count = 0_i64;
    for item in &body.object.objs {
        let item = item.lock().map_err(|_| TransferStatus::Unsupported)?;
        if python_contains(&item, "아이템속성", "출력안함") {
            continue;
        }
        let item_weight = match item.get("무게") {
            Value::Int(value) => value,
            Value::Float(value) => value as i64,
            Value::String(value) => parse_int_prefix(&value),
        };
        weight = weight
            .checked_add(item_weight)
            .ok_or(TransferStatus::Unsupported)?;
        count = count.checked_add(1).ok_or(TransferStatus::Unsupported)?;
    }
    for (key, quantity) in &body.object.inv_stack {
        if *quantity <= 0 {
            continue;
        }
        let details = stack_details(key).ok_or(TransferStatus::Unsupported)?;
        if details.hidden {
            continue;
        }
        weight = weight
            .checked_add(
                details
                    .weight
                    .ok_or(TransferStatus::Unsupported)?
                    .saturating_mul(*quantity),
            )
            .ok_or(TransferStatus::Unsupported)?;
        count = count
            .checked_add(*quantity)
            .ok_or(TransferStatus::Unsupported)?;
    }
    Ok((weight, count))
}

fn can_take_candidate(
    body: &Body,
    item: &ItemDetails,
    weight: i64,
    count: i64,
    max_count: i64,
) -> Result<(), TransferStatus> {
    let item_weight = item.weight.ok_or(TransferStatus::Unsupported)?;
    let strength_limit = body
        .get_str()
        .checked_mul(10)
        .ok_or(TransferStatus::Unsupported)?;
    let next_weight = weight
        .checked_add(item_weight)
        .ok_or(TransferStatus::Unsupported)?;
    if next_weight > strength_limit {
        return Err(TransferStatus::TooHeavy);
    }
    // Python uses `>` rather than `>=`: an inventory exactly at the limit
    // accepts one additional visible item.
    if count > max_count {
        return Err(TransferStatus::ItemLimit);
    }
    Ok(())
}

fn plan_take_bulk(
    body: &Body,
    box_object: &Object,
    mode: TakeBulkMode,
    max_count: i64,
) -> Result<(Vec<Arc<Mutex<Object>>>, Vec<TransferGroup>), TransferStatus> {
    let (mut weight, mut count) = current_inventory_load(body)?;
    let mut selected = Vec::new();
    let mut groups = Vec::new();
    for item in box_object.objs.clone() {
        let details = item_arc_details(&item)?;
        if !mode.matches(&details) {
            continue;
        }
        if let Err(status) = can_take_candidate(body, &details, weight, count, max_count) {
            return if selected.is_empty() {
                Err(status)
            } else {
                Ok((selected, groups))
            };
        }
        increment_group(&mut groups, &details);
        selected.push(item);
        // getItemWeight/getItemCount ignore 출력안함 after insertion.
        if !details.hidden {
            weight = weight
                .checked_add(details.weight.ok_or(TransferStatus::Unsupported)?)
                .ok_or(TransferStatus::Unsupported)?;
            count = count.checked_add(1).ok_or(TransferStatus::Unsupported)?;
        }
    }
    if selected.is_empty() {
        Err(TransferStatus::Nothing)
    } else {
        Ok((selected, groups))
    }
}

fn apply_take(
    body: &mut Body,
    box_object: &mut Object,
    selected: Vec<Arc<Mutex<Object>>>,
    groups: Vec<TransferGroup>,
    oneitems: &mut dyn OneItemActions,
) -> TransferResult {
    let owner = body.get_name();
    if selected.iter().any(|item| {
        item.lock()
            .ok()
            .is_some_and(|item| !super::inventory_compat::can_accept_object(&body.object, &item))
    }) {
        return TransferResult::for_box(TransferStatus::Unsupported, box_object.getName());
    }
    if !can_save_after_take(box_object, &selected) {
        return TransferResult::for_box(TransferStatus::Unsupported, box_object.getName());
    }
    for item in selected {
        let details = match item_arc_details(&item) {
            Ok(details) => details,
            Err(status) => return TransferResult::for_box(status, box_object.getName()),
        };
        box_object.remove(&item);
        let accepted = super::inventory_compat::store_acquired_object(&mut body.object, item, true);
        debug_assert!(accepted);
        if details.one_item {
            oneitems.have(&details.index, &owner);
        }
    }
    let _ = save_box(box_object);
    TransferResult {
        status: TransferStatus::Ok,
        box_name: box_object.getName(),
        money: 0,
        groups,
    }
}

fn take_bulk(
    body: &mut Body,
    box_object: &mut Object,
    mode: TakeBulkMode,
    max_count: i64,
    oneitems: &mut dyn OneItemActions,
) -> TransferResult {
    let box_name = box_object.getName();
    let (selected, mut groups) = match plan_take_bulk(body, box_object, mode, max_count) {
        Ok(value) => value,
        Err(TransferStatus::Nothing) => (Vec::new(), Vec::new()),
        Err(status) => return TransferResult::for_box(status, box_name),
    };
    let owner = body.get_name();
    if selected.iter().any(|item| {
        item.lock()
            .ok()
            .is_some_and(|item| !super::inventory_compat::can_accept_object(&body.object, &item))
    }) {
        return TransferResult::for_box(TransferStatus::Unsupported, box_name);
    }
    for item in selected {
        let details = match item_arc_details(&item) {
            Ok(details) => details,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        box_object.remove(&item);
        let accepted = super::inventory_compat::store_acquired_object(&mut body.object, item, true);
        debug_assert!(accepted);
        if details.one_item {
            oneitems.have(&details.index, &owner);
        }
    }
    let (mut weight, mut count) = match current_inventory_load(body) {
        Ok(value) => value,
        Err(status) => return TransferResult::for_box(status, box_name),
    };
    let mut stack_keys = box_object.inv_stack.keys().cloned().collect::<Vec<_>>();
    stack_keys.sort();
    for key in stack_keys {
        let have = box_object.inv_stack.get(&key).copied().unwrap_or(0);
        let Some(details) = stack_details(&key) else {
            continue;
        };
        if !mode.matches(&details) {
            continue;
        }
        let mut moved = 0_i64;
        while moved < have {
            if can_take_candidate(body, &details, weight, count, max_count).is_err() {
                break;
            }
            moved += 1;
            if !details.hidden {
                weight = weight.saturating_add(details.weight.unwrap_or(0));
                count = count.saturating_add(1);
            }
        }
        if moved > 0
            && super::inventory_compat::remove_pristine_count(box_object, &key, moved)
            && super::inventory_compat::add_pristine_count(&mut body.object, &key, moved)
        {
            increment_group_count(&mut groups, &details, moved);
        }
    }
    if groups.is_empty() {
        return TransferResult::for_box(TransferStatus::Nothing, box_name);
    }
    let _ = save_box(box_object);
    TransferResult {
        status: TransferStatus::Ok,
        box_name: box_object.getName(),
        money: 0,
        groups,
    }
}

fn take_single_preflight(
    body: &Body,
    item: &Arc<Mutex<Object>>,
    max_count: i64,
) -> Result<ItemDetails, TransferStatus> {
    let details = item_arc_details(item)?;
    let (weight, count) = current_inventory_load(body)?;
    can_take_candidate(body, &details, weight, count, max_count)?;
    Ok(details)
}

fn take_selected(
    body: &mut Body,
    box_object: &mut Object,
    words: &[&str],
    max_count: i64,
    oneitems: &mut dyn OneItemActions,
) -> TransferResult {
    let box_name = box_object.getName();
    let raw_name = words[1];
    let mut count = selected_count(words);
    let mut item: Option<Arc<Mutex<Object>>> = None;
    let mut numeric_stack: Option<String> = None;
    let mut order = -1_i64;

    if python_is_digit_string(raw_name) {
        let index = parse_int_prefix(raw_name);
        let length = inventory_unit_count(box_object);
        let object_length = box_object.objs.len();
        let length = match i64::try_from(length) {
            Ok(value) => value,
            Err(_) => return TransferResult::for_box(TransferStatus::Unsupported, box_name),
        };
        if length
            .checked_sub(index)
            .is_some_and(|difference| difference >= 0)
        {
            if length == 0 && index == 0 {
                // Python evaluates box.objs[-1] and raises IndexError.
                return TransferResult::for_box(TransferStatus::Unsupported, box_name);
            }
            let unit_index = if index == 0 {
                usize::try_from(length - 1).unwrap_or(usize::MAX)
            } else {
                usize::try_from(index - 1).unwrap_or(usize::MAX)
            };
            if unit_index < object_length {
                item = box_object.objs.get(unit_index).cloned();
                order = 0;
            } else {
                let mut offset = unit_index.saturating_sub(object_length);
                let mut stack_keys = box_object.inv_stack.keys().cloned().collect::<Vec<_>>();
                stack_keys.sort();
                for key in stack_keys {
                    let quantity = box_object.inv_stack.get(&key).copied().unwrap_or(0).max(0);
                    if offset < quantity as usize {
                        numeric_stack = Some(key);
                        break;
                    }
                    offset = offset.saturating_sub(quantity as usize);
                }
            }
        }
    }

    if let Some(key) = numeric_stack {
        let Some(details) = stack_details(&key) else {
            return TransferResult::for_box(TransferStatus::Unsupported, box_name);
        };
        let (weight, current_count) = match current_inventory_load(body) {
            Ok(value) => value,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        if let Err(status) = can_take_candidate(body, &details, weight, current_count, max_count) {
            return TransferResult::for_box(status, box_name);
        }
        if !super::inventory_compat::remove_pristine_count(box_object, &key, 1)
            || !super::inventory_compat::add_pristine_count(&mut body.object, &key, 1)
        {
            return TransferResult::for_box(TransferStatus::Unsupported, box_name);
        }
        let _ = save_box(box_object);
        let mut groups = Vec::new();
        increment_group_count(&mut groups, &details, 1);
        return TransferResult {
            status: TransferStatus::Ok,
            box_name: box_object.getName(),
            money: 0,
            groups,
        };
    }

    if item.is_none() {
        item = match find_inventory_item(&box_object.objs, raw_name, 1, false, true) {
            Ok(value) => value,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
    }

    if item.is_none() {
        let (name, parsed_order) = python_name_order(raw_name);
        order = parsed_order;
        item = match find_inventory_item(&box_object.objs, &name, order, true, false) {
            Ok(value) => value,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        if item.is_none() {
            let object_count = match count_inventory_items(&box_object.objs, &name, true, false) {
                Ok(count) => count,
                Err(status) => return TransferResult::for_box(status, box_name),
            };
            let remaining = order.max(1).saturating_sub(object_count);
            let Some((key, offset)) =
                super::inventory_compat::counted_item_at(&box_object.inv_stack, &name, remaining)
            else {
                return TransferResult::for_box(TransferStatus::ItemNotFound, box_name);
            };
            let have = box_object.inv_stack.get(&key).copied().unwrap_or(0);
            let Some(details) = stack_details(&key) else {
                return TransferResult::for_box(TransferStatus::Unsupported, box_name);
            };
            let requested = if order > 1 { 1 } else { count.max(1) };
            let (mut weight, mut current_count) = match current_inventory_load(body) {
                Ok(value) => value,
                Err(status) => return TransferResult::for_box(status, box_name),
            };
            let mut moved = 0_i64;
            let wanted = requested.min(have.saturating_sub(offset - 1));
            while moved < wanted {
                if let Err(status) =
                    can_take_candidate(body, &details, weight, current_count, max_count)
                {
                    if moved == 0 {
                        return TransferResult::for_box(status, box_name);
                    }
                    break;
                }
                moved += 1;
                if !details.hidden {
                    weight = weight.saturating_add(details.weight.unwrap_or(0));
                    current_count = current_count.saturating_add(1);
                }
            }
            if !super::inventory_compat::remove_pristine_count(box_object, &key, moved)
                || !super::inventory_compat::add_pristine_count(&mut body.object, &key, moved)
            {
                return TransferResult::for_box(TransferStatus::Unsupported, box_name);
            }
            let _ = save_box(box_object);
            let mut groups = Vec::new();
            increment_group_count(&mut groups, &details, moved);
            return TransferResult {
                status: TransferStatus::Ok,
                box_name: box_object.getName(),
                money: 0,
                groups,
            };
        }
        count = 1;
    }

    if order != -1 {
        let item = item.unwrap();
        let details = match take_single_preflight(body, &item, max_count) {
            Ok(details) => details,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        let mut groups = Vec::new();
        increment_group(&mut groups, &details);
        return apply_take(body, box_object, vec![item], groups, oneitems);
    }

    let (mut weight, mut current_count) = match current_inventory_load(body) {
        Ok(value) => value,
        Err(status) => return TransferResult::for_box(status, box_name),
    };
    let mut selected = Vec::new();
    let mut groups = Vec::new();
    for candidate in box_object.objs.clone() {
        let matches = match item_arc_matches(&candidate, raw_name) {
            Ok(value) => value,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        if !matches {
            continue;
        }
        let details = match item_arc_details(&candidate) {
            Ok(details) => details,
            Err(status) => return TransferResult::for_box(status, box_name),
        };
        if let Err(status) = can_take_candidate(body, &details, weight, current_count, max_count) {
            if selected.is_empty() {
                return TransferResult::for_box(status, box_name);
            }
            break;
        }
        increment_group(&mut groups, &details);
        selected.push(candidate);
        if !details.hidden {
            weight = match weight.checked_add(details.weight.unwrap_or(0)) {
                Some(value) => value,
                None => return TransferResult::for_box(TransferStatus::Unsupported, box_name),
            };
            current_count = match current_count.checked_add(1) {
                Some(value) => value,
                None => return TransferResult::for_box(TransferStatus::Unsupported, box_name),
            };
        }
        if i64::try_from(selected.len()).ok() == Some(count) {
            break;
        }
    }
    let mut stack_moves = Vec::<(String, i64)>::new();
    let mut remaining = count.saturating_sub(selected.len() as i64);
    if remaining > 0 {
        for key in super::inventory_compat::counted_item_keys(&box_object.inv_stack, raw_name) {
            let Some(details) = stack_details(&key) else {
                continue;
            };
            let have = box_object.inv_stack.get(&key).copied().unwrap_or(0);
            let mut moved = 0_i64;
            while moved < have.min(remaining) {
                if let Err(status) =
                    can_take_candidate(body, &details, weight, current_count, max_count)
                {
                    if selected.is_empty() && stack_moves.is_empty() {
                        return TransferResult::for_box(status, box_name);
                    }
                    break;
                }
                moved += 1;
                if !details.hidden {
                    weight = weight.saturating_add(details.weight.unwrap_or(0));
                    current_count = current_count.saturating_add(1);
                }
            }
            if moved > 0 {
                increment_group_count(&mut groups, &details, moved);
                stack_moves.push((key, moved));
                remaining -= moved;
            }
            if remaining == 0 {
                break;
            }
        }
    }
    if selected.is_empty() && stack_moves.is_empty() {
        return TransferResult::for_box(TransferStatus::Nothing, box_name);
    }
    let mut result = apply_take(body, box_object, selected, groups, oneitems);
    if result.status != TransferStatus::Ok {
        return result;
    }
    for (key, count) in stack_moves {
        if !super::inventory_compat::remove_pristine_count(box_object, &key, count)
            || !super::inventory_compat::add_pristine_count(&mut body.object, &key, count)
        {
            result.status = TransferStatus::Unsupported;
            return result;
        }
    }
    let _ = save_box(box_object);
    result
}

fn execute_take(body: &mut Body, line: &str, oneitems: &mut dyn OneItemActions) -> TransferResult {
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.len() < 2 {
        return TransferResult::status(TransferStatus::Unsupported);
    }
    let box_arc = match resolve_box(body, words[0]) {
        Ok(value) => value,
        Err(status) => return TransferResult::status(status),
    };
    if super::inventory_compat::materialize_stacks_for_save(body).is_err() {
        return TransferResult::status(TransferStatus::Unsupported);
    }
    let Some(max_count) = max_inventory_count() else {
        return TransferResult::status(TransferStatus::Unsupported);
    };
    let Ok(mut box_object) = box_arc.lock() else {
        return TransferResult::status(TransferStatus::Unsupported);
    };
    if let Some(mode) = TakeBulkMode::from_word(words[1]) {
        return take_bulk(body, &mut box_object, mode, max_count, oneitems);
    }
    take_selected(body, &mut box_object, &words, max_count, oneitems)
}

/// Python `cmds/정렬.py`: sort a persistent box's children and charge 100000
/// 은전.  The sort is intentionally stable and uses the same secondary name
/// key as Python's `(getOp(item), item['이름'])` expression.
fn execute_sort(body: &mut Body, line: &str) -> &'static str {
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.len() < 2 {
        return "usage";
    }
    let box_arc = match resolve_box(body, words[0]) {
        Ok(value) => value,
        Err(_) => return "missing",
    };
    let key = words[1];
    const KEYS: [&str; 11] = [
        "힘",
        "민첩성",
        "맷집",
        "명중",
        "회피",
        "필살",
        "운",
        "방어력",
        "체력",
        "내공",
        "이름",
    ];
    if !KEYS.contains(&key) {
        return "invalid";
    }
    if body.get_int("은전") < 100_000 {
        return "money";
    }
    let Ok(mut box_object) = box_arc.lock() else {
        return "missing";
    };
    box_object.objs.sort_by(|left, right| {
        let left = left.lock().ok();
        let right = right.lock().ok();
        let left_name = left.as_ref().map(|item| item.getName()).unwrap_or_default();
        let right_name = right
            .as_ref()
            .map(|item| item.getName())
            .unwrap_or_default();
        if key == "이름" {
            return left_name.cmp(&right_name);
        }
        let left_value = left
            .as_ref()
            .and_then(|item| item.get_option())
            .and_then(|options| options.get(key).copied())
            .unwrap_or(0);
        let right_value = right
            .as_ref()
            .and_then(|item| item.get_option())
            .and_then(|options| options.get(key).copied())
            .unwrap_or(0);
        left_value
            .cmp(&right_value)
            .then(left_name.cmp(&right_name))
    });
    body.set("은전", body.get_int("은전") - 100_000);
    "ok"
}

pub(super) fn register_box_command_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    let view_body_ptr = body_ptr;
    engine.register_fn(
        "find_view_box",
        move |_ob: &mut rhai::Map, line: &str| -> Dynamic {
            let body = unsafe { &*view_body_ptr };
            let (name, order) = python_name_order(line);
            // Python first selects from the complete inventory, then falls
            // back to Room.findObjName's integrated object order.  Filtering
            // for boxes before selection lets a room box jump ahead of a
            // same-name ordinary inventory item.
            let selected =
                body.object
                    .findObjInven(&name, order.max(1) as usize)
                    .or_else(|| {
                        if let Some(selected) = match super::select_python_room_object(body, line) {
                            Some(crate::world::RoomObjectRef::InstalledBox(ordinal)) => {
                                let position = get_world_state().read().ok().and_then(|world| {
                                    world.get_player_position(&body.get_name()).cloned()
                                })?;
                                installed_boxes_for_room(&position.zone, &position.room)
                                    .and_then(|boxes| boxes.get(ordinal).cloned())
                            }
                            _ => None,
                        } {
                            return Some(selected);
                        }
                        // Older loaded fixtures may have no integrated room-order
                        // record.  Preserve their installed-box registration
                        // order only as a fallback.
                        let position = get_world_state().read().ok().and_then(|world| {
                            world.get_player_position(&body.get_name()).cloned()
                        })?;
                        let boxes = installed_boxes_for_room(&position.zone, &position.room)?;
                        let mut matched = 0_i64;
                        boxes.into_iter().find(|candidate| {
                            let Ok(value) = candidate.lock() else {
                                return false;
                            };
                            let aliases = value.getString("반응이름");
                            let is_match = value.getName() == name
                                || aliases
                                    .split_whitespace()
                                    .any(|alias| alias == name || alias.starts_with(name.as_str()));
                            if is_match {
                                matched += 1;
                            }
                            is_match && matched == order
                        })
                    });
            if let Some(candidate) = selected {
                let Ok(value) = candidate.lock() else {
                    let mut result = rhai::Map::new();
                    result.insert("found".into(), Dynamic::from(false));
                    return Dynamic::from(result);
                };
                if !is_box(&value) {
                    let mut result = rhai::Map::new();
                    result.insert("found".into(), Dynamic::from(false));
                    return Dynamic::from(result);
                }
                let mut result = rhai::Map::new();
                result.insert("found".into(), Dynamic::from(true));
                for key in ["이름", "주인", "인덱스"] {
                    result.insert(key.into(), Dynamic::from(value.getString(key)));
                }
                for key in ["보관수량", "보관최대수량", "보관증가은전", "은전"] {
                    result.insert(key.into(), Dynamic::from(value.getInt(key)));
                }
                let mut items = value
                    .objs
                    .iter()
                    .filter_map(|item| {
                        let item = item.lock().ok()?;
                        let mut data = rhai::Map::new();
                        data.insert("name".into(), Dynamic::from(item.getName()));
                        data.insert("option".into(), Dynamic::from(item.get_option_str()));
                        data.insert(
                            "oneitem".into(),
                            Dynamic::from(item.checkAttr("아이템속성", "단일아이템")),
                        );
                        Some(Dynamic::from(data))
                    })
                    .collect::<rhai::Array>();
                let mut stack_keys = value.inv_stack.keys().cloned().collect::<Vec<_>>();
                stack_keys.sort();
                for key in stack_keys {
                    let count = value.inv_stack.get(&key).copied().unwrap_or(0).max(0);
                    let Some((item, _)) = super::object_from_item_json(&key) else {
                        continue;
                    };
                    let Ok(item) = item.lock() else { continue };
                    for _ in 0..count {
                        let mut data = rhai::Map::new();
                        data.insert("name".into(), Dynamic::from(item.getName()));
                        data.insert("option".into(), Dynamic::from(item.get_option_str()));
                        data.insert("oneitem".into(), Dynamic::from(false));
                        items.push(Dynamic::from(data));
                    }
                }
                result.insert("items".into(), Dynamic::from(items));
                return Dynamic::from(result);
            }
            let mut missing = rhai::Map::new();
            missing.insert("found".into(), Dynamic::from(false));
            Dynamic::from(missing)
        },
    );

    engine.register_fn("get_box_room_context", || -> Dynamic {
        Dynamic::from(
            PRECOMPUTED_BOX_CONTEXT
                .with(|slot| slot.borrow().clone())
                .unwrap_or_else(|| {
                    let mut context = rhai::Map::new();
                    context.insert("self_id".into(), Dynamic::from(String::new()));
                    context.insert("players".into(), Dynamic::from(rhai::Array::new()));
                    context
                }),
        )
    });

    let delivery_body_ptr = body_ptr;
    engine.register_fn(
        "box_send_raw",
        move |_ob: &mut rhai::Map, connection_id: &str, raw_text: &str| -> bool {
            if raw_text.is_empty() || !box_context_knows(connection_id) {
                return false;
            }
            let body = unsafe { &mut *delivery_body_ptr };
            let current = body
                .temp()
                .get(BOX_DELIVERY_REQUESTS)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mut deliveries: Vec<BoxDelivery> =
                serde_json::from_str(&current).unwrap_or_default();
            deliveries.push(BoxDelivery {
                connection_id: connection_id.to_string(),
                raw_text: raw_text.to_string(),
            });
            let Ok(serialized) = serde_json::to_string(&deliveries) else {
                return false;
            };
            body.temp_mut()
                .insert(BOX_DELIVERY_REQUESTS.to_string(), Value::String(serialized));
            true
        },
    );

    let put_body_ptr = body_ptr;
    engine.register_fn(
        "box_put_python",
        move |_ob: &mut rhai::Map, line: &str| -> Dynamic {
            let body = unsafe { &mut *put_body_ptr };
            let mut oneitems = PersistentOneItemActions;
            execute_put(body, line, &mut oneitems).into_dynamic()
        },
    );

    let take_body_ptr = body_ptr;
    engine.register_fn(
        "box_take_python",
        move |_ob: &mut rhai::Map, line: &str| -> Dynamic {
            let body = unsafe { &mut *take_body_ptr };
            let mut oneitems = PersistentOneItemActions;
            execute_take(body, line, &mut oneitems).into_dynamic()
        },
    );

    let sort_body_ptr = body_ptr;
    engine.register_fn(
        "box_sort_python",
        move |_ob: &mut rhai::Map, line: &str| -> String {
            let body = unsafe { &mut *sort_body_ptr };
            execute_sort(body, line).to_string()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandResult;
    use crate::world::PlayerPosition;
    use rhai::Scope;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_ID: AtomicUsize = AtomicUsize::new(0);

    #[derive(Default)]
    struct RecordingOneItems {
        kept: Vec<(String, String)>,
        held: Vec<(String, String)>,
    }

    impl OneItemActions for RecordingOneItems {
        fn keep(&mut self, index: &str, location: &str) {
            self.kept.push((index.to_string(), location.to_string()));
        }

        fn have(&mut self, index: &str, owner: &str) {
            self.held.push((index.to_string(), owner.to_string()));
        }
    }

    fn test_directory(label: &str) -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "muc_box_parity_{label}_{}_{}",
            std::process::id(),
            id
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_item(
        index: &str,
        name: &str,
        kind: &str,
        reactions: &[&str],
        attrs: &[&str],
        weight: i64,
    ) -> Arc<Mutex<Object>> {
        let mut item = Object::new();
        item.set("인덱스", index);
        item.set("이름", name);
        item.set("종류", kind);
        item.set("무게", weight);
        item.set("inUse", 0_i64);
        super::super::inventory_compat::set_item_json_field(
            &mut item,
            "반응이름",
            &serde_json::json!(reactions),
        );
        super::super::inventory_compat::set_item_json_field(
            &mut item,
            "아이템속성",
            &serde_json::json!(attrs),
        );
        Arc::new(Mutex::new(item))
    }

    fn make_box(
        directory: &Path,
        name: &str,
        owner: &str,
        capacity: i64,
        max_capacity: i64,
        kinds: &[&str],
        attrs: &[&str],
    ) -> Object {
        let path = directory.join(format!("{owner}_{name}.json"));
        let mut object = Object::new();
        mark_box(&mut object, &format!("{owner}_{name}"), &path);
        object.set("이름", name);
        object.set("주인", owner);
        object.set("보관수량", capacity);
        object.set("보관최대수량", max_capacity);
        object.set("보관증가은전", 100_i64);
        object.set("은전", 0_i64);
        super::super::inventory_compat::set_item_json_field(
            &mut object,
            "보관종류",
            &serde_json::json!(kinds),
        );
        super::super::inventory_compat::set_item_json_field(
            &mut object,
            "아이템속성",
            &serde_json::json!(attrs),
        );
        super::super::inventory_compat::set_item_json_field(
            &mut object,
            "반응이름",
            &serde_json::json!([name]),
        );
        object
    }

    fn object_names(objects: &[Arc<Mutex<Object>>]) -> Vec<String> {
        objects
            .iter()
            .map(|item| item.lock().unwrap().getName())
            .collect()
    }

    #[test]
    fn authoritative_sources_lock_python_transfer_quirks() {
        let put = include_str!("../../cmds/넣어.py");
        let take = include_str!("../../cmds/꺼내.py");
        let box_source = include_str!("../../objs/box.py");
        assert!(put.contains("if c == count:"));
        assert!(!put.contains("if count <= 0:"));
        assert!(put.contains("box['주인'] + ' %s' % box['이름']"));
        assert!(put.contains("ob['이름'] + ' %s' % box['이름']"));
        assert!(put.contains("\x1b[37m%s 보관합니다.' % (box.getNameA(), post)"));
        assert!(take.contains("ob.getItemCount() > getInt"));
        assert!(take.contains("item = box.objs[idx - 1]"));
        assert!(box_source.contains("with open(self.path + '.json'"));
    }

    #[test]
    fn room_string_installation_is_iterated_as_characters_and_array_order_is_preserved() {
        assert_eq!(
            python_installation_names(Some(&serde_json::json!("보관함"))),
            ["보", "관", "함"]
        );
        assert_eq!(
            python_installation_names(Some(&serde_json::json!(["무기함", "방어함"]))),
            ["무기함", "방어함"]
        );
    }

    #[test]
    fn missing_fixed_room_box_uses_item_defaults_instead_of_empty_zero_capacity_box() {
        let directory = test_directory("installed_defaults");
        let weapon = load_installed_box("_무기보관함", "무기보관함", &directory);
        assert_eq!(weapon.getName(), "무기보관함");
        assert_eq!(weapon.getInt("보관수량"), 10);
        assert_eq!(weapon.getString("보관종류"), "무기");

        let armor = load_installed_box("_방어구보관함", "방어구보관함", &directory);
        assert_eq!(armor.getName(), "방어구보관함");
        assert_eq!(armor.getInt("보관수량"), 10);
        assert_eq!(armor.getString("보관종류"), "방어구");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn sort_checks_box_before_key_and_money_sorts_memory_without_python_file_write() {
        let directory = test_directory("sort");
        let suffix = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let player = format!("정렬사용자-{suffix}");
        let zone = format!("정렬존-{suffix}");
        let room = "1";
        let box_name = format!("정렬함-{suffix}");
        let mut box_object = make_box(&directory, &box_name, &player, 10, 10, &["무기"], &[]);
        let high_b = make_item("높은나", "나검", "무기", &["나검"], &[], 1);
        high_b.lock().unwrap().set("옵션", "힘 7");
        let zero = make_item("영", "가검", "무기", &["가검"], &[], 1);
        let high_a = make_item("높은가", "가검2", "무기", &["가검2"], &[], 1);
        high_a.lock().unwrap().set("옵션", "힘 7");
        box_object.objs.extend([high_b, zero, high_a]);
        let path = box_path(&box_object).unwrap();
        std::fs::write(&path, "원본파일유지").unwrap();
        let box_arc = Arc::new(Mutex::new(box_object));
        register_installed_box(&zone, room, box_arc.clone());
        get_world_state()
            .write()
            .unwrap()
            .set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
        let mut body = Body::new();
        body.set("이름", player.as_str());

        let storage = crate::script::ScriptStorage::default();
        assert_eq!(
            storage
                .execute("정렬", &mut body, "", None, None, None)
                .unwrap()
                .0,
            vec!["☞ 사용법: [보관함] [특성치] 정렬"]
        );
        assert_eq!(execute_sort(&mut body, "없는함 잘못"), "missing");
        assert_eq!(
            storage
                .execute(
                    "정렬",
                    &mut body,
                    &format!("{box_name} 잘못"),
                    None,
                    None,
                    None,
                )
                .unwrap()
                .0,
            vec!["☞ 힘|민첩성|맷집|명중|회피|필살|운|방어력|체력|내공|이름 만 가능합니다."]
        );
        let poor = storage
            .execute(
                "정렬",
                &mut body,
                &format!("{box_name} 힘"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(poor.0, vec!["☞ 은전이 부족해요."]);
        body.set("은전", 100_000_i64);
        let sorted = storage
            .execute(
                "정렬",
                &mut body,
                &format!("  {box_name}   힘  "),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(sorted.0, vec!["☞ 정렬되었습니다."]);
        assert_eq!(body.get_int("은전"), 0);
        assert_eq!(
            object_names(&box_arc.lock().unwrap().objs),
            ["가검", "가검2", "나검"]
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "원본파일유지");

        body.set("은전", 100_000_i64);
        let by_name = storage
            .execute(
                "정렬",
                &mut body,
                &format!("{box_name} 이름"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(by_name.0, vec!["☞ 정렬되었습니다."]);
        assert_eq!(
            object_names(&box_arc.lock().unwrap().objs),
            ["가검", "가검2", "나검"]
        );

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn box_resolution_uses_integrated_floor_item_and_box_order() {
        let directory = test_directory("integrated_order");
        let suffix = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let player = format!("보관순서사용자-{suffix}");
        let zone = format!("보관순서존-{suffix}");
        let room = "1";
        let box_object = Arc::new(Mutex::new(make_box(
            &directory,
            "충돌보관함",
            &player,
            5,
            5,
            &["기타"],
            &[],
        )));
        register_installed_box(&zone, room, box_object.clone());
        let mut floor = Object::new();
        floor.set("이름", "충돌물건");
        floor.set("반응이름", "충돌보관함");
        let floor = Arc::new(Mutex::new(floor));
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
            world.get_room_objs_mut(&zone, room).push(floor.clone());
            world.record_floor_item(&zone, room, &floor);
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        assert_eq!(
            resolve_box(&body, "충돌보관함").unwrap_err(),
            TransferStatus::NoBox
        );

        get_world_state()
            .write()
            .unwrap()
            .record_box(&zone, room, &box_object);
        assert!(Arc::ptr_eq(
            &resolve_box(&body, "충돌보관함").unwrap(),
            &box_object
        ));

        let summoned_id = {
            let mut summoned = Body::new();
            summoned.set("이름", "보관소환경쟁자");
            summoned.set("반응이름", "충돌보관함");
            get_world_state()
                .write()
                .unwrap()
                .add_summoned_user(summoned, PlayerPosition::new(zone.clone(), room.into()))
        };
        assert_eq!(
            resolve_box(&body, "충돌보관함").unwrap_err(),
            TransferStatus::NoBox
        );
        get_world_state()
            .write()
            .unwrap()
            .remove_summoned_user_by_id(summoned_id);

        let mut connected = Body::new();
        connected.set("이름", "보관연결경쟁자");
        connected.set("반응이름", "충돌보관함");
        let snapshot = build_box_observer_snapshot("경쟁자토큰".into(), &connected, 1);
        set_precomputed_box_context("행위자토큰".into(), vec![snapshot]);
        get_world_state().write().unwrap().set_player_position(
            "보관연결경쟁자",
            PlayerPosition::new(zone.clone(), room.into()),
        );
        assert_eq!(
            resolve_box(&body, "충돌보관함").unwrap_err(),
            TransferStatus::NoBox
        );
        clear_precomputed_box_context();

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position("보관연결경쟁자");
        world.remove_player_position(&player);
        world.get_room_objs_mut(&zone, room).clear();
        drop(world);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn take_empty_box_all_keeps_python_bulk_space_message() {
        let directory = test_directory("take_nothing_text");
        let suffix = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let player = format!("빈꺼내사용자-{suffix}");
        let zone = format!("빈꺼내존-{suffix}");
        let room = "1";
        let box_name = format!("빈보관함-{suffix}");
        let box_object = make_box(&directory, &box_name, &player, 10, 10, &["무기"], &[]);
        register_installed_box(zone.as_str(), room, Arc::new(Mutex::new(box_object)));
        get_world_state()
            .write()
            .unwrap()
            .set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
        let mut body = Body::new();
        body.set("이름", player.as_str());

        let output = crate::script::ScriptStorage::default()
            .execute(
                "꺼내",
                &mut body,
                &format!("{box_name} 모두"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(output.0, vec!["☞ 더 이상 꺼낼 물건이 없어요. ^^"]);

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn put_and_take_rhai_commands_match_python_text_and_move_the_real_item() {
        use crate::script::ScriptStorage;

        let directory = test_directory("put_take_command");
        let suffix = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let player = format!("보관명령사용자-{suffix}");
        let zone = format!("보관명령존-{suffix}");
        let room = "1";
        let box_name = format!("시험보관함-{suffix}");
        let box_object = make_box(&directory, &box_name, &player, 10, 10, &["무기"], &[]);
        let box_arc = Arc::new(Mutex::new(box_object));
        register_installed_box(&zone, room, box_arc.clone());
        get_world_state()
            .write()
            .unwrap()
            .set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));

        let item = make_item("보관시험검", "청명검", "무기", &["검", "청명"], &[], 2);
        item.lock().unwrap().set("안시", "\x1b[1;35m");
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("힘", 100_i64);
        body.object.objs.push(item.clone());
        let storage = ScriptStorage::default();

        let put = storage
            .execute(
                "넣어",
                &mut body,
                &format!("  {box_name}   검  "),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            put.0,
            vec![format!(
                "당신이 \x1b[36m{box_name}\x1b[37m에 \x1b[36m\x1b[1;35m청명검\x1b[0;37m을\x1b[37m 보관합니다."
            )]
        );
        assert!(body.object.objs.is_empty());
        assert_eq!(box_arc.lock().unwrap().objs.len(), 1);
        assert!(Arc::ptr_eq(&box_arc.lock().unwrap().objs[0], &item));

        let take = storage
            .execute(
                "꺼내",
                &mut body,
                &format!(" {box_name} 청명 "),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            take.0,
            vec![format!(
                "당신이 \x1b[36m{box_name}\x1b[37m에서 \x1b[36m\x1b[1;35m청명검\x1b[0;37m을\x1b[37m 꺼냅니다."
            )]
        );
        assert!(box_arc.lock().unwrap().objs.is_empty());
        assert_eq!(body.object.objs.len(), 1);
        assert!(Arc::ptr_eq(&body.object.objs[0], &item));

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player);
        let _ = std::fs::remove_file(format!("data/user/{player}.json"));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn box_load_reverses_json_items_and_preserves_raw_field_shapes() {
        let directory = test_directory("load");
        let path = directory.join("주인_함.json");
        let root = serde_json::json!({
            "상자정보": {
                "이름": "함",
                "반응이름": "함별칭",
                "보관종류": ["무기", "방어구"]
            },
            "아이템": [
                {"인덱스": "1", "이름": "첫째", "반응이름": "123", "옵션": "힘 1"},
                {"인덱스": "2", "이름": "둘째", "반응이름": ["둘", "두번째"], "옵션": ["힘 2"]}
            ]
        });
        std::fs::write(&path, serde_json::to_string(&root).unwrap()).unwrap();
        let loaded = load_box_with("주인_함", &path, |index| {
            Some(make_item(index, "템플릿", "무기", &["템플릿"], &[], 1))
        });
        assert_eq!(object_names(&loaded.objs), ["둘째", "첫째"]);
        assert!(!has_array_shape(&loaded, "반응이름"));
        assert!(has_array_shape(&loaded, "보관종류"));

        let first_json_item = loaded.objs[1].lock().unwrap();
        assert_eq!(
            box_item_record(&first_json_item).unwrap()["반응이름"],
            serde_json::json!("123")
        );
        assert_eq!(
            box_item_record(&first_json_item).unwrap()["옵션"],
            serde_json::json!("힘 1")
        );
        let second_json_item = loaded.objs[0].lock().unwrap();
        assert_eq!(
            box_item_record(&second_json_item).unwrap()["반응이름"],
            serde_json::json!(["둘", "두번째"])
        );
        assert_eq!(
            box_item_record(&second_json_item).unwrap()["옵션"],
            serde_json::json!(["힘 2"])
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn malformed_box_item_array_is_marked_unsupported_instead_of_silently_skipped() {
        let directory = test_directory("malformed_load");
        let path = directory.join("주인_함.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({
                "상자정보": {"이름": "함", "반응이름": ["함"]},
                "아이템": [{"인덱스": "1"}, "잘못된레코드"]
            }))
            .unwrap(),
        )
        .unwrap();
        let loaded = load_box_with("주인_함", &path, |index| {
            Some(make_item(index, "검", "무기", &["검"], &[], 1))
        });
        assert_eq!(loaded.getName(), "함");
        assert!(!box_load_supported(&loaded));
        assert!(loaded.objs.is_empty());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn box_save_repairs_python_double_suffix_and_preserves_vec_order() {
        let directory = test_directory("save");
        let source_path = directory.join("주인_함.json");
        let mut box_object = make_box(&directory, "함", "주인", 10, 20, &["무기"], &[]);
        let first = make_item("1", "첫째", "무기", &["첫"], &[], 1);
        let second = make_item("2", "둘째", "무기", &["둘"], &[], 1);
        first.lock().unwrap().set("시간", 12.5_f64);
        box_object.objs = vec![first, second];

        assert!(save_box(&box_object));
        let saved: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(&source_path).unwrap()).unwrap();
        assert_eq!(saved["아이템"][0]["변경"]["이름"], "첫째");
        assert_eq!(saved["아이템"][1]["변경"]["이름"], "둘째");
        assert_eq!(saved["아이템"][0]["변경"]["시간"], serde_json::json!(12.5));

        let malformed = Arc::new(Mutex::new(Object::new()));
        box_object.objs.push(malformed);
        let before = std::fs::read_to_string(&source_path).unwrap();
        assert!(!save_box(&box_object));
        assert_eq!(std::fs::read_to_string(&source_path).unwrap(), before);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn money_expansion_refunds_excess_at_maximum() {
        let directory = test_directory("money");
        let mut box_object = make_box(&directory, "함", "주인", 9, 10, &["무기"], &[]);
        box_object.set("은전", 50_i64);
        let mut body = Body::new();
        body.set("이름", "난이");
        body.set("은전", 1000_i64);

        let result = put_money(&mut body, &mut box_object, 1000);
        assert_eq!(result.status, TransferStatus::Ok);
        assert_eq!(result.money, 50);
        assert_eq!(body.get_int("은전"), 950);
        assert_eq!(box_object.getInt("보관수량"), 10);
        assert_eq!(box_object.getInt("은전"), 0);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn money_put_matches_python_default_prefix_parse_and_guard_order() {
        let directory = test_directory("money_edges");
        let mut body = Body::new();
        body.set("이름", "은전보관자");
        body.set("은전", 10_i64);

        let mut fixed = make_box(&directory, "고정함", "주인", 10, 10, &["무기"], &[]);
        let fixed_result = put_money(&mut body, &mut fixed, 999);
        assert_eq!(fixed_result.status, TransferStatus::NotExpandable);
        assert_eq!(body.get_int("은전"), 10);

        let mut box_object = make_box(&directory, "함", "주인", 1, 5, &["무기"], &[]);
        let insufficient = put_money(&mut body, &mut box_object, 11);
        assert_eq!(insufficient.status, TransferStatus::NotEnoughMoney);
        assert_eq!(body.get_int("은전"), 10);
        let default_one = put_money(&mut body, &mut box_object, parse_int_prefix("0"));
        assert_eq!(default_one.status, TransferStatus::Ok);
        assert_eq!(default_one.money, 1);
        assert_eq!(body.get_int("은전"), 9);
        assert_eq!(box_object.getInt("은전"), 1);
        let prefixed = put_money(&mut body, &mut box_object, parse_int_prefix("8개"));
        assert_eq!(prefixed.status, TransferStatus::Ok);
        assert_eq!(prefixed.money, 8);
        assert_eq!(body.get_int("은전"), 1);
        assert_eq!(box_object.getInt("은전"), 9);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn put_bulk_prepends_each_identity_and_uses_actor_for_oneitem_location() {
        let directory = test_directory("put_bulk");
        let mut box_object = make_box(&directory, "함", "주인", 3, 10, &["무기"], &[]);
        box_object
            .objs
            .push(make_item("old", "기존", "무기", &["기존"], &[], 1));
        let first = make_item("1", "검", "무기", &["검"], &["단일아이템"], 1);
        let blocked = make_item("x", "보관금지", "무기", &["금지"], &["보관못함"], 1);
        let second = make_item("2", "검", "무기", &["검"], &["단일아이템"], 1);
        let untouched = make_item("3", "나머지", "무기", &["나머지"], &[], 1);
        let mut body = Body::new();
        body.set("이름", "보관자");
        body.object.objs = vec![first, blocked, second, untouched];
        let mut oneitems = RecordingOneItems::default();

        let result = put_bulk(&mut body, &mut box_object, PutBulkMode::All, &mut oneitems);
        assert_eq!(result.status, TransferStatus::Ok);
        assert_eq!(result.groups[0].count, 2);
        assert_eq!(object_names(&box_object.objs), ["검", "검", "기존"]);
        assert_eq!(object_names(&body.object.objs), ["보관금지", "나머지"]);
        assert_eq!(
            oneitems.kept,
            [
                ("1".to_string(), "보관자 함".to_string()),
                ("2".to_string(), "보관자 함".to_string())
            ]
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn selected_zero_moves_every_match_and_preserves_public_recheck_bug() {
        let directory = test_directory("put_selected");
        let mut box_object = make_box(
            &directory,
            "함",
            "상자주인",
            10,
            10,
            &["무기"],
            &["공용보관함"],
        );
        let safe = make_item("1", "검", "무기", &["검"], &["단일아이템"], 1);
        let public_blocked = make_item("2", "검", "무기", &["검"], &["줄수없음", "단일아이템"], 1);
        let mut body = Body::new();
        body.set("이름", "행위자");
        body.object.objs = vec![safe, public_blocked];
        let mut oneitems = RecordingOneItems::default();

        let result = put_selected(
            &mut body,
            &mut box_object,
            &["함", "검", "0"],
            &mut oneitems,
        );
        assert_eq!(result.status, TransferStatus::Ok);
        assert!(body.object.objs.is_empty());
        assert_eq!(object_names(&box_object.objs), ["검", "검"]);
        assert_eq!(
            oneitems.kept,
            [
                ("1".to_string(), "상자주인 함".to_string()),
                ("2".to_string(), "상자주인 함".to_string())
            ]
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn single_herb_group_repairs_python_format_exception_and_completes_transfer() {
        let directory = test_directory("herb_bug");
        let mut box_object = make_box(&directory, "함", "주인", 10, 10, &["먹는것"], &[]);
        let herb = make_item("1", "산삼", "먹는것", &["산삼"], &[], 1);
        herb.lock().unwrap().set("구매이름", "약초");
        let mut body = Body::new();
        body.set("이름", "심마니");
        body.object.objs.push(herb.clone());
        let mut oneitems = RecordingOneItems::default();

        let result = put_bulk(&mut body, &mut box_object, PutBulkMode::Herb, &mut oneitems);
        assert_eq!(result.status, TransferStatus::Ok);
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].name, "산삼");
        assert_eq!(result.groups[0].count, 1);
        assert!(body.object.objs.is_empty());
        assert_eq!(object_names(&box_object.objs), ["산삼"]);
        assert!(oneitems.kept.is_empty());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn take_at_exact_item_limit_allows_one_more_then_stops() {
        let directory = test_directory("take_limit");
        let mut box_object = make_box(&directory, "함", "주인", 10, 10, &["무기"], &[]);
        box_object.objs = vec![
            make_item("1", "첫째", "무기", &["첫"], &[], 1),
            make_item("2", "둘째", "무기", &["둘"], &[], 1),
        ];
        let mut body = Body::new();
        body.set("이름", "꺼내는이");
        body.set("힘", 100_i64);
        body.object
            .objs
            .push(make_item("old", "기존", "무기", &["기존"], &[], 1));
        let mut oneitems = RecordingOneItems::default();

        let result = take_bulk(
            &mut body,
            &mut box_object,
            TakeBulkMode::All,
            1,
            &mut oneitems,
        );
        assert_eq!(result.status, TransferStatus::Ok);
        assert_eq!(object_names(&body.object.objs), ["첫째", "기존"]);
        assert_eq!(object_names(&box_object.objs), ["둘째"]);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn take_checks_weight_before_count_and_keeps_python_partial_success() {
        let directory = test_directory("take_guards");
        let mut box_object = make_box(&directory, "함", "주인", 10, 10, &["무기"], &[]);
        box_object.objs = vec![make_item("1", "후보", "무기", &["후보"], &[], 1)];
        let mut body = Body::new();
        body.set("이름", "한계시험");
        body.set("힘", 1_i64);
        body.object.objs = vec![
            make_item("a", "기존1", "무기", &["기존1"], &[], 5),
            make_item("b", "기존2", "무기", &["기존2"], &[], 5),
        ];
        let mut oneitems = RecordingOneItems::default();
        let both = take_bulk(
            &mut body,
            &mut box_object,
            TakeBulkMode::All,
            1,
            &mut oneitems,
        );
        assert_eq!(both.status, TransferStatus::TooHeavy);
        assert_eq!(object_names(&box_object.objs), ["후보"]);

        body.set("힘", 100_i64);
        let count_only = take_bulk(
            &mut body,
            &mut box_object,
            TakeBulkMode::All,
            1,
            &mut oneitems,
        );
        assert_eq!(count_only.status, TransferStatus::ItemLimit);
        assert_eq!(object_names(&box_object.objs), ["후보"]);

        body.object.objs.clear();
        body.set("힘", 1_i64);
        box_object.objs = vec![
            make_item("1", "첫째", "무기", &["첫째"], &[], 6),
            make_item("2", "둘째", "무기", &["둘째"], &[], 6),
        ];
        let partial = take_bulk(
            &mut body,
            &mut box_object,
            TakeBulkMode::All,
            300,
            &mut oneitems,
        );
        assert_eq!(partial.status, TransferStatus::Ok);
        assert_eq!(partial.groups.len(), 1);
        assert_eq!(partial.groups[0].name, "첫째");
        assert_eq!(object_names(&body.object.objs), ["첫째"]);
        assert_eq!(object_names(&box_object.objs), ["둘째"]);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn numeric_zero_take_selects_last_box_item() {
        let directory = test_directory("take_zero");
        let mut box_object = make_box(&directory, "함", "주인", 10, 10, &["무기"], &[]);
        box_object.objs = vec![
            make_item("1", "첫째", "무기", &["첫"], &[], 1),
            make_item("2", "마지막", "무기", &["마지막"], &[], 1),
        ];
        let mut body = Body::new();
        body.set("이름", "꺼내는이");
        body.set("힘", 100_i64);
        let mut oneitems = RecordingOneItems::default();
        let result = take_selected(&mut body, &mut box_object, &["함", "0"], 300, &mut oneitems);
        assert_eq!(result.status, TransferStatus::Ok);
        assert_eq!(object_names(&body.object.objs), ["마지막"]);
        assert_eq!(object_names(&box_object.objs), ["첫째"]);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn numeric_take_selects_a_compact_stack_unit_without_expanding_the_box() {
        let directory = test_directory("take_compact_numeric");
        let mut box_object = make_box(&directory, "함", "주인", 10, 10, &["무기"], &[]);
        box_object.inv_stack.insert("289".to_string(), 2);
        let mut body = Body::new();
        body.set("이름", "수량꺼내는이");
        body.set("힘", 100_i64);
        let mut oneitems = RecordingOneItems::default();

        let result = take_selected(&mut body, &mut box_object, &["함", "1"], 300, &mut oneitems);

        assert_eq!(result.status, TransferStatus::Ok);
        assert_eq!(body.object.inv_stack.get("289"), Some(&1));
        assert_eq!(box_object.inv_stack.get("289"), Some(&1));
        assert!(body.object.objs.is_empty());
        assert!(box_object.objs.is_empty());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn numbered_box_transfer_counts_changed_objects_before_pristine_quantity() {
        let directory = test_directory("mixed_numbered_transfer");
        let mut box_object = make_box(&directory, "함", "주인", 10, 10, &["무기"], &[]);
        let (changed, _) = super::super::object_from_item_json("289").unwrap();
        changed.lock().unwrap().set("판매가격", 9_i64);

        let mut body = Body::new();
        body.set("이름", "혼합보관자");
        body.set("힘", 100_i64);
        body.object.objs.push(changed.clone());
        body.object.inv_stack.insert("289".to_string(), 1);
        let mut oneitems = RecordingOneItems::default();

        let put = put_selected(&mut body, &mut box_object, &["함", "2철퇴"], &mut oneitems);
        assert_eq!(put.status, TransferStatus::Ok);
        assert_eq!(body.object.inv_stack.get("289"), None);
        assert!(Arc::ptr_eq(&body.object.objs[0], &changed));
        assert_eq!(box_object.inv_stack.get("289"), Some(&1));
        assert!(box_object.objs.is_empty());

        box_object.objs.push(changed.clone());
        body.object.objs.clear();
        let take = take_selected(
            &mut body,
            &mut box_object,
            &["함", "2철퇴"],
            300,
            &mut oneitems,
        );
        assert_eq!(take.status, TransferStatus::Ok);
        assert_eq!(body.object.inv_stack.get("289"), Some(&1));
        assert!(Arc::ptr_eq(&box_object.objs[0], &changed));
        assert_eq!(box_object.inv_stack.get("289"), None);

        body.object.objs = vec![changed.clone()];
        body.object.inv_stack.insert("289".to_string(), 2);
        box_object.objs.clear();
        box_object.inv_stack.clear();
        let put_bulk_same_name = put_selected(
            &mut body,
            &mut box_object,
            &["함", "철퇴", "3"],
            &mut oneitems,
        );
        assert_eq!(put_bulk_same_name.status, TransferStatus::Ok);
        assert!(body.object.objs.is_empty());
        assert_eq!(body.object.inv_stack.get("289"), None);
        assert!(Arc::ptr_eq(&box_object.objs[0], &changed));
        assert_eq!(box_object.inv_stack.get("289"), Some(&2));

        let take_bulk_same_name = take_selected(
            &mut body,
            &mut box_object,
            &["함", "철퇴", "3"],
            300,
            &mut oneitems,
        );
        assert_eq!(take_bulk_same_name.status, TransferStatus::Ok);
        assert!(Arc::ptr_eq(&body.object.objs[0], &changed));
        assert_eq!(body.object.inv_stack.get("289"), Some(&2));
        assert!(box_object.objs.is_empty());
        assert_eq!(box_object.inv_stack.get("289"), None);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn malformed_final_box_child_blocks_before_transfer() {
        let directory = test_directory("preflight");
        let mut box_object = make_box(&directory, "함", "주인", 10, 10, &["무기"], &[]);
        box_object.objs.push(Arc::new(Mutex::new(Object::new())));
        let item = make_item("1", "검", "무기", &["검"], &[], 1);
        let mut body = Body::new();
        body.set("이름", "이동자");
        body.object.objs.push(item);
        let mut oneitems = RecordingOneItems::default();

        let result = put_selected(&mut body, &mut box_object, &["함", "검"], &mut oneitems);
        assert_eq!(result.status, TransferStatus::Unsupported);
        assert_eq!(body.object.objs.len(), 1);
        assert_eq!(box_object.objs.len(), 1);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn rhai_put_output_and_observer_delivery_use_same_room_index() {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let zone = format!("상자테스트존{id}");
        let room = "1".to_string();
        let actor_name = format!("갑{id}");
        let observer_name = format!("을{id}");
        let elsewhere_name = format!("병{id}");
        let directory = test_directory("rhai");
        let mut box_object = make_box(&directory, "시험함", "주인", 10, 10, &["무기"], &[]);
        box_object.set("반응이름", "시험함");
        let room_key = format!("{zone}:{room}");
        {
            let mut boxes = registry().lock().unwrap();
            boxes.loaded_rooms.insert(room_key.clone());
            boxes
                .rooms
                .insert(room_key.clone(), vec![Arc::new(Mutex::new(box_object))]);
        }
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(&actor_name, PlayerPosition::new(zone.clone(), room.clone()));
            world.set_player_position(
                &observer_name,
                PlayerPosition::new(zone.clone(), room.clone()),
            );
            world.set_player_position(
                &elsewhere_name,
                PlayerPosition::new(zone.clone(), "2".to_string()),
            );
        }

        let mut body = Body::new();
        body.set("이름", actor_name.as_str());
        body.object
            .objs
            .push(make_item("1", "검", "무기", &["검"], &[], 1));
        let mut observer = Body::new();
        observer.set("이름", observer_name.as_str());
        observer.set("체력", 31_i64);
        observer.set("최고체력", 45_i64);
        observer.set("내공", 7_i64);
        observer.set("최고내공", 9_i64);
        set_precomputed_box_context(
            "actor-id".to_string(),
            vec![
                build_box_observer_snapshot("actor-id".to_string(), &body, 1),
                build_box_observer_snapshot("observer-id".to_string(), &observer, 1),
            ],
        );
        let outputs = Arc::new(Mutex::new(Vec::new()));
        let special = Arc::new(Mutex::new(None::<CommandResult>));
        let sends = Arc::new(Mutex::new(Vec::new()));
        let engine = super::super::create_engine_with_body_and_output(
            &mut body,
            outputs.clone(),
            None,
            None,
            special,
            sends.clone(),
            None,
            Some("넣어"),
            None,
        );
        let mut scope = Scope::new();
        scope.push("ob", Dynamic::from(super::super::build_ob_from_body(&body)));
        scope.push("cmdline", "시험함 검".to_string());
        let source = include_str!("../../cmds/넣어.rhai");
        engine
            .run_with_scope(&mut scope, &format!("{source}\nmain(ob, cmdline)"))
            .unwrap();

        assert_eq!(
            outputs.lock().unwrap().as_slice(),
            ["당신이 \x1b[36m시험함\x1b[37m에 \x1b[36m\x1b[0;36m검\x1b[37m을\x1b[37m 보관합니다."
                .to_string()]
        );
        assert!(sends.lock().unwrap().is_empty());
        let deliveries = take_box_deliveries(&mut body);
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].connection_id, "observer-id");
        assert_eq!(
            deliveries[0].raw_text,
            format!(
                "\r\n\x1b[1m{}\x1b[0;37m{} \x1b[36m시험함\x1b[37m에 \x1b[36m\x1b[0;36m검\x1b[37m을\x1b[37m 보관합니다.\r\n\r\n\x1b[0;37;40m[ 31/45, 7/9 ] ",
                actor_name,
                crate::hangul::han_iga(&actor_name)
            )
        );
        assert_ne!(deliveries[0].connection_id, elsewhere_name);
        clear_precomputed_box_context();

        {
            let mut world = get_world_state().write().unwrap();
            world.remove_player_position(&actor_name);
            world.remove_player_position(&observer_name);
            world.remove_player_position(&elsewhere_name);
        }
        {
            let mut boxes = registry().lock().unwrap();
            boxes.rooms.remove(&room_key);
            boxes.loaded_rooms.remove(&room_key);
        }
        let _ = std::fs::remove_dir_all(directory);
    }
}
