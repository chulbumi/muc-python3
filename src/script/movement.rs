//! Python `Player.parse_command` one-word exit and `enterRoom` state efuns.
//!
//! This module deliberately returns only structured data/status codes.  The
//! hot-reloaded `cmds/__movement.rhai` script owns every visible byte.

use super::return_home::{first_hazard, json_string, property_limit, python_int};
use super::{current_body_position, room_view_player_snapshots};
use crate::object::Value;
use crate::player::{ActState, Body};
use crate::world::{base_zone_name, get_world_state, PlayerPosition};
use rand::Rng;
use rhai::{Array, Dynamic, Engine, Map};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::{HashMap, HashSet};

const ENTRY_EVENT_KEY: &str = "이벤트 $%입장이벤트%";
const AFTER_FIGHT_ROUTE_FINISHED: &str = "_after_fight_route_finished";
const DIRECTIONS: &[&str] = &[
    "동", "서", "남", "북", "위", "아래", "남동", "남서", "북동", "북서",
];

fn take_after_fight_route_finished(body: &mut Body) -> bool {
    matches!(
        body.temp_mut().remove(AFTER_FIGHT_ROUTE_FINISHED),
        Some(Value::Int(1))
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AccessibleExit {
    name: String,
    destinations: Vec<String>,
    random_destination: bool,
    hidden: bool,
}

fn room_info(zone: &str, room: &str) -> Option<JsonMap<String, JsonValue>> {
    let path = std::path::Path::new("data/map")
        .join(base_zone_name(zone))
        .join(format!("{room}.json"));
    let source = std::fs::read_to_string(path).ok()?;
    let root: JsonValue = serde_json::from_str(&source).ok()?;
    root.get("맵정보")?.as_object().cloned()
}

fn json_strings(value: Option<&JsonValue>) -> Vec<String> {
    match value {
        Some(JsonValue::String(value)) => vec![value.clone()],
        Some(JsonValue::Array(values)) => values
            .iter()
            .filter_map(JsonValue::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn room_properties(info: &JsonMap<String, JsonValue>) -> Vec<String> {
    json_strings(info.get("맵속성"))
}

/// Reproduce `Room.initExit()` followed by `Room.setHiddenExit()`.
///
/// The prefix search iterates `Room.Exits`, not `exitList`: hidden keys are
/// deleted and reinserted without `$`, which moves them to the end unless the
/// stripped key already exists.  A Vec keeps that Python dict ordering without
/// relying on Rust HashMap iteration.
fn accessible_exits(info: &JsonMap<String, JsonValue>) -> Vec<AccessibleExit> {
    let mut exits: Vec<(String, Vec<String>, bool)> = Vec::new();
    for line in json_strings(info.get("출구")) {
        let words = line.split_whitespace().collect::<Vec<_>>();
        if words.len() < 2 {
            continue;
        }
        let name = words[0].to_string();
        let destinations = words[1..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>();
        let random = words.len() > 2;
        if let Some(existing) = exits.iter_mut().find(|entry| entry.0 == name) {
            existing.1 = destinations;
            existing.2 = random;
        } else {
            exits.push((name, destinations, random));
        }
    }

    let copied_names = exits
        .iter()
        .map(|entry| entry.0.clone())
        .collect::<Vec<_>>();
    let hidden_names = copied_names
        .iter()
        .filter_map(|name| name.strip_suffix('$').map(str::to_string))
        .collect::<HashSet<_>>();
    for raw_name in copied_names {
        let Some(display_name) = raw_name.strip_suffix('$') else {
            continue;
        };
        let Some(index) = exits.iter().position(|entry| entry.0 == raw_name) else {
            continue;
        };
        let (_, destinations, random) = exits.remove(index);
        if let Some(existing) = exits.iter_mut().find(|entry| entry.0 == display_name) {
            existing.1 = destinations;
            existing.2 = random;
        } else {
            exits.push((display_name.to_string(), destinations, random));
        }
    }

    exits
        .into_iter()
        .map(|(name, destinations, random_destination)| AccessibleExit {
            hidden: hidden_names.contains(&name),
            name,
            destinations,
            random_destination,
        })
        .collect()
}

/// Reproduce the independently sorted `Room.exitList` used by viewMapData.
fn sorted_exit_names(info: &JsonMap<String, JsonValue>) -> Vec<String> {
    let mut insertion_order = Vec::<String>::new();
    for line in json_strings(info.get("출구")) {
        let words = line.split_whitespace().collect::<Vec<_>>();
        if words.len() < 2 {
            continue;
        }
        let name = words[0].to_string();
        if !insertion_order.iter().any(|existing| existing == &name) {
            insertion_order.push(name);
        }
    }
    let mut ordered = Vec::new();
    for direction in DIRECTIONS {
        if let Some(index) = insertion_order.iter().position(|name| name == direction) {
            ordered.push(insertion_order.remove(index));
        }
    }
    ordered.extend(insertion_order);
    ordered
}

fn resolve_exit_destination(
    current_zone: &str,
    info: &JsonMap<String, JsonValue>,
    destination: &str,
) -> Option<(String, String)> {
    let raw_room_zone = info
        .get("존이름")
        .and_then(JsonValue::as_str)
        .unwrap_or(current_zone);
    let room_zone = if base_zone_name(current_zone) == raw_room_zone {
        current_zone
    } else {
        raw_room_zone
    };
    let (mut zone, room) = if let Some((zone, room)) = destination.split_once(':') {
        (zone.to_string(), room.to_string())
    } else {
        (room_zone.to_string(), destination.to_string())
    };
    if destination.contains(':') {
        if let Some(difficulty) = current_zone.chars().last().filter(char::is_ascii_digit) {
            zone.push(difficulty);
        }
    }
    (!zone.is_empty() && !room.is_empty()).then_some((zone, room))
}

fn effective_room_string(
    world: &crate::world::WorldState,
    zone: &str,
    room: &str,
    info: &JsonMap<String, JsonValue>,
    key: &str,
) -> String {
    world
        .room_attrs
        .get(&format!("{zone}:{room}"))
        .and_then(|attrs| attrs.get(key))
        .cloned()
        .unwrap_or_else(|| json_string(info, key))
}

fn effective_room_int(
    world: &crate::world::WorldState,
    zone: &str,
    room: &str,
    info: &JsonMap<String, JsonValue>,
    key: &str,
) -> i64 {
    world
        .room_attrs
        .get(&format!("{zone}:{room}"))
        .and_then(|attrs| attrs.get(key))
        .map_or_else(
            || python_int(info.get(key)),
            |value| python_int(Some(&JsonValue::String(value.clone()))),
        )
}

fn result_map(status: &str) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from(status.to_string()));
    result.insert("move".into(), Dynamic::from(String::new()));
    result.insert("hidden".into(), Dynamic::from(false));
    result.insert("exit_script".into(), Dynamic::from(String::new()));
    result.insert("entry_script".into(), Dynamic::from(String::new()));
    result.insert("has_hazard".into(), Dynamic::from(false));
    result.insert("hazard_damage".into(), Dynamic::from(0_i64));
    result.insert("hazard_message".into(), Dynamic::from(String::new()));
    result.insert("has_entry_events".into(), Dynamic::from(false));
    // `Player.enterRoom` schedules the first word of this value one second
    // later.  Keep the raw command as state, not presentation; the network
    // boundary owns the delayed input dispatch.
    result.insert("auto_move".into(), Dynamic::from(String::new()));
    result.insert("followers".into(), Dynamic::from(Array::new()));
    result
}

fn command_limited(body: &Body, raw_command: &str) -> bool {
    let Some((zone, room)) = current_body_position(body) else {
        return false;
    };
    let properties = get_world_state()
        .read()
        .ok()
        .and_then(|world| world.room_cache.get_room_cached(&zone, &room))
        .and_then(|room| room.read().ok().map(|room| room.properties.clone()))
        .or_else(|| room_info(&zone, &room).map(|info| room_properties(&info)))
        .unwrap_or_default();
    // Python stores getNextWords(attr) as a string and uses `cmd in
    // limitCmds`, so this is substring membership rather than token lookup.
    properties.iter().any(|property| {
        property
            .strip_prefix("명령금지")
            .is_some_and(|commands| commands.trim_start().contains(raw_command))
    })
}

fn move_through_exit_with_roller(
    body: &mut Body,
    command: &str,
    roll: &mut impl FnMut(usize) -> usize,
) -> Map {
    let player_name = body.get_name();
    let Some((current_zone, current_room)) = current_body_position(body) else {
        return result_map("not_exit");
    };
    let Some(current_info) = room_info(&current_zone, &current_room) else {
        return result_map("not_exit");
    };
    let exits = accessible_exits(&current_info);
    let selected = exits.iter().find(|exit| exit.name == command).or_else(|| {
        if DIRECTIONS.contains(&command) {
            None
        } else {
            exits.iter().find(|exit| exit.name.starts_with(command))
        }
    });
    let Some(selected) = selected.cloned() else {
        return if DIRECTIONS.contains(&command) {
            result_map("direction_missing")
        } else {
            result_map("not_exit")
        };
    };
    if selected.destinations.is_empty() {
        return result_map("move_where");
    }
    let destination_index = if selected.random_destination {
        roll(selected.destinations.len()) % selected.destinations.len()
    } else {
        0
    };
    let Some((destination_zone, destination_room)) = resolve_exit_destination(
        &current_zone,
        &current_info,
        &selected.destinations[destination_index],
    ) else {
        return result_map("move_where");
    };

    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return result_map("move_where"),
    };
    // The Python command resolves the authoritative room JSON exit list at
    // command time.  Room.exits is a runtime cache and can legitimately lag
    // after hot-reload/admin edits, so a cache-shape mismatch must not turn a
    // valid static direction into a silent no-op.  `selected` above already
    // came from the Python-compatible source list.
    let destination_arc = match world
        .room_cache
        .get_room(&destination_zone, &destination_room)
    {
        Ok(room) => room,
        Err(_) => return result_map("move_where"),
    };
    let Some(destination_info) = room_info(&destination_zone, &destination_room) else {
        return result_map("move_where");
    };

    // Player.enterRoom checks isMovable only after getExit/getRoom succeeds.
    if matches!(body.act, ActState::Fight | ActState::Rest) {
        return result_map("not_movable");
    }

    let destination_properties = room_properties(&destination_info);
    let level = body.get_int("레벨");
    let level_upper = effective_room_int(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "레벨상한",
    );
    let level_lower = effective_room_int(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "레벨제한",
    );
    let strength_upper = effective_room_int(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "힘상한제한",
    );
    let dexterity_upper = effective_room_int(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "민첩상한제한",
    );
    if (level_upper > 0 && level_upper < level)
        || level_lower > level
        || (strength_upper > 0 && strength_upper < body.get_int("힘"))
        || (dexterity_upper > 0 && dexterity_upper < body.get_dex())
    {
        return result_map("pressure");
    }

    let player_limit = property_limit(&destination_properties, "인원제한");
    if player_limit > 0
        && world
            .get_players_in_room(&destination_zone, &destination_room)
            .len() as i64
            >= player_limit
    {
        return result_map("room_full");
    }
    let personality = body.get_string("성격");
    if destination_properties
        .iter()
        .any(|property| property == "사파출입금지")
        && personality == "사파"
    {
        return result_map("evil_forbidden");
    }
    if destination_properties
        .iter()
        .any(|property| property == "정파출입금지")
        && personality == "정파"
    {
        return result_map("good_forbidden");
    }
    let guild_owner = effective_room_string(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "방파주인",
    );
    if !guild_owner.is_empty() && guild_owner != body.get_string("소속") {
        return result_map("guild_forbidden");
    }

    let exit_script = effective_room_string(
        &world,
        &current_zone,
        &current_room,
        &current_info,
        &format!("이동스크립:{}", selected.name),
    );
    let entry_script = effective_room_string(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        &format!("진입스크립:{}", selected.name),
    );
    let room_auto_move = effective_room_string(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "자동이동",
    );
    // Floor objects never prevent Python's enterRoom().  Individually
    // represented objects are expired by this Room.update; legacy compressed
    // stacks have no trustworthy per-object timestamp, so leave them intact
    // instead of turning their presence into an invented movement failure.

    let hazard = first_hazard(&destination_properties);
    let (hazard_damage, hazard_message) = hazard.clone().unwrap_or((0, String::new()));
    world.spawn_mobs_for_room(&destination_zone, &destination_room);
    let now_millis = chrono::Utc::now().timestamp_millis();
    let room_update_due = destination_arc
        .read()
        .is_ok_and(|room| now_millis.saturating_sub(room.last_update_millis) >= 1_000);
    let room_update_players = world.get_players_in_room(&destination_zone, &destination_room);
    let room_expired_items = if room_update_due {
        world.expire_floor_items_at(
            &[(destination_zone.clone(), destination_room.clone())],
            now_millis as f64 / 1_000.0,
        )
    } else {
        Vec::new()
    };
    let room_update_messages = if room_update_due {
        world.update_occupied_room_mobs(&[(destination_zone.clone(), destination_room.clone())])
    } else {
        Vec::new()
    };
    let mut mob_metadata = world
        .mob_cache
        .get_all_mobs_in_room(&destination_zone, &destination_room)
        .into_iter()
        .filter_map(|mob| {
            world
                .mob_cache
                .get_mob(&mob.mob_key)
                .cloned()
                .map(|data| (mob.clone(), data))
        })
        .collect::<Vec<_>>();
    // Python iterates Room.objs in the room JSON's mob insertion order.
    // Runtime cache traversal is not a stable substitute (the two visible
    // mobs in room 19 otherwise swap places in the rendered description).
    let source_mob_order = json_strings(destination_info.get("몹"));
    mob_metadata.sort_by_key(|(mob, _)| {
        source_mob_order
            .iter()
            .position(|name| mob.mob_key.rsplit(':').next() == Some(name.as_str()))
            .map(|position| source_mob_order.len().saturating_sub(position + 1))
            .unwrap_or(usize::MAX)
    });
    // Multiple mobs are a normal room state. The cache's room-local vector
    // supplies deterministic insertion order for the view; do not suppress
    // movement merely because the destination contains more than one mob.
    let mut has_entry_events = false;
    for (_mob, data) in &mob_metadata {
        if data.events.contains_key(ENTRY_EVENT_KEY) {
            has_entry_events = true;
        }
    }

    if room_update_due {
        if let Ok(mut room) = destination_arc.write() {
            room.last_update_millis = now_millis;
        }
    }

    world.set_player_position(
        &player_name,
        PlayerPosition::new(destination_zone.clone(), destination_room.clone()),
    );
    drop(world);
    let position = format!("{destination_zone}:{destination_room}");
    body.set("위치", position.as_str());
    body.set("현재방", position.as_str());
    body.temp_mut().insert(
        "_movement_completed_move".to_string(),
        crate::object::Value::String(selected.name.clone()),
    );

    let follower_names = body
        .temp()
        .get("_movement_follower_names")
        .and_then(crate::object::Value::as_str)
        .map(|names| {
            names
                .split('\n')
                .filter(|name| !name.is_empty())
                .map(|name| Dynamic::from(name.to_string()))
                .collect::<Array>()
        })
        .unwrap_or_default();

    let mut result = result_map("ok");
    result.insert("move".into(), Dynamic::from(selected.name));
    result.insert("hidden".into(), Dynamic::from(selected.hidden));
    result.insert("exit_script".into(), Dynamic::from(exit_script));
    result.insert("entry_script".into(), Dynamic::from(entry_script));
    result.insert("has_hazard".into(), Dynamic::from(hazard.is_some()));
    result.insert("hazard_damage".into(), Dynamic::from(hazard_damage));
    result.insert("hazard_message".into(), Dynamic::from(hazard_message));
    result.insert("has_entry_events".into(), Dynamic::from(has_entry_events));
    result.insert("auto_move".into(), Dynamic::from(room_auto_move));
    result.insert("followers".into(), Dynamic::from(follower_names));
    result.insert(
        "room_update_players".into(),
        Dynamic::from(
            room_update_players
                .into_iter()
                .map(Dynamic::from)
                .collect::<Array>(),
        ),
    );
    result.insert(
        "room_expired_items".into(),
        Dynamic::from(
            room_expired_items
                .into_iter()
                .map(|item| Dynamic::from(item.name))
                .collect::<Array>(),
        ),
    );
    result.insert(
        "room_update_messages".into(),
        Dynamic::from(
            room_update_messages
                .iter()
                .filter(|message| message.kind == crate::world::RoomMobMessageKind::Speech)
                .map(|message| Dynamic::from(message.message.clone()))
                .collect::<Array>(),
        ),
    );
    result.insert(
        "room_corpse_updates".into(),
        Dynamic::from(
            room_update_messages
                .into_iter()
                .filter(|message| message.kind == crate::world::RoomMobMessageKind::CorpseGone)
                .map(|message| {
                    let mut value = Map::new();
                    value.insert("mob".into(), Dynamic::from(message.mob_name));
                    value.insert(
                        "items".into(),
                        Dynamic::from(
                            message
                                .revealed_items
                                .into_iter()
                                .map(|item| {
                                    let mut data = Map::new();
                                    data.insert("name".into(), Dynamic::from(item.name));
                                    data.insert("ansi".into(), Dynamic::from(item.ansi));
                                    Dynamic::from(data)
                                })
                                .collect::<Array>(),
                        ),
                    );
                    Dynamic::from(value)
                })
                .collect::<Array>(),
        ),
    );
    result
}

fn move_through_exit(body: &mut Body, command: &str) -> Map {
    let mut rng = rand::thread_rng();
    move_through_exit_with_roller(body, command, &mut |length| rng.gen_range(0..length))
}

/// Candidate destination rooms for one-word movement snapshotting.  This is
/// bounded to exits of the current room and never scans online players.
pub(crate) fn immediate_exit_destinations(zone: &str, room: &str) -> Vec<(String, String)> {
    let Some(info) = room_info(zone, room) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut destinations = Vec::new();
    for exit in accessible_exits(&info) {
        for destination in exit.destinations {
            let Some(resolved) = resolve_exit_destination(zone, &info, &destination) else {
                continue;
            };
            if seen.insert(resolved.clone()) {
                destinations.push(resolved);
            }
        }
    }
    destinations
}

/// Reproduce `cmds/맵.py`'s recursive explorer, including its branch-size
/// ordering and explicit reverse-direction markers.  The command script only
/// formats the returned direction list.
pub(crate) fn python_map_explore(body: &Body, excluded: &str) -> Array {
    let mut rng = rand::thread_rng();
    python_map_explore_with_roller(body, excluded, &mut |length| rng.gen_range(0..length))
}

pub(crate) fn python_map_explore_with_roller(
    body: &Body,
    excluded: &str,
    roller: &mut impl FnMut(usize) -> usize,
) -> Array {
    let Some((zone, start)) = current_body_position(body) else {
        return Array::new();
    };
    let compass = ["동", "서", "남", "북", "북서", "북동", "남서", "남동"];
    let mut graph: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut pending = vec![start.clone()];
    let mut loaded = HashSet::new();
    while let Some(room) = pending.pop() {
        if !loaded.insert(room.clone()) {
            continue;
        }
        let Some(info) = room_info(&zone, &room) else {
            continue;
        };
        let mut edges = Vec::new();
        for exit in accessible_exits(&info) {
            // Python explorer iterates Room.exitList, where hidden directions
            // still carry their trailing `$` and therefore never equal one of
            // the eight accepted compass names.
            if exit.hidden {
                continue;
            }
            if !compass.contains(&exit.name.as_str()) {
                continue;
            }
            let Some(destination) = exit
                .destinations
                .get(roller(exit.destinations.len()).min(exit.destinations.len() - 1))
            else {
                continue;
            };
            let Some((dest_zone, dest_room)) = resolve_exit_destination(&zone, &info, destination)
            else {
                continue;
            };
            if dest_zone != zone {
                continue;
            }
            edges.push((exit.name, dest_room.clone()));
            if !loaded.contains(&dest_room) {
                pending.push(dest_room);
            }
        }
        graph.insert(room, edges);
    }

    fn reverse(direction: &str) -> &'static str {
        match direction {
            "동" => "서",
            "서" => "동",
            "남" => "북",
            "북" => "남",
            "북서" => "남동",
            "남동" => "북서",
            "북동" => "남서",
            "남서" => "북동",
            _ => "",
        }
    }
    fn count_explorer(
        graph: &HashMap<String, Vec<(String, String)>>,
        room: &str,
        direction: &str,
        temp: &mut HashMap<String, Vec<String>>,
        budget: &mut usize,
    ) -> i64 {
        if *budget == 0 {
            return 0;
        }
        *budget -= 1;
        let Some(edges) = graph.get(room) else {
            return 0;
        };
        let rev = reverse(direction);
        if rev.is_empty() {
            return 0;
        }
        if temp.contains_key(room) {
            return 0;
        }
        let mut remaining = edges.iter().map(|(d, _)| d.clone()).collect::<Vec<_>>();
        temp.insert(room.to_string(), remaining.clone());
        remaining.retain(|d| d != rev);
        if remaining.is_empty() {
            return 1;
        }
        let mut count = 0;
        for d in remaining {
            if let Some((_, next)) = edges.iter().find(|(name, _)| name == &d) {
                count += count_explorer(graph, next, &d, temp, budget);
            }
        }
        temp.insert(room.to_string(), Vec::new());
        count
    }
    fn explore(
        graph: &HashMap<String, Vec<(String, String)>>,
        room: &str,
        direction: &str,
        mapq: &mut HashMap<String, Vec<String>>,
        walk: &mut Vec<String>,
        budget: &mut usize,
    ) {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let rev = reverse(direction);
        if rev.is_empty() {
            return;
        }
        let existed = mapq.contains_key(room);
        let edges = if let Some(existing) = mapq.get(room) {
            let mut copy = existing.clone();
            copy.retain(|d| d != rev);
            copy
        } else {
            let mut copy: Vec<String> = graph
                .get(room)
                .map(|e| e.iter().map(|(d, _)| d.clone()).collect())
                .unwrap_or_default();
            copy.retain(|d| d != rev);
            mapq.insert(room.to_string(), copy.clone());
            copy
        };
        if existed {
            return;
        }
        walk.push(direction.to_string());
        let mut dirs: Vec<(String, i64)> = Vec::new();
        for d in &edges {
            if let Some((_, next)) = graph
                .get(room)
                .and_then(|e| e.iter().find(|(name, _)| name == d))
            {
                let mut temp = HashMap::new();
                dirs.push((d.clone(), count_explorer(graph, next, d, &mut temp, budget)));
            }
        }
        dirs.sort_by_key(|(_, n)| *n);
        while let Some((d, _)) = dirs.pop() {
            if let Some((_, next)) = graph
                .get(room)
                .and_then(|e| e.iter().find(|(name, _)| name == &d))
            {
                explore(graph, next, &d, mapq, walk, budget);
            }
        }
        mapq.insert(room.to_string(), Vec::new());
        if mapq.values().any(|v| !v.is_empty()) {
            walk.push(rev.to_string());
        }
    }
    let Some(first) = graph.get(&start).and_then(|e| {
        e.iter()
            .find(|(d, _)| d != excluded && compass.contains(&d.as_str()))
    }) else {
        return Array::new();
    };
    let mut mapq = HashMap::new();
    // cmds/맵.py initializes ob.mapQ[current_room] to [], not to the
    // current room's remaining exits.  Seeding the exits adds a spurious
    // reverse-direction marker at the end of an otherwise terminal branch.
    mapq.insert(start.clone(), Vec::new());
    let mut walk = Vec::new();
    // Python's implementation is recursive too, but a malformed/cyclic map
    // must not monopolize the Rust command worker.  This is ample for the
    // normal zone traversal while preserving a bounded response time.
    let mut budget = 5_000usize;
    explore(
        &graph,
        &first.1,
        &first.0,
        &mut mapq,
        &mut walk,
        &mut budget,
    );
    walk.into_iter().map(Dynamic::from).collect()
}

fn room_view_data(body: &Body) -> Map {
    let mut result = Map::new();
    result.insert("ok".into(), Dynamic::from(false));
    result.insert("zone".into(), Dynamic::from(String::new()));
    result.insert("room".into(), Dynamic::from(String::new()));
    result.insert("name".into(), Dynamic::from(String::new()));
    result.insert("description".into(), Dynamic::from(Array::new()));
    result.insert("exits".into(), Dynamic::from(Array::new()));
    result.insert("boxes".into(), Dynamic::from(Array::new()));
    result.insert("mobs".into(), Dynamic::from(Array::new()));
    result.insert("players".into(), Dynamic::from(Array::new()));

    let Some((zone, room)) = current_body_position(body) else {
        return result;
    };
    let Some(info) = room_info(&zone, &room) else {
        return result;
    };
    let world = match get_world_state().read() {
        Ok(world) => world,
        Err(_) => return result,
    };
    let room_arc = match world.room_cache.get_room_cached(&zone, &room) {
        Some(room) => room,
        None => return result,
    };
    if room_arc.read().is_err() {
        return result;
    }

    let descriptions = json_strings(info.get("설명"))
        .into_iter()
        .map(Dynamic::from)
        .collect::<Array>();
    let exits = sorted_exit_names(&info)
        .into_iter()
        .filter(|name| !name.ends_with('$'))
        .map(Dynamic::from)
        .collect::<Array>();
    let source_mob_order = json_strings(info.get("몹"));
    let unified_order = world.get_room_object_order(&zone, &room);
    let mut room_mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
    room_mobs.sort_by_key(|mob| {
        unified_order
            .iter()
            .position(|object| *object == crate::world::RoomObjectRef::Mob(mob.instance_id))
            .unwrap_or_else(|| {
                source_mob_order
                    .iter()
                    .position(|name| mob.mob_key.rsplit(':').next() == Some(name.as_str()))
                    .map(|position| source_mob_order.len().saturating_sub(position + 1))
                    .unwrap_or(usize::MAX)
            })
    });
    let mut mobs = Array::new();
    for mob in room_mobs {
        let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
            continue;
        };
        let mut value = Map::new();
        value.insert("name".into(), Dynamic::from(mob.name.clone()));
        value.insert("desc1".into(), Dynamic::from(data.desc1.clone()));
        value.insert("act".into(), Dynamic::from(mob.act as i64));
        value.insert("mob_type".into(), Dynamic::from(data.mob_type));
        value.insert(
            "defense_heads".into(),
            Dynamic::from(
                mob.skills
                    .iter()
                    .filter_map(|skill| {
                        let head = crate::data::get_skill_defense_head(skill);
                        (!head.is_empty()).then_some(Dynamic::from(head))
                    })
                    .collect::<Array>(),
            ),
        );
        mobs.push(Dynamic::from(value));
    }

    result.insert("ok".into(), Dynamic::from(true));
    result.insert("zone".into(), Dynamic::from(zone.clone()));
    result.insert("room".into(), Dynamic::from(room.clone()));
    result.insert(
        "name".into(),
        Dynamic::from(effective_room_string(&world, &zone, &room, &info, "이름")),
    );
    result.insert("description".into(), Dynamic::from(descriptions));
    result.insert("exits".into(), Dynamic::from(exits));
    result.insert(
        "boxes".into(),
        Dynamic::from(
            crate::script::installed_box_short_views(&zone, &room)
                .into_iter()
                .map(Dynamic::from)
                .collect::<Array>(),
        ),
    );
    result.insert("mobs".into(), Dynamic::from(mobs));
    let mut players = room_view_player_snapshots(&zone, &room);
    // WorldState appends arrivals; Python Room.insert places each new Player
    // at index 0. Reversing the player-only index recreates their relative
    // viewMapData order (objects of other types are rendered in separate loops).
    players.reverse();
    result.insert("players".into(), Dynamic::from(players));
    result
}

/// Return the current room's legacy `#오브젝트:<command>` text.
///
/// Converted JSON stores these entries under `맵정보.오브젝트`, while
/// Python's command loop exposed them as direct room commands after ordinary
/// commands and emotions had failed to match.
fn room_object_lines(body: &Body, command: &str) -> Array {
    let Some((zone, room)) = current_body_position(body) else {
        return Array::new();
    };
    let Some(info) = room_info(&zone, &room) else {
        return Array::new();
    };
    let Some(value) = info
        .get("오브젝트")
        .and_then(JsonValue::as_object)
        .and_then(|objects| objects.get(command))
    else {
        return Array::new();
    };
    json_strings(Some(value))
        .into_iter()
        .map(Dynamic::from)
        .collect()
}

fn apply_movement_hazard(body: &mut Body, damage: i64) -> bool {
    if body.minus_hp(damage) {
        body.act = ActState::Death;
        body.unwear_all();
        body.clear_targets_death();
        body.clear_skills();
        body.set_death_step(0);
        return false;
    }
    true
}

fn python_map_mark(
    zone: &str,
    room: &str,
    index: i32,
    grid: &mut [String],
    visited: &mut HashSet<(String, String, i32)>,
) {
    if !(0..132).contains(&index) || !visited.insert((zone.to_string(), room.to_string(), index)) {
        return;
    }
    let Some(info) = room_info(zone, room) else {
        return;
    };
    let room_numbers = [
        12, 14, 16, 18, 20, 34, 36, 38, 40, 42, 56, 58, 60, 62, 64, 78, 80, 82, 84, 86, 100, 102,
        104, 106, 108,
    ];
    if !room_numbers.contains(&index) && index != 60 {
        return;
    }
    if grid[index as usize] != "  " {
        return;
    }
    grid[index as usize] = if index == 60 {
        "\x1b[1;33m○\x1b[37;0m".into()
    } else {
        "○".into()
    };
    for exit in accessible_exits(&info) {
        let (next, connector, recurse) = match exit.name.as_str() {
            "동" => (index + 1, '→', index + 2),
            "서" => (index - 1, '←', index - 2),
            "남" => (index + 11, '↓', index + 22),
            "북" => (index - 11, '↑', index - 22),
            "북동" => (index - 10, '↗', index - 20),
            "북서" => (index - 12, '↖', index - 24),
            "남동" => (index + 12, '↘', index + 24),
            "남서" => (index + 10, '↙', index + 20),
            "위" => {
                let both = grid[index as usize].contains('∨');
                grid[index as usize] = if index == 60 {
                    if both {
                        "\x1b[1;33m↕\x1b[37;0m".into()
                    } else {
                        "\x1b[1;33m∧\x1b[37;0m".into()
                    }
                } else if both {
                    "↕".into()
                } else {
                    "∧".into()
                };
                continue;
            }
            "아래" | "밑" => {
                let both = grid[index as usize].contains('∧');
                grid[index as usize] = if index == 60 {
                    if both {
                        "\x1b[1;33m↕\x1b[37;0m".into()
                    } else {
                        "\x1b[1;33m∨\x1b[37;0m".into()
                    }
                } else if both {
                    "↕".into()
                } else {
                    "∨".into()
                };
                continue;
            }
            _ => continue,
        };
        if !(0..132).contains(&next) {
            continue;
        }
        grid[next as usize] = if grid[next as usize] == "  " {
            connector.to_string()
        } else {
            match connector {
                '←' | '→' => "─",
                '↑' | '↓' => "│",
                '↗' | '↙' => "／",
                '↖' | '↘' => "＼",
                _ => "  ",
            }
            .into()
        };
        if let Some(destination) = exit
            .destinations
            .first()
            .and_then(|d| resolve_exit_destination(zone, &info, d))
        {
            python_map_mark(&destination.0, &destination.1, recurse, grid, visited);
        }
    }
}

pub(crate) fn python_map_text(zone: &str, room: &str) -> String {
    let Some(info) = room_info(zone, room) else {
        return String::new();
    };
    // Python 지도.cmd는 재귀 표식을 시작하기 전에 exitList에서 `$`로
    // 끝나는 숨김 출구를 제외해 하나라도 남는지 검사한다.
    let has_visible_exit = json_strings(info.get("출구")).into_iter().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|name| !name.ends_with('$'))
    });
    if !has_visible_exit {
        return String::new();
    }
    let mut grid = vec!["  ".to_string(); 132];
    let mut visited = HashSet::new();
    python_map_mark(zone, room, 60, &mut grid, &mut visited);
    grid.chunks(11)
        .map(|row| row.concat())
        .collect::<Vec<_>>()
        .join("\r\n")
}

