use crate::object::{Object, Value};
use crate::player::Body;
use crate::world::get_world_state;
use rhai::{Dynamic, Engine};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const ROOM_ITEM_LIMIT: usize = 50;
const DROP_TIME_KEY: &str = "timeofdrop";

#[derive(Clone, Debug, PartialEq, Eq)]
struct DropGroup {
    name: String,
    particle_source: String,
    ansi: String,
    count: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropStatus {
    Ok,
    NotFound,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DropResult {
    status: DropStatus,
    all: bool,
    removed: i64,
    dropped: Vec<DropGroup>,
    failed: Vec<DropGroup>,
}

impl DropResult {
    fn empty(status: DropStatus, all: bool) -> Self {
        Self {
            status,
            all,
            removed: 0,
            dropped: Vec::new(),
            failed: Vec::new(),
        }
    }

    fn into_dynamic(self) -> Dynamic {
        let mut result = rhai::Map::new();
        let status = match self.status {
            DropStatus::Ok => "ok",
            DropStatus::NotFound => "not_found",
            DropStatus::Blocked => "blocked",
        };
        result.insert("status".into(), Dynamic::from(status));
        result.insert("all".into(), Dynamic::from(self.all));
        result.insert("removed".into(), Dynamic::from(self.removed));
        result.insert(
            "dropped".into(),
            Dynamic::from(groups_to_array(self.dropped)),
        );
        result.insert("failed".into(), Dynamic::from(groups_to_array(self.failed)));
        Dynamic::from(result)
    }

    fn into_death_dynamic(self, insured: i64) -> Dynamic {
        let mut result = self.into_dynamic().cast::<rhai::Map>();
        result.insert("insured".into(), Dynamic::from(insured));
        Dynamic::from(result)
    }
}

fn groups_to_array(groups: Vec<DropGroup>) -> rhai::Array {
    groups
        .into_iter()
        .map(|group| {
            let mut map = rhai::Map::new();
            map.insert("name".into(), Dynamic::from(group.name));
            map.insert(
                "particle_source".into(),
                Dynamic::from(group.particle_source),
            );
            map.insert("ansi".into(), Dynamic::from(group.ansi));
            map.insert("count".into(), Dynamic::from(group.count));
            Dynamic::from(map)
        })
        .collect()
}

trait OneItemActions {
    fn dropped(&mut self, index: &str, owner: &str);
    fn destroyed(&mut self, index: &str);
}

struct PersistentOneItemActions;

impl OneItemActions for PersistentOneItemActions {
    fn dropped(&mut self, index: &str, owner: &str) {
        let _ = crate::oneitem::oneitem_drop(index, owner);
    }

    fn destroyed(&mut self, index: &str) {
        let _ = crate::oneitem::oneitem_destroy(index);
    }
}

#[derive(Clone, Debug)]
struct ItemDetails {
    name: String,
    reactions: Vec<String>,
    particle_source: String,
    ansi: String,
    index: String,
    one_item: bool,
    in_use: bool,
    hidden: bool,
    cannot_drop: bool,
    cannot_give: bool,
    insurance_excluded: bool,
}

fn reaction_names(raw: &str) -> Vec<String> {
    raw.split(|character: char| character == '|' || character.is_whitespace())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn item_details(item: &Object) -> ItemDetails {
    let name = item.getName();
    let reactions = reaction_names(&item.getString("반응이름"));
    let particle_source = if crate::hangul::is_han(&name) {
        name.clone()
    } else {
        reactions.first().cloned().unwrap_or_else(|| name.clone())
    };
    ItemDetails {
        name,
        reactions,
        particle_source,
        ansi: item.getString("안시"),
        index: item.getString("인덱스"),
        one_item: item.checkAttr("아이템속성", "단일아이템"),
        in_use: item.getBool("inUse"),
        hidden: item.checkAttr("아이템속성", "출력안함"),
        cannot_drop: item.checkAttr("아이템속성", "버리지못함"),
        cannot_give: item.checkAttr("아이템속성", "줄수없음"),
        insurance_excluded: item.checkAttr("아이템속성", "보험적용안됨"),
    }
}

/// `inv_stack` is Rust's compressed representation of repeated Python Item
/// objects. Rebuild the same item metadata from the authoritative item JSON;
/// the stored count itself carries no duplicate metadata.
fn stack_item_details(key: &str) -> Option<ItemDetails> {
    if !super::is_stackable(key) {
        return None;
    }
    let (item, _) = super::object_from_item_json(key)?;
    let item = item.lock().ok()?;
    Some(item_details(&item))
}

fn matches_name(item: &ItemDetails, name: &str) -> bool {
    item.name == name || item.reactions.iter().any(|reaction| reaction == name)
}

fn increment_group(groups: &mut Vec<DropGroup>, details: &ItemDetails) {
    if let Some(group) = groups.iter_mut().find(|group| group.name == details.name) {
        group.count += 1;
        return;
    }
    groups.push(DropGroup {
        name: details.name.clone(),
        particle_source: details.particle_source.clone(),
        ansi: details.ansi.clone(),
        count: 1,
    });
}

/// `버려.py` selected-branch indentation bug: the first destroyed item creates
/// count 1, but later destroyed items with the same name never increment it.
fn insert_selected_failure_bug(groups: &mut Vec<DropGroup>, details: &ItemDetails) {
    if groups.iter().any(|group| group.name == details.name) {
        return;
    }
    groups.push(DropGroup {
        name: details.name.clone(),
        particle_source: details.particle_source.clone(),
        ansi: details.ansi.clone(),
        count: 1,
    });
}

fn mark_dropped(item: &Arc<Mutex<Object>>, now: f64) {
    if let Ok(mut item) = item.lock() {
        item.temp
            .insert(DROP_TIME_KEY.to_string(), Value::Float(now));
    }
}

fn move_or_destroy(
    item: Arc<Mutex<Object>>,
    details: &ItemDetails,
    floor: &mut Vec<Arc<Mutex<Object>>>,
    room_item_count: &mut usize,
    owner: &str,
    now: f64,
    oneitems: &mut dyn OneItemActions,
) -> bool {
    if *room_item_count < ROOM_ITEM_LIMIT {
        // Python Room.insert() inserts at index zero.
        mark_dropped(&item, now);
        floor.insert(0, item);
        *room_item_count += 1;
        if details.one_item {
            oneitems.dropped(&details.index, owner);
        }
        true
    } else {
        if details.one_item {
            oneitems.destroyed(&details.index);
        }
        false
    }
}

fn move_stack_unit_or_destroy(
    key: &str,
    details: &ItemDetails,
    floor: &mut Vec<Arc<Mutex<Object>>>,
    floor_stack: &mut HashMap<String, i64>,
    room_item_count: &mut usize,
    owner: &str,
    now: f64,
    oneitems: &mut dyn OneItemActions,
) -> bool {
    if *room_item_count < ROOM_ITEM_LIMIT {
        if let Some((template, _)) = super::object_from_item_json(key) {
            if let Ok(template) = template.lock() {
                let item = Arc::new(Mutex::new(template.deepclone()));
                mark_dropped(&item, now);
                floor.insert(0, item);
            } else {
                *floor_stack.entry(key.to_string()).or_insert(0) += 1;
            }
        } else {
            *floor_stack.entry(key.to_string()).or_insert(0) += 1;
        }
        *room_item_count += 1;
        if details.one_item {
            oneitems.dropped(&details.index, owner);
        }
        true
    } else {
        if details.one_item {
            oneitems.destroyed(&details.index);
        }
        false
    }
}

fn parse_selected(line: &str) -> (String, i64, usize) {
    let args: Vec<&str> = line.split_whitespace().collect();
    let mut count = args
        .get(1)
        .map(|value| parse_int_prefix(value))
        .unwrap_or(1)
        .clamp(1, 50) as usize;
    let original_name = args.first().copied().unwrap_or("");
    let parsed_order = parse_int_prefix(original_name);
    let (name, order) = if parsed_order != 0 {
        let mut stripped_name = original_name.to_string();
        // Python reuses the count variable `i` as this loop index. Preserve
        // that observable bug: `1검 5` therefore drops one, and the numeric-
        // only name `1` leaves the count at zero.
        for (index, (byte_index, character)) in original_name.char_indices().enumerate() {
            count = index;
            if !character.is_ascii_digit() {
                stripped_name = original_name[byte_index..].to_string();
                break;
            }
        }
        (stripped_name, parsed_order)
    } else {
        (original_name.to_string(), 1)
    };
    if order != 1 {
        count = 1;
    }
    (name, order, count)
}

fn parse_int_prefix(value: &str) -> i64 {
    if value.is_empty() {
        return 0;
    }
    if let Ok(integer) = value.parse::<i64>() {
        return integer;
    }
    if !value.starts_with(|character: char| character.is_ascii_digit()) {
        return 0;
    }
    let digits: String = value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

fn drop_all(
    body: &mut Body,
    floor: &mut Vec<Arc<Mutex<Object>>>,
    floor_stack: &mut HashMap<String, i64>,
    room_item_count: usize,
    now: f64,
    oneitems: &mut dyn OneItemActions,
) -> DropResult {
    let owner = body.get_name();
    let inventory = body.object.objs.clone();
    let mut result = DropResult::empty(DropStatus::NotFound, true);
    let mut room_item_count = room_item_count;

    for item in inventory {
        let details = match item.lock() {
            Ok(item) => item_details(&item),
            Err(_) => continue,
        };
        // Python order: inUse -> 버리지못함 -> 출력안함.
        if details.in_use || details.cannot_drop || details.hidden {
            continue;
        }
        body.object.remove(&item);
        result.removed += 1;
        if move_or_destroy(
            item,
            &details,
            floor,
            &mut room_item_count,
            &owner,
            now,
            oneitems,
        ) {
            increment_group(&mut result.dropped, &details);
        } else {
            increment_group(&mut result.failed, &details);
        }
    }

    // `inv_stack` has no Python `ob.objs` insertion sequence. Keep the
    // repository's existing deterministic representation (object instances
    // first, then stack keys in key order) without claiming mixed-order
    // parity. Each quantity still behaves as one Python Item for filters,
    // room capacity, destruction, and output aggregation.
    let mut stacks: Vec<(String, i64)> = body
        .object
        .inv_stack
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(key, count)| (key.clone(), *count))
        .collect();
    stacks.sort_by(|left, right| left.0.cmp(&right.0));
    for (key, quantity) in stacks {
        let Some(details) = stack_item_details(&key) else {
            continue;
        };
        if details.in_use || details.cannot_drop || details.hidden {
            continue;
        }
        body.object.inv_stack.remove(&key);
        for _ in 0..quantity {
            result.removed += 1;
            if move_stack_unit_or_destroy(
                &key,
                &details,
                floor,
                floor_stack,
                &mut room_item_count,
                &owner,
                now,
                oneitems,
            ) {
                increment_group(&mut result.dropped, &details);
            } else {
                increment_group(&mut result.failed, &details);
            }
        }
    }

    if result.removed > 0 {
        result.status = DropStatus::Ok;
    }
    result
}

fn death_drop_all(
    body: &mut Body,
    floor: &mut Vec<Arc<Mutex<Object>>>,
    floor_stack: &mut HashMap<String, i64>,
    room_item_count: usize,
    now: f64,
    oneitems: &mut dyn OneItemActions,
) -> (DropResult, i64) {
    let (insurance_unit, dispatch_rate) = std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|source| serde_json::from_str::<serde_json::Value>(&source).ok())
        .and_then(|root| root.get("메인설정").cloned())
        .map(|config| {
            (
                config
                    .get("보험료단가")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(80),
                config
                    .get("보험출장률")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(30),
            )
        })
        .unwrap_or((80, 30));
    let level = body.get_int("레벨");
    let premium = body.get_int("보험료");
    let insurance_count = if level > 0 && insurance_unit > 0 {
        premium / (level * insurance_unit)
    } else {
        0
    };
    let owner = body.get_name();
    let mut result = DropResult::empty(DropStatus::NotFound, true);
    let mut insured = 0_i64;
    let mut room_item_count = room_item_count;

    for item in body.object.objs.clone() {
        let details = match item.lock() {
            Ok(item) => item_details(&item),
            Err(_) => continue,
        };
        if insurance_count > 0 && !details.insurance_excluded {
            insured += 1;
            continue;
        }
        if details.cannot_give || details.cannot_drop || details.hidden {
            continue;
        }
        if let Ok(mut item) = item.lock() {
            item.set("inUse", 0_i64);
        }
        body.object.remove(&item);
        result.removed += 1;
        let mut movement_details = details.clone();
        if details.one_item {
            // Python death drop calls ONEITEM.drop2 before checking the room
            // capacity; a full room still clears the character ownership.
            oneitems.dropped(&details.index, &owner);
            movement_details.one_item = false;
        }
        if move_or_destroy(
            item,
            &movement_details,
            floor,
            &mut room_item_count,
            &owner,
            now,
            oneitems,
        ) {
            increment_group(&mut result.dropped, &details);
        } else {
            increment_group(&mut result.failed, &details);
        }
    }

    let mut stacks: Vec<(String, i64)> = body
        .object
        .inv_stack
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(key, count)| (key.clone(), *count))
        .collect();
    stacks.sort_by(|left, right| left.0.cmp(&right.0));
    for (key, quantity) in stacks {
        let Some(details) = stack_item_details(&key) else {
            continue;
        };
        if insurance_count > 0 && !details.insurance_excluded {
            insured += quantity;
            continue;
        }
        if details.cannot_give || details.cannot_drop || details.hidden {
            continue;
        }
        body.object.inv_stack.remove(&key);
        for _ in 0..quantity {
            result.removed += 1;
            let mut movement_details = details.clone();
            if details.one_item {
                oneitems.dropped(&details.index, &owner);
                movement_details.one_item = false;
            }
            if move_stack_unit_or_destroy(
                &key,
                &movement_details,
                floor,
                floor_stack,
                &mut room_item_count,
                &owner,
                now,
                oneitems,
            ) {
                increment_group(&mut result.dropped, &details);
            } else {
                increment_group(&mut result.failed, &details);
            }
        }
    }

    // Python decInsureCount always charges the dispatch fee, even if nothing
    // was dropped or no item was protected.
    let dispatch_fee = (level * insurance_unit * dispatch_rate).div_euclid(100);
    body.set("보험료", premium.saturating_sub(dispatch_fee).max(0));
    body.set("_death_insured_items", insured);
    if result.removed > 0 {
        result.status = DropStatus::Ok;
    }
    (result, insured)
}

fn drop_selected(
    body: &mut Body,
    floor: &mut Vec<Arc<Mutex<Object>>>,
    floor_stack: &mut HashMap<String, i64>,
    room_item_count: usize,
    line: &str,
    now: f64,
    oneitems: &mut dyn OneItemActions,
) -> DropResult {
    let (name, order, count) = parse_selected(line);
    let owner = body.get_name();
    let inventory = body.object.objs.clone();
    let mut result = DropResult::empty(DropStatus::NotFound, false);
    let mut room_item_count = room_item_count;
    let mut occurrence = 0_i64;

    for item in inventory {
        if result.removed as usize >= count {
            break;
        }
        let details = match item.lock() {
            Ok(item) => item_details(&item),
            Err(_) => continue,
        };
        if !matches_name(&details, &name) {
            continue;
        }
        // Python order: 출력안함 -> inUse -> order count -> 버리지못함.
        if details.hidden || details.in_use {
            continue;
        }
        occurrence += 1;
        if occurrence < order {
            continue;
        }
        if details.cannot_drop {
            if result.removed == 0 {
                result.status = DropStatus::Blocked;
                return result;
            }
            continue;
        }

        body.object.remove(&item);
        result.removed += 1;
        if move_or_destroy(
            item,
            &details,
            floor,
            &mut room_item_count,
            &owner,
            now,
            oneitems,
        ) {
            increment_group(&mut result.dropped, &details);
        } else {
            insert_selected_failure_bug(&mut result.failed, &details);
        }
    }

    if (result.removed as usize) < count {
        let mut stacks: Vec<(String, i64)> = body
            .object
            .inv_stack
            .iter()
            .filter(|(_, quantity)| **quantity > 0)
            .map(|(key, quantity)| (key.clone(), *quantity))
            .collect();
        stacks.sort_by(|left, right| left.0.cmp(&right.0));

        for (key, quantity) in stacks {
            let Some(details) = stack_item_details(&key) else {
                continue;
            };
            if !matches_name(&details, &name) || details.hidden || details.in_use {
                continue;
            }

            let mut removed_from_stack = 0_i64;
            for _ in 0..quantity {
                if result.removed as usize >= count {
                    break;
                }
                occurrence += 1;
                if occurrence < order {
                    continue;
                }
                if details.cannot_drop {
                    if result.removed == 0 {
                        result.status = DropStatus::Blocked;
                        return result;
                    }
                    continue;
                }

                removed_from_stack += 1;
                result.removed += 1;
                if move_stack_unit_or_destroy(
                    &key,
                    &details,
                    floor,
                    floor_stack,
                    &mut room_item_count,
                    &owner,
                    now,
                    oneitems,
                ) {
                    increment_group(&mut result.dropped, &details);
                } else {
                    insert_selected_failure_bug(&mut result.failed, &details);
                }
            }

            if removed_from_stack > 0 {
                let should_remove = body
                    .object
                    .inv_stack
                    .get_mut(&key)
                    .is_some_and(|remaining| {
                        *remaining -= removed_from_stack;
                        *remaining <= 0
                    });
                if should_remove {
                    body.object.inv_stack.remove(&key);
                }
            }
            if result.removed as usize >= count {
                break;
            }
        }
    }

    if result.removed > 0 {
        result.status = DropStatus::Ok;
    }
    result
}

fn current_unix_seconds() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

pub(super) fn register_drop_item_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    let body_ptr_env = body_ptr;
    engine.register_fn("has_drop_environment", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &*body_ptr_env };
        get_world_state()
            .read()
            .ok()
            .is_some_and(|world| world.get_player_position(&body.get_name()).is_some())
    });

    let body_ptr_drop = body_ptr;
    engine.register_fn(
        "drop_inventory_python",
        move |_ob: &mut rhai::Map, line: &str, all: bool| -> Dynamic {
            let body = unsafe { &mut *body_ptr_drop };
            let mut world = match get_world_state().write() {
                Ok(world) => world,
                Err(_) => return DropResult::empty(DropStatus::NotFound, all).into_dynamic(),
            };
            let (zone, room) = match world.get_player_position(&body.get_name()) {
                Some(position) => (position.zone.clone(), position.room.clone()),
                None => return DropResult::empty(DropStatus::NotFound, all).into_dynamic(),
            };
            let room_key = format!("{zone}:{room}");
            let room_item_count = world.room_objs.get(&room_key).map_or(0, Vec::len)
                + world
                    .room_inv_stack
                    .get(&room_key)
                    .into_iter()
                    .flat_map(HashMap::values)
                    .map(|count| (*count).max(0) as usize)
                    .sum::<usize>();
            let mut oneitems = PersistentOneItemActions;
            let result = {
                let world = &mut *world;
                let floor = world.room_objs.entry(room_key.clone()).or_default();
                let floor_stack = world.room_inv_stack.entry(room_key).or_default();
                if all {
                    drop_all(
                        body,
                        floor,
                        floor_stack,
                        room_item_count,
                        current_unix_seconds(),
                        &mut oneitems,
                    )
                } else {
                    drop_selected(
                        body,
                        floor,
                        floor_stack,
                        room_item_count,
                        line,
                        current_unix_seconds(),
                        &mut oneitems,
                    )
                }
            };
            world.sync_floor_item_order(&zone, &room);
            result.into_dynamic()
        },
    );
    let body_ptr_death = body_ptr;
    engine.register_fn(
        "death_drop_inventory_python",
        move |_ob: &mut rhai::Map| -> Dynamic {
            let body = unsafe { &mut *body_ptr_death };
            let mut world = match get_world_state().write() {
                Ok(world) => world,
                Err(_) => {
                    return DropResult::empty(DropStatus::NotFound, true).into_death_dynamic(0)
                }
            };
            let (zone, room) = match world.get_player_position(&body.get_name()) {
                Some(position) => (position.zone.clone(), position.room.clone()),
                None => return DropResult::empty(DropStatus::NotFound, true).into_death_dynamic(0),
            };
            let room_key = format!("{zone}:{room}");
            let room_item_count = world.room_objs.get(&room_key).map_or(0, Vec::len)
                + world
                    .room_inv_stack
                    .get(&room_key)
                    .into_iter()
                    .flat_map(HashMap::values)
                    .map(|count| (*count).max(0) as usize)
                    .sum::<usize>();
            let mut oneitems = PersistentOneItemActions;
            let (result, insured) = {
                let world = &mut *world;
                let floor = world.room_objs.entry(room_key.clone()).or_default();
                let floor_stack = world.room_inv_stack.entry(room_key).or_default();
                death_drop_all(
                    body,
                    floor,
                    floor_stack,
                    room_item_count,
                    current_unix_seconds(),
                    &mut oneitems,
                )
            };
            world.sync_floor_item_order(&zone, &room);
            result.into_death_dynamic(insured)
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

    static SCRIPT_TEST_ID: AtomicUsize = AtomicUsize::new(0);

    #[derive(Default)]
    struct RecordingOneItems {
        dropped: Vec<(String, String)>,
        destroyed: Vec<String>,
    }

    impl OneItemActions for RecordingOneItems {
        fn dropped(&mut self, index: &str, owner: &str) {
            self.dropped.push((index.to_string(), owner.to_string()));
        }

        fn destroyed(&mut self, index: &str) {
            self.destroyed.push(index.to_string());
        }
    }

    fn item(
        index: &str,
        name: &str,
        reactions: &str,
        attrs: &str,
        in_use: bool,
    ) -> Arc<Mutex<Object>> {
        let mut item = Object::new();
        item.set("인덱스", index);
        item.set("이름", name);
        item.set("종류", "기타");
        item.set("반응이름", reactions);
        item.set("아이템속성", attrs);
        item.set("inUse", if in_use { 1_i64 } else { 0_i64 });
        Arc::new(Mutex::new(item))
    }

    fn floor(count: usize) -> Vec<Arc<Mutex<Object>>> {
        (0..count)
            .map(|index| item(&format!("floor-{index}"), "바닥돌", "돌", "", false))
            .collect()
    }

    fn run_drop_script(body: &mut Body, line: &str) -> (Vec<String>, Vec<(String, String)>) {
        let outputs = Arc::new(Mutex::new(Vec::new()));
        let special = Arc::new(Mutex::new(None::<CommandResult>));
        let user_sends = Arc::new(Mutex::new(Vec::new()));
        let engine = super::super::create_engine_with_body_and_output(
            body,
            outputs.clone(),
            None,
            None,
            special,
            user_sends.clone(),
            None,
            Some("버려"),
            None,
        );
        let mut scope = Scope::new();
        scope.push("ob", Dynamic::from(super::super::build_ob_from_body(body)));
        scope.push("cmdline", line.to_string());
        let source = include_str!("../../cmds/버려.rhai");
        engine
            .run_with_scope(&mut scope, &format!("{source}\nmain(ob, cmdline)"))
            .unwrap();

        let output = outputs.lock().unwrap().clone();
        let sends = user_sends.lock().unwrap().clone();
        (output, sends)
    }

    fn run_death_script(body: &mut Body) -> Vec<String> {
        let outputs = Arc::new(Mutex::new(Vec::new()));
        let special = Arc::new(Mutex::new(None::<CommandResult>));
        let user_sends = Arc::new(Mutex::new(Vec::new()));
        let engine = super::super::create_engine_with_body_and_output(
            body,
            outputs.clone(),
            None,
            None,
            special,
            user_sends,
            None,
            Some("__death"),
            None,
        );
        let mut scope = Scope::new();
        scope.push("ob", Dynamic::from(super::super::build_ob_from_body(body)));
        scope.push("cmdline", String::new());
        let library = include_str!("../../lib/death.rhai");
        let source = include_str!("../../cmds/__death.rhai");
        engine
            .run_with_scope(
                &mut scope,
                &format!("{library}\n{source}\nmain(ob, cmdline)"),
            )
            .unwrap();
        let result = outputs.lock().unwrap().clone();
        result
    }

    fn script_room_names() -> (String, String, String, String, String) {
        let id = SCRIPT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let suffix = char::from_u32(0xAC00 + (id % 11_172) as u32).unwrap();
        (
            format!("버려회귀영역{suffix}"),
            format!("버려회귀방{suffix}"),
            format!("버린이{suffix}"),
            format!("관찰자{suffix}"),
            format!("다른방{suffix}"),
        )
    }

    fn clear_script_room(zone: &str, room: &str, names: &[&str]) {
        let mut world = get_world_state().write().unwrap();
        for name in names {
            world.remove_player_position(name);
        }
        world.get_room_objs_mut(zone, room).clear();
        world.get_room_objs_stack_mut(zone, room).clear();
    }

    #[test]
    fn selected_drop_preserves_filters_order_count_clamp_and_insertion_order() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        let hidden = item("hidden", "검", "칼", "출력안함", false);
        let worn = item("worn", "검", "칼", "", true);
        let first = item("first", "검", "칼", "", false);
        let second = item("second", "검", "칼", "", false);
        let third = item("third", "검", "칼", "", false);
        for item in [&hidden, &worn, &first, &second, &third] {
            body.object.objs.push(item.clone());
        }
        let mut floor = floor(0);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let result = drop_selected(
            &mut body,
            &mut floor,
            &mut floor_stack,
            0,
            "2칼 99",
            123.5,
            &mut oneitems,
        );

        // order != 1 forces count to one; hidden/inUse do not count toward order.
        assert_eq!(result.status, DropStatus::Ok);
        assert_eq!(result.removed, 1);
        assert!(Arc::ptr_eq(&floor[0], &second));
        assert!(matches!(
            second.lock().unwrap().temp.get(DROP_TIME_KEY),
            Some(Value::Float(value)) if (*value - 123.5).abs() < f64::EPSILON
        ));
        assert_eq!(body.object.objs.len(), 4);
    }

    #[test]
    fn death_drop_applies_python_insurance_filters_fee_and_room_limit() {
        let mut body = Body::new();
        body.set("이름", "사망자");
        body.set("레벨", 10_i64);
        body.set("보험료", 800_i64); // getInsureCount == 1
        let insured_a = item("a", "보험검", "검", "", false);
        let insured_b = item("b", "보험도", "도", "", false);
        let excluded = item("c", "무보험창", "창", "보험적용안됨", false);
        let protected = item("d", "보존봉", "봉", "보험적용안됨 줄수없음", false);
        body.object.objs = vec![
            insured_a.clone(),
            insured_b.clone(),
            excluded.clone(),
            protected.clone(),
        ];
        let mut floor = floor(50);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let (result, insured) = death_drop_all(
            &mut body,
            &mut floor,
            &mut floor_stack,
            50,
            100.0,
            &mut oneitems,
        );

        assert_eq!(insured, 2);
        assert_eq!(result.removed, 1);
        assert!(result.dropped.is_empty());
        assert_eq!(result.failed[0].name, "무보험창");
        assert_eq!(body.object.objs.len(), 3);
        assert!(body
            .object
            .objs
            .iter()
            .any(|item| Arc::ptr_eq(item, &protected)));
        assert_eq!(body.get_int("보험료"), 560); // 800 - 10*80*30//100
        assert_eq!(body.get_int("_death_insured_items"), 2);
    }

    #[test]
    fn death_rhai_owns_drop_and_coma_output_order() {
        let (zone, room, player_name, _, _) = script_room_names();
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), room.clone()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.clone());
        body.set("레벨", 10_i64);
        body.set("보험료", 0_i64);
        body.object
            .objs
            .push(item("death-item", "청동검", "검", "", false));

        let outputs = run_death_script(&mut body);
        assert!(outputs[0].contains("청동검"));
        assert!(outputs[0].ends_with("떨어뜨립니다."));
        assert_eq!(outputs.last().unwrap(), "당신은 정신이 혼미합니다.");
        assert!(body.object.objs.is_empty());
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_room_objs(&zone, &room)
                .len(),
            1
        );
        clear_script_room(&zone, &room, &[&player_name]);
    }

    #[test]
    fn all_drop_skips_in_use_hidden_and_forbidden_and_aggregates_by_first_name() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        let kept_worn = item("worn", "검", "칼", "", true);
        let kept_hidden = item("hidden", "검", "칼", "출력안함", false);
        let kept_forbidden = item("forbidden", "검", "칼", "버리지못함", false);
        let first = item("first", "검", "칼", "", false);
        let herb = item("herb", "약초", "풀", "", false);
        let second = item("second", "검", "칼", "", false);
        for item in [
            &kept_worn,
            &kept_hidden,
            &kept_forbidden,
            &first,
            &herb,
            &second,
        ] {
            body.object.objs.push(item.clone());
        }
        let mut floor = floor(0);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let result = drop_all(
            &mut body,
            &mut floor,
            &mut floor_stack,
            0,
            5.0,
            &mut oneitems,
        );

        assert_eq!(result.removed, 3);
        assert_eq!(result.dropped[0].name, "검");
        assert_eq!(result.dropped[0].count, 2);
        assert_eq!(result.dropped[1].name, "약초");
        // Room.insert() on each iteration reverses the three dropped objects.
        assert!(Arc::ptr_eq(&floor[0], &second));
        assert!(Arc::ptr_eq(&floor[1], &herb));
        assert!(Arc::ptr_eq(&floor[2], &first));
        assert_eq!(body.object.objs.len(), 3);
    }

    #[test]
    fn capacity_49_moves_one_then_destroys_and_selected_failure_count_stays_one() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        let one = item("unique-1", "비보", "보물", "단일아이템", false);
        let two = item("unique-2", "비보", "보물", "단일아이템", false);
        let three = item("unique-3", "비보", "보물", "단일아이템", false);
        body.object.objs = vec![one.clone(), two, three];
        let mut floor = floor(49);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let result = drop_selected(
            &mut body,
            &mut floor,
            &mut floor_stack,
            49,
            "보물 3",
            9.0,
            &mut oneitems,
        );

        assert_eq!(result.removed, 3);
        assert_eq!(floor.len(), 50);
        assert!(Arc::ptr_eq(&floor[0], &one));
        assert_eq!(result.dropped[0].count, 1);
        // Original selected-branch bug: two destroyed objects still report one.
        assert_eq!(result.failed[0].count, 1);
        assert_eq!(oneitems.dropped, vec![("unique-1".into(), "버린이".into())]);
        assert_eq!(oneitems.destroyed, vec!["unique-2", "unique-3"]);
    }

    #[test]
    fn capacity_50_destroys_all_and_all_branch_counts_every_failure() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        body.object.objs = vec![
            item("first", "검", "칼", "", false),
            item("second", "검", "칼", "", false),
        ];
        let mut floor = floor(50);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let result = drop_all(
            &mut body,
            &mut floor,
            &mut floor_stack,
            50,
            11.0,
            &mut oneitems,
        );

        assert_eq!(floor.len(), 50);
        assert_eq!(result.failed[0].count, 2);
        assert!(body.object.objs.is_empty());
    }

    #[test]
    fn selected_forbidden_first_candidate_returns_blocked_without_mutation() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        let forbidden = item("no", "검", "칼", "버리지못함", false);
        let allowed = item("yes", "검", "칼", "", false);
        body.object.objs = vec![forbidden, allowed];
        let mut floor = floor(0);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let result = drop_selected(
            &mut body,
            &mut floor,
            &mut floor_stack,
            0,
            "칼 2",
            1.0,
            &mut oneitems,
        );

        assert_eq!(result.status, DropStatus::Blocked);
        assert_eq!(result.removed, 0);
        assert_eq!(body.object.objs.len(), 2);
        assert!(floor.is_empty());
    }

    #[test]
    fn selected_parser_preserves_python_count_loop_variable_reuse() {
        assert_eq!(parse_selected("검 50"), ("검".to_string(), 1, 50));
        assert_eq!(parse_selected("1검 50"), ("검".to_string(), 1, 1));
        assert_eq!(parse_selected("2검 50"), ("검".to_string(), 2, 1));
        assert_eq!(parse_selected("1 50"), ("1".to_string(), 1, 0));
    }

    #[test]
    fn stack_metadata_keeps_python_reaction_list_membership_exact() {
        let (item, _) = super::super::object_from_item_json("1037").unwrap();
        let item = item.lock().unwrap();
        // inventory_compat preserves Python JSON array element boundaries
        // with newlines; item_details accepts whitespace separators.
        assert_eq!(item.getString("반응이름"), "탕수육\n탕수\n탕슉");
        let details = item_details(&item);
        drop(item);

        assert_eq!(details.reactions, vec!["탕수육", "탕수", "탕슉"]);
        assert!(matches_name(&details, "탕수"));
        assert!(matches_name(&details, "탕슉"));
        assert!(!matches_name(&details, "탕"));

        let forbidden = stack_item_details("도전장1").unwrap();
        assert!(forbidden.cannot_drop);
    }

    #[test]
    fn selected_stack_quantity_obeys_capacity_and_selected_failure_count_bug() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        body.object.inv_stack.insert("1037".to_string(), 3);
        let mut floor = floor(49);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let result = drop_selected(
            &mut body,
            &mut floor,
            &mut floor_stack,
            49,
            "탕수 3",
            1.0,
            &mut oneitems,
        );

        assert_eq!(result.status, DropStatus::Ok);
        assert_eq!(result.removed, 3);
        assert_eq!(result.dropped[0].name, "탕수육");
        assert_eq!(result.dropped[0].count, 1);
        assert_eq!(result.failed[0].name, "탕수육");
        assert_eq!(result.failed[0].count, 1);
        assert_eq!(floor.len(), 50);
        assert!(floor_stack.get("1037").is_none());
        assert!(!body.object.inv_stack.contains_key("1037"));
    }

    #[test]
    fn all_stack_destroys_at_capacity_and_keeps_forbidden_and_hidden_entries() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        body.object.inv_stack.insert("1037".to_string(), 2);
        body.object.inv_stack.insert("도전장1".to_string(), 2);
        body.object.inv_stack.insert("사강시".to_string(), 1);
        let mut floor = floor(50);
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let result = drop_all(
            &mut body,
            &mut floor,
            &mut floor_stack,
            50,
            1.0,
            &mut oneitems,
        );

        assert_eq!(result.status, DropStatus::Ok);
        assert_eq!(result.removed, 2);
        assert_eq!(result.failed[0].name, "탕수육");
        assert_eq!(result.failed[0].count, 2);
        assert!(floor_stack.is_empty());
        assert!(!body.object.inv_stack.contains_key("1037"));
        assert_eq!(body.object.inv_stack.get("도전장1"), Some(&2));
        assert_eq!(body.object.inv_stack.get("사강시"), Some(&1));
    }

    #[test]
    fn selected_forbidden_stack_blocks_and_reaction_prefix_does_not_match() {
        let mut body = Body::new();
        body.set("이름", "버린이");
        body.object.inv_stack.insert("1037".to_string(), 1);
        body.object.inv_stack.insert("도전장1".to_string(), 2);
        let mut floor = Vec::new();
        let mut floor_stack = HashMap::new();
        let mut oneitems = RecordingOneItems::default();

        let prefix = drop_selected(
            &mut body,
            &mut floor,
            &mut floor_stack,
            0,
            "탕",
            1.0,
            &mut oneitems,
        );
        assert_eq!(prefix.status, DropStatus::NotFound);
        assert_eq!(body.object.inv_stack.get("1037"), Some(&1));

        let forbidden = drop_selected(
            &mut body,
            &mut floor,
            &mut floor_stack,
            0,
            "구위",
            1.0,
            &mut oneitems,
        );
        assert_eq!(forbidden.status, DropStatus::Blocked);
        assert_eq!(body.object.inv_stack.get("도전장1"), Some(&2));
        assert!(floor.is_empty());
        assert!(floor_stack.is_empty());
    }

    #[test]
    fn rhai_checks_missing_environment_before_silver_and_uses_no_global_player_scan() {
        let source = include_str!("../../cmds/버려.rhai");
        assert!(!source.contains("get_all_online_players"));
        assert!(source.contains("get_room_players(ob)"));

        let mut body = Body::new();
        body.set("이름", "환경없음버려검사");
        let (outputs, sends) = run_drop_script(&mut body, "은전");

        assert_eq!(outputs, vec!["☞ 아무것도 버릴수 없습니다."]);
        assert!(sends.is_empty());
    }

    #[test]
    fn rhai_selected_drop_formats_python_plural_output_and_only_notifies_same_room() {
        use crate::script::party::set_precomputed_party_context;

        let (zone, room, actor_name, observer_name, other_name) = script_room_names();
        let mut body = Body::new();
        body.set("이름", actor_name.as_str());
        body.object.objs = vec![
            item("first", "검", "칼", "", false),
            item("second", "검", "칼", "", false),
        ];
        {
            let mut world = get_world_state().write().unwrap();
            world.get_room_objs_mut(&zone, &room).clear();
            world.get_room_objs_stack_mut(&zone, &room).clear();
            world.set_player_position(&actor_name, PlayerPosition::new(zone.clone(), room.clone()));
            world.set_player_position(
                &observer_name,
                PlayerPosition::new(zone.clone(), room.clone()),
            );
            world.set_player_position(
                &other_name,
                PlayerPosition::new(zone.clone(), "다른방".to_string()),
            );
        }
        let mut person = rhai::Map::new();
        person.insert("name".into(), Dynamic::from(observer_name.clone()));
        person.insert("show_prompt".into(), Dynamic::from(true));
        person.insert("hp".into(), Dynamic::from(17_i64));
        person.insert("max_hp".into(), Dynamic::from(28_i64));
        person.insert("mp".into(), Dynamic::from(3_i64));
        person.insert("max_mp".into(), Dynamic::from(4_i64));
        let mut context = rhai::Map::new();
        context.insert(
            "room_players".into(),
            Dynamic::from(vec![Dynamic::from(person)]),
        );
        set_precomputed_party_context(context);

        let (outputs, sends) = run_drop_script(&mut body, "칼 2");
        let actor = format!(
            "\x1b[1m{actor_name}\x1b[0;37m{}",
            crate::hangul::han_iga(&actor_name)
        );
        assert_eq!(outputs, vec!["당신이 \x1b[36m검\x1b[37m 2개를 버립니다."]);
        assert_eq!(
            sends,
            vec![(
                observer_name.clone(),
                format!(
                    "{}\r\n{actor} \x1b[36m검\x1b[37m 2개를 버립니다.\r\n\r\n\x1b[0;37;40m[ 17/28, 3/4 ] ",
                    crate::script::RAW_USER_MESSAGE_PREFIX,
                )
            )]
        );
        assert!(sends.iter().all(|(name, _)| name != &other_name));
        assert!(body.object.objs.is_empty());
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_room_objs(&zone, &room)
                .len(),
            2
        );

        clear_script_room(&zone, &room, &[&actor_name, &observer_name, &other_name]);
        set_precomputed_party_context(rhai::Map::new());
    }

    #[test]
    fn rhai_selected_stack_drop_uses_python_quantity_output_and_room_capacity_state() {
        let (zone, room, actor_name, observer_name, other_name) = script_room_names();
        let mut body = Body::new();
        body.set("이름", actor_name.as_str());
        body.object.inv_stack.insert("1037".to_string(), 2);
        {
            let mut world = get_world_state().write().unwrap();
            world.get_room_objs_mut(&zone, &room).clear();
            world.get_room_objs_stack_mut(&zone, &room).clear();
            world.set_player_position(&actor_name, PlayerPosition::new(zone.clone(), room.clone()));
            world.set_player_position(
                &observer_name,
                PlayerPosition::new(zone.clone(), room.clone()),
            );
            world.set_player_position(
                &other_name,
                PlayerPosition::new(zone.clone(), "다른방".to_string()),
            );
        }

        let (outputs, sends) = run_drop_script(&mut body, "탕수 2");
        let actor = format!(
            "\x1b[1m{actor_name}\x1b[0;37m{}",
            crate::hangul::han_iga(&actor_name)
        );
        assert_eq!(
            outputs,
            vec!["당신이 \x1b[36m탕수육\x1b[37m 2개를 버립니다."]
        );
        assert_eq!(
            sends,
            vec![(
                observer_name.clone(),
                format!(
                    "{}\r\n{actor} \x1b[36m탕수육\x1b[37m 2개를 버립니다.\r\n",
                    crate::script::RAW_USER_MESSAGE_PREFIX,
                )
            )]
        );
        assert!(!body.object.inv_stack.contains_key("1037"));
        {
            let world = get_world_state().read().unwrap();
            assert!(world.get_room_objs_stack(&zone, &room).is_empty());
            assert_eq!(world.get_room_objs(&zone, &room).len(), 2);
            let order = world.get_room_object_order(&zone, &room);
            assert!(matches!(
                order.as_slice(),
                [
                    crate::world::RoomObjectRef::FloorItem(_),
                    crate::world::RoomObjectRef::FloorItem(_),
                    crate::world::RoomObjectRef::Player(observer),
                    crate::world::RoomObjectRef::Player(actor),
                ] if observer == &observer_name && actor == &actor_name
            ));
        }
        let expected = get_world_state()
            .read()
            .unwrap()
            .get_room_object_order(&zone, &room)
            .first()
            .cloned();
        assert_eq!(
            super::super::select_python_room_object(&body, "탕수"),
            expected,
            "Python Room.findObjName must see a compact inventory item immediately after it is dropped"
        );

        clear_script_room(&zone, &room, &[&actor_name, &observer_name, &other_name]);
    }

    #[test]
    fn legacy_room_stack_materializes_before_python_room_lookup() {
        let (zone, room, actor_name, observer_name, other_name) = script_room_names();
        let mut body = Body::new();
        body.set("이름", actor_name.as_str());
        {
            let mut world = get_world_state().write().unwrap();
            world.get_room_objs_mut(&zone, &room).clear();
            world.get_room_objs_stack_mut(&zone, &room).clear();
            world
                .get_room_objs_stack_mut(&zone, &room)
                .insert("1037".to_string(), 2);
            world.set_player_position(&actor_name, PlayerPosition::new(zone.clone(), room.clone()));
            world.set_player_position(
                &observer_name,
                PlayerPosition::new(zone.clone(), room.clone()),
            );
        }

        super::super::materialize_legacy_room_stacks(&body);

        let expected = {
            let world = get_world_state().read().unwrap();
            assert!(world.get_room_objs_stack(&zone, &room).is_empty());
            assert_eq!(world.get_room_objs(&zone, &room).len(), 2);
            let order = world.get_room_object_order(&zone, &room);
            assert!(matches!(
                order.as_slice(),
                [
                    crate::world::RoomObjectRef::FloorItem(_),
                    crate::world::RoomObjectRef::FloorItem(_),
                    crate::world::RoomObjectRef::Player(observer),
                    crate::world::RoomObjectRef::Player(actor),
                ] if observer == &observer_name && actor == &actor_name
            ));
            order.first().cloned()
        };
        assert_eq!(
            super::super::select_python_room_object(&body, "탕수"),
            expected
        );

        clear_script_room(&zone, &room, &[&actor_name, &observer_name, &other_name]);
    }

    #[test]
    fn unknown_legacy_room_stack_is_quarantined_without_inventing_an_item() {
        let (zone, room, actor_name, observer_name, other_name) = script_room_names();
        let mut body = Body::new();
        body.set("이름", actor_name.as_str());
        let unknown = "없는-아이템-카탈로그";
        {
            let mut world = get_world_state().write().unwrap();
            world.get_room_objs_mut(&zone, &room).clear();
            world.get_room_objs_stack_mut(&zone, &room).clear();
            world
                .get_room_objs_stack_mut(&zone, &room)
                .insert(unknown.to_string(), 2);
            world.set_player_position(&actor_name, PlayerPosition::new(zone.clone(), room.clone()));
        }

        super::super::materialize_legacy_room_stacks(&body);

        let world = get_world_state().read().unwrap();
        assert!(world.get_room_objs(&zone, &room).is_empty());
        assert_eq!(
            world.get_room_objs_stack(&zone, &room).get(unknown),
            Some(&2)
        );
        drop(world);
        assert_eq!(
            super::super::select_python_room_object(&body, unknown),
            None
        );

        clear_script_room(&zone, &room, &[&actor_name, &observer_name, &other_name]);
    }

    #[test]
    fn rhai_all_capacity_failure_keeps_python_actor_double_space_bug() {
        let (zone, room, actor_name, observer_name, other_name) = script_room_names();
        let mut body = Body::new();
        body.set("이름", actor_name.as_str());
        body.object.objs = vec![item("broken", "검", "칼", "", false)];
        {
            let mut world = get_world_state().write().unwrap();
            *world.get_room_objs_mut(&zone, &room) = floor(50);
            world.get_room_objs_stack_mut(&zone, &room).clear();
            world.set_player_position(&actor_name, PlayerPosition::new(zone.clone(), room.clone()));
            world.set_player_position(
                &observer_name,
                PlayerPosition::new(zone.clone(), room.clone()),
            );
            world.set_player_position(
                &other_name,
                PlayerPosition::new(zone.clone(), "다른방".to_string()),
            );
        }

        let (outputs, sends) = run_drop_script(&mut body, "모두");
        let post = "\x1b[0;36m검\x1b[37m을";
        let actor = format!(
            "\x1b[1m{actor_name}\x1b[0;37m{}",
            crate::hangul::han_iga(&actor_name)
        );
        assert_eq!(
            outputs,
            vec![format!(
                "당신이 \x1b[36m{post}\x1b[37m  버리자 바로 부서집니다."
            )]
        );
        assert_eq!(
            sends,
            vec![(
                observer_name.clone(),
                format!(
                    "{}\r\n{actor} \x1b[36m{post}\x1b[37m 버리자 바로 부서집니다.\r\n",
                    crate::script::RAW_USER_MESSAGE_PREFIX,
                )
            )]
        );
        assert!(body.object.objs.is_empty());
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_room_objs(&zone, &room)
                .len(),
            50
        );

        clear_script_room(&zone, &room, &[&actor_name, &observer_name, &other_name]);
    }
}