pub(super) fn register_movement_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    engine.register_fn("movement_internal_args", |line: &str| -> Array {
        line.split_whitespace()
            .map(|word| Dynamic::from(word.to_string()))
            .collect()
    });
    let ptr = body_ptr;
    engine.register_fn(
        "take_after_fight_route_finished",
        move |_ob: &mut Map| -> bool { take_after_fight_route_finished(unsafe { &mut *ptr }) },
    );
    let ptr = body_ptr;
    engine.register_fn(
        "movement_command_limited",
        move |_ob: &mut Map, raw_command: &str| -> bool {
            command_limited(unsafe { &*ptr }, raw_command)
        },
    );
    let ptr = body_ptr;
    engine.register_fn(
        "move_through_exit",
        move |_ob: &mut Map, command: &str| -> Map {
            move_through_exit(unsafe { &mut *ptr }, command)
        },
    );
    let ptr = body_ptr;
    engine.register_fn("movement_room_view", move |_ob: &mut Map| -> Map {
        room_view_data(unsafe { &*ptr })
    });
    let ptr = body_ptr;
    engine.register_fn(
        "get_room_object_lines",
        move |_ob: &mut Map, command: &str| -> Array {
            room_object_lines(unsafe { &*ptr }, command)
        },
    );
    engine.register_fn("get_python_map", |zone: &str, room: &str| -> String {
        python_map_text(zone, room)
    });
    let ptr = body_ptr;
    engine.register_fn(
        "apply_movement_hazard",
        move |_ob: &mut Map, damage: i64| -> bool {
            apply_movement_hazard(unsafe { &mut *ptr }, damage)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn after_fight_route_completion_is_consumed_once_by_rhai() {
        let mut body = Body::new();
        body.temp_mut()
            .insert(AFTER_FIGHT_ROUTE_FINISHED.to_string(), Value::Int(1));
        assert!(take_after_fight_route_finished(&mut body));
        assert!(!take_after_fight_route_finished(&mut body));
    }

    fn map_info(exits: &[&str]) -> JsonMap<String, JsonValue> {
        let mut info = JsonMap::new();
        info.insert(
            "출구".to_string(),
            JsonValue::Array(
                exits
                    .iter()
                    .map(|exit| JsonValue::String((*exit).to_string()))
                    .collect(),
            ),
        );
        info
    }

    fn status(result: &Map) -> String {
        result
            .get("status")
            .and_then(|value| value.clone().into_string().ok())
            .unwrap_or_default()
    }

    #[test]
    fn ten_directions_follow_python_exit_list_order() {
        let info = map_info(&[
            "문 99",
            "북서 10",
            "북동 11",
            "남서 12",
            "남동 13",
            "아래 6",
            "위 5",
            "북 4",
            "남 3",
            "서 2",
            "동 1",
        ]);
        assert_eq!(
            sorted_exit_names(&info),
            vec![
                "동", "서", "남", "북", "위", "아래", "남동", "남서", "북동", "북서", "문"
            ]
        );
    }

    #[test]
    fn hidden_exits_move_to_python_exits_tail_and_strip_dollar() {
        let info = map_info(&["비밀$ 2", "북 1", "문 3"]);
        let exits = accessible_exits(&info);
        assert_eq!(
            exits
                .iter()
                .map(|exit| exit.name.as_str())
                .collect::<Vec<_>>(),
            vec!["북", "문", "비밀"]
        );
        assert!(exits[2].hidden);
        assert_eq!(sorted_exit_names(&info), vec!["북", "비밀$", "문"]);
    }

    #[test]
    fn prefix_resolution_uses_first_python_exits_entry() {
        let info = map_info(&["문밖 1", "문안 2"]);
        let exits = accessible_exits(&info);
        assert_eq!(
            exits
                .iter()
                .find(|exit| exit.name.starts_with("문"))
                .unwrap()
                .name,
            "문밖"
        );
    }

    #[test]
    fn multi_destination_roll_is_inclusive_index_domain() {
        let info = map_info(&["문 1 2 3"]);
        let exit = accessible_exits(&info).remove(0);
        assert!(exit.random_destination);
        assert_eq!(exit.destinations, vec!["1", "2", "3"]);
    }

    #[test]
    fn zero_and_negative_hazards_keep_python_minus_hp_behavior() {
        let mut body = Body::new();
        body.set("체력", 100);
        assert!(apply_movement_hazard(&mut body, 0));
        assert_eq!(body.get_hp(), 100);
        assert!(apply_movement_hazard(&mut body, -7));
        assert_eq!(body.get_hp(), 107);
    }

    #[test]
    fn normal_direction_moves_and_view_data_uses_destination() {
        let player_name = "일반이동상태검사";
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "5").unwrap();
            world.room_cache.get_room("산동성", "6").unwrap();
            world.set_player_position(
                player_name,
                PlayerPosition::new("산동성".to_string(), "5".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name);
        body.set("레벨", 1_i64);
        body.set("체력", 100_i64);
        body.set("최고체력", 100_i64);
        body.set("위치", "산동성:5");
        body.set("현재방", "산동성:5");

        let result = move_through_exit_with_roller(&mut body, "동", &mut |_| 0);
        assert_eq!(status(&result), "ok");
        assert_eq!(body.get_string("위치"), "산동성:6");
        let view = room_view_data(&body);
        assert!(view
            .get("ok")
            .and_then(|value| value.as_bool().ok())
            .unwrap_or(false));
        assert_eq!(
            view.get("room")
                .and_then(|value| value.clone().into_string().ok())
                .unwrap_or_default(),
            "6"
        );

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(player_name);
    }

    #[test]
    fn starting_room_description_preserves_python_map_line_breaks() {
        get_world_state()
            .write()
            .unwrap()
            .room_cache
            .get_room("낙양성", "42")
            .unwrap();
        let mut body = Body::new();
        body.set("이름", "시작방설명검사");
        body.set("위치", "낙양성:42");

        let view = room_view_data(&body);
        let description = view["description"].clone().try_cast::<Array>().unwrap();
        let lines = description
            .into_iter()
            .filter_map(|line| line.into_string().ok())
            .collect::<Vec<_>>();

        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines[0],
            "산들바람에 여유롭게 흔들리는 꽃들은 주위배경과 절묘하게 조"
        );
        assert_eq!(
            lines[3],
            "『\u{1b}[36;1m안내문\u{1b}[0m\u{1b}[37m\u{1b}[40m』이 붙어 있다."
        );
    }

    #[test]
    fn starting_room_notice_is_exposed_as_legacy_direct_room_command() {
        let mut body = Body::new();
        body.set("위치", "낙양성:42");

        let lines = room_object_lines(&body, "안내문")
            .into_iter()
            .filter_map(|line| line.into_string().ok())
            .collect::<Vec<_>>();

        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("무림크래프트를 처음 하시는분은"));
        assert!(lines[2].contains("\u{1b}[32m초보수련장"));
        assert!(room_object_lines(&body, "없는오브젝트").is_empty());
    }

    #[test]
    fn room_view_keeps_dead_and_living_same_name_mobs_visible_together() {
        let player_name = format!("시체방보기-{}", std::process::id());
        let dead_id;
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("낙양성", "33").unwrap();
            world.spawn_mobs_for_room("낙양성", "33");
            dead_id = world
                .get_room_object_order("낙양성", "33")
                .into_iter()
                .find_map(|object| match object {
                    crate::world::RoomObjectRef::Mob(id) => Some(id),
                    _ => None,
                })
                .unwrap();
            let mobs = world
                .mob_cache
                .get_all_mobs_in_room_mut("낙양성", "33")
                .unwrap();
            let mob = mobs
                .iter_mut()
                .find(|mob| mob.instance_id == dead_id)
                .unwrap();
            mob.alive = false;
            mob.act = 2;
            mob.hp = 0;
            mob.death_time = chrono::Utc::now().timestamp();
            world.set_player_position(
                &player_name,
                PlayerPosition::new("낙양성".to_string(), "33".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("위치", "낙양성:33");
        let view = room_view_data(&body);
        let mobs = view["mobs"].clone().try_cast::<Array>().unwrap();
        let acts = mobs
            .into_iter()
            .filter_map(|mob| mob.try_cast::<Map>())
            .filter(|mob| mob["name"].clone().into_string().ok().as_deref() == Some("생쥐"))
            .filter_map(|mob| mob["act"].as_int().ok())
            .collect::<Vec<_>>();
        assert!(acts.contains(&0));
        assert!(acts.contains(&2));
        assert_eq!(acts.first(), Some(&2));
        let command_view = crate::script::build_room_lines(&player_name, &[]).unwrap();
        assert!(command_view.contains("생쥐의 싸늘한 시체가 있습니다."));
        assert!(command_view.contains("기웃 거립니다."));

        let mut world = get_world_state().write().unwrap();
        let data_key = world
            .mob_cache
            .get_all_mobs_in_room("낙양성", "33")
            .into_iter()
            .find(|mob| mob.instance_id == dead_id)
            .unwrap()
            .mob_key
            .clone();
        let data = world.mob_cache.get_mob(&data_key).unwrap().clone();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room_mut("낙양성", "33")
            .unwrap()
            .iter_mut()
            .find(|mob| mob.instance_id == dead_id)
            .unwrap();
        mob.respawn(&data);
        world.remove_player_position(&player_name);
    }

    #[test]
    fn entering_auto_move_room_returns_python_delayed_command() {
        let player_name = "자동이동방검사";
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("귀주성", "170").unwrap();
            world.room_cache.get_room("귀주성", "171").unwrap();
            world.set_player_position(
                player_name,
                PlayerPosition::new("귀주성".to_string(), "170".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name);
        body.set("레벨", 1_i64);
        body.set("체력", 100_i64);
        body.set("최고체력", 100_i64);
        body.set("위치", "귀주성:170");
        body.set("현재방", "귀주성:170");

        let result = move_through_exit_with_roller(&mut body, "동", &mut |_| 0);
        assert_eq!(status(&result), "ok");
        assert_eq!(body.get_string("위치"), "귀주성:171");
        assert_eq!(
            result
                .get("auto_move")
                .and_then(|value| value.clone().into_string().ok())
                .as_deref(),
            Some("동 12")
        );

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(player_name);
    }

    #[test]
    fn missing_canonical_direction_does_not_mutate_position() {
        let player_name = "없는방향상태검사";
        get_world_state().write().unwrap().set_player_position(
            player_name,
            PlayerPosition::new("산동성".to_string(), "5".to_string()),
        );
        let mut body = Body::new();
        body.set("이름", player_name);
        body.set("위치", "산동성:5");

        let result = move_through_exit_with_roller(&mut body, "북", &mut |_| 0);
        assert_eq!(status(&result), "direction_missing");
        let mut world = get_world_state().write().unwrap();
        let position = world.get_player_position(player_name).unwrap();
        assert_eq!(
            (position.zone.as_str(), position.room.as_str()),
            ("산동성", "5")
        );
        world.remove_player_position(player_name);
    }

    #[test]
    fn royal_tomb_hidden_entrance_and_random_corridors_match_map_data() {
        let entrance = room_info("낙양성", "150").expect("royal tomb entrance fixture");
        let exits = accessible_exits(&entrance);
        let down = exits
            .iter()
            .find(|exit| exit.name == "아래")
            .expect("hidden downward entrance");
        assert!(down.hidden);
        assert_eq!(down.destinations, vec!["151"]);
        let displayed = sorted_exit_names(&entrance)
            .into_iter()
            .filter(|name| !name.ends_with('$'))
            .collect::<Vec<_>>();
        assert!(!displayed.iter().any(|name| name == "아래"));

        for (room, direction, destinations) in [
            ("154", "남", vec!["153", "155"]),
            ("155", "북", vec!["153", "154"]),
        ] {
            let info = room_info("낙양성", room).expect("random royal tomb corridor fixture");
            let exit = accessible_exits(&info)
                .into_iter()
                .find(|exit| exit.name == direction)
                .expect("random corridor exit");
            assert!(exit.random_destination);
            assert_eq!(exit.destinations, destinations);
        }
    }

    #[test]
    fn muguk_cave_hidden_slide_randomly_reaches_three_circuits() {
        for (room, destination) in [("6002", "6003"), ("6003", "6004"), ("6004", "6005")] {
            let info = room_info("낙양성", room).expect("Muguk cave slide fixture");
            let down = accessible_exits(&info)
                .into_iter()
                .find(|exit| exit.name == "아래")
                .expect("hidden downward slide");
            assert!(down.hidden);
            assert_eq!(down.destinations, vec![destination]);
        }
        let info = room_info("낙양성", "6005").expect("three-way slide fixture");
        let down = accessible_exits(&info)
            .into_iter()
            .find(|exit| exit.name == "아래")
            .expect("random hidden downward slide");
        assert!(down.hidden);
        assert!(down.random_destination);
        assert_eq!(down.destinations, vec!["6014", "6023", "6032"]);
    }

    #[test]
    fn room_command_limit_uses_python_substring_membership() {
        let player_name = "명령제한상태검사";
        get_world_state().write().unwrap().set_player_position(
            player_name,
            PlayerPosition::new("백층탑".to_string(), "224".to_string()),
        );
        let mut body = Body::new();
        body.set("이름", player_name);
        body.set("위치", "백층탑:224");
        assert!(command_limited(&body, "주"));
        assert!(command_limited(&body, "버려"));
        assert!(!command_limited(&body, "동"));
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(player_name);
    }
}
