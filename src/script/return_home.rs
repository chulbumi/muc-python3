//! Data/state efun for Python `cmds/귀환.py`.
//!
//! User-visible text deliberately stays in `cmds/귀환.rhai`. This module only
//! validates the same room/body state as Python and commits the position move.

use crate::command::CommandResult;
use crate::player::{ActState, Body};
use crate::world::event::do_event;
use crate::world::{get_world_state, EventScript, PlayerPosition, RawMobData};
use rhai::{Array, Dynamic, Engine, Map};
use serde_json::{Map as JsonMap, Value as JsonValue};

const ENTRY_EVENT_KEY: &str = "이벤트 $%입장이벤트%";

fn result_map(status: &str) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from(status.to_string()));
    result.insert("exit_script".into(), Dynamic::from(String::new()));
    result.insert("entry_script".into(), Dynamic::from(String::new()));
    result.insert("has_hazard".into(), Dynamic::from(false));
    result.insert("hazard_damage".into(), Dynamic::from(0_i64));
    result.insert("hazard_message".into(), Dynamic::from(String::new()));
    result.insert("has_entry_events".into(), Dynamic::from(false));
    result
}

pub(super) fn room_info(zone: &str, room: &str) -> Option<JsonMap<String, JsonValue>> {
    let path = std::path::Path::new("data/map")
        .join(zone)
        .join(format!("{room}.json"));
    let source = std::fs::read_to_string(path).ok()?;
    let root: JsonValue = serde_json::from_str(&source).ok()?;
    root.get("맵정보")?.as_object().cloned()
}

pub(super) fn python_int(value: Option<&JsonValue>) -> i64 {
    let Some(value) = value else {
        return 0;
    };
    if let Some(value) = value.as_i64() {
        return value;
    }
    let Some(value) = value.as_str() else {
        return 0;
    };
    if let Ok(number) = value.parse::<i64>() {
        return number;
    }
    if !value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        return 0;
    }
    let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

pub(super) fn property_limit(properties: &[String], prefix: &str) -> i64 {
    properties
        .iter()
        .find_map(|property| {
            let rest = property.strip_prefix(prefix)?;
            Some(python_int(Some(&JsonValue::String(
                rest.trim_start().to_string(),
            ))))
        })
        .unwrap_or(0)
}

pub(super) fn json_string(info: &JsonMap<String, JsonValue>, key: &str) -> String {
    info.get(key)
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(super) fn first_hazard(properties: &[String]) -> Option<(i64, String)> {
    properties.iter().find_map(|property| {
        // Python `attr.split(None, 2)` collapses whitespace before the
        // first two fields while preserving the remainder of the message.
        let property = property.trim_start();
        let first_end = property.find(char::is_whitespace)?;
        (&property[..first_end] == "체력감소").then_some(())?;
        let after_name = property[first_end..].trim_start();
        let damage_end = after_name.find(char::is_whitespace)?;
        let damage = python_int(Some(&JsonValue::String(
            after_name[..damage_end].to_string(),
        )));
        let message = after_name[damage_end..].trim_start().to_string();
        Some((damage, message))
    })
}

pub(super) fn entry_event_is_supported(data: &RawMobData) -> bool {
    let Some(script) = data.events.get(ENTRY_EVENT_KEY) else {
        return true;
    };
    match script {
        EventScript::Legacy(lines) => !lines.iter().any(|line| {
            let line = line.trim_start();
            line.starts_with("$엔터$")
                || line.starts_with("$위치이동")
                || line.starts_with("$스크립트호출")
        }),
        EventScript::Rhai(path) => {
            let source_path = std::path::Path::new("data/script").join(&data.zone).join(
                if path.ends_with(".rhai") {
                    path.clone()
                } else {
                    format!("{path}.rhai")
                },
            );
            std::fs::read_to_string(source_path).is_ok_and(|source| {
                !source.contains("wait_enter(") && !source.contains("set_position(")
            })
        }
    }
}

pub(super) fn mob_has_unrepresented_entry_update(mob_key: &str, data: &RawMobData) -> bool {
    if data.mob_type == 6 {
        return true;
    }
    let file_name = mob_key
        .split_once(':')
        .map(|(_, file_name)| file_name)
        .unwrap_or(mob_key);
    let path = std::path::Path::new("data/mob")
        .join(&data.zone)
        .join(format!("{file_name}.json"));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|source| serde_json::from_str::<JsonValue>(&source).ok())
        .and_then(|root| root.get("몹정보").cloned())
        .is_some_and(|info| {
            // A matching talk tick can call say() and makes Room.update print
            // prompts even when the randomly selected speech is empty.  Rust
            // does not retain the Python RNG/talk-tick state yet.
            info.get("대화틱").is_some_and(|value| match value {
                JsonValue::String(value) => !value.is_empty(),
                JsonValue::Null => false,
                _ => true,
            })
        })
}

fn event_result(status: &str, lines: Vec<String>) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from(status.to_string()));
    result.insert(
        "lines".into(),
        Dynamic::from(lines.into_iter().map(Dynamic::from).collect::<Array>()),
    );
    result
}

/// Run Python's destination-room `이벤트 $%입장이벤트%` loop after the room
/// view has been emitted.  Only the result shape which can be applied at this
/// exact point is accepted; interactive/nested/moving event forms are rejected
/// by `return_home` before the player is moved.
fn run_return_entry_events(body: &mut Body) -> Map {
    let player_name = body.get_name();
    let events = {
        let world = match get_world_state().read() {
            Ok(world) => world,
            Err(_) => return event_result("unsupported_entry_event", Vec::new()),
        };
        let Some(position) = world.get_player_position(&player_name) else {
            return event_result("unsupported_entry_event", Vec::new());
        };
        world
            .mob_cache
            .get_all_mobs_in_room(&position.zone, &position.room)
            .into_iter()
            .filter_map(|mob| {
                let data = world.mob_cache.get_mob(&mob.mob_key)?;
                data.events
                    .contains_key(ENTRY_EVENT_KEY)
                    .then(|| (mob.mob_key.clone(), data.clone()))
            })
            .collect::<Vec<_>>()
    };

    let mut lines = Vec::new();
    for (mob_key, data) in events {
        match do_event(body, &data, ENTRY_EVENT_KEY, &[], &mob_key, None, None) {
            CommandResult::MobEvent {
                output_lines,
                set_position: None,
            } => lines.extend(output_lines),
            _ => return event_result("unsupported_entry_event", lines),
        }
    }
    event_result("ok", lines)
}

/// Apply the already-validated non-lethal `체력감소` room attribute.
/// Python calls `minusHP(dmg, False)` here.  Lethal damage is rejected before
/// movement because Rust cannot yet reproduce Python's selective death drops
/// and coma input callback.
fn apply_return_hazard(body: &mut Body, damage: i64) -> bool {
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

fn return_vitals(body: &Body) -> Map {
    let mut result = Map::new();
    result.insert("hp".into(), Dynamic::from(body.get_hp()));
    result.insert("max_hp".into(), Dynamic::from(body.get_max_hp()));
    result.insert("mp".into(), Dynamic::from(body.get_mp()));
    result.insert("max_mp".into(), Dynamic::from(body.get_max_mp()));
    result
}

/// Validate and perform Python `ob.enterRoom(room, '귀환', '귀환')` state work.
///
/// The returned stable codes are interpreted by Rhai, which owns every output
/// string. The command captures old-room recipients with `get_room_players`
/// before calling this function and destination recipients after it succeeds.
fn return_home(body: &mut Body) -> Map {
    let player_name = body.get_name();
    if player_name.is_empty() {
        return result_map("missing_room");
    }

    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return result_map("missing_room"),
    };
    let (current_zone, current_room) = match world.get_player_position(&player_name) {
        Some(position) => (position.zone.clone(), position.room.clone()),
        None => return result_map("missing_room"),
    };

    // Python `귀환.py` checks the current room before `isMovable()`.
    if let Ok(room) = world.room_cache.get_room(&current_zone, &current_room) {
        if room
            .read()
            .is_ok_and(|room| room.properties.iter().any(|value| value == "귀환금지"))
        {
            return result_map("return_forbidden");
        }
    }

    if matches!(body.act, ActState::Fight | ActState::Rest) {
        return result_map("not_movable");
    }

    let configured = body.get_string("귀환지맵");
    let destination = if configured.is_empty() {
        "낙양성:42"
    } else {
        configured.as_str()
    };
    let Some((destination_zone, destination_room)) = destination.split_once(':') else {
        // Python passes a non-empty malformed value directly to `getRoom`,
        // which returns None; it does not silently use the default room.
        return result_map("missing_room");
    };

    let destination_arc = match world
        .room_cache
        .get_room(destination_zone, destination_room)
    {
        Ok(room) => room,
        Err(_) => return result_map("missing_room"),
    };
    if destination_zone == current_zone && destination_room == current_room {
        return result_map("same_room");
    }

    let properties = destination_arc
        .read()
        .map(|room| room.properties.clone())
        .unwrap_or_default();
    let info = room_info(destination_zone, destination_room).unwrap_or_default();

    let level = body.get_int("레벨");
    let level_upper = python_int(info.get("레벨상한"));
    let level_lower = python_int(info.get("레벨제한"));
    let strength_upper = python_int(info.get("힘상한제한"));
    let dexterity_upper = python_int(info.get("민첩상한제한"));
    if (level_upper > 0 && level_upper < level)
        || level_lower > level
        || (strength_upper > 0 && strength_upper < body.get_int("힘"))
        || (dexterity_upper > 0 && dexterity_upper < body.get_dex())
    {
        return result_map("pressure");
    }

    let player_limit = property_limit(&properties, "인원제한");
    let destination_player_count = world
        .get_players_in_room(destination_zone, destination_room)
        .len();
    if player_limit > 0 && destination_player_count as i64 >= player_limit {
        return result_map("room_full");
    }

    let personality = body.get_string("성격");
    if properties.iter().any(|value| value == "사파출입금지") && personality == "사파" {
        return result_map("evil_forbidden");
    }
    if properties.iter().any(|value| value == "정파출입금지") && personality == "정파" {
        return result_map("good_forbidden");
    }

    let guild_owner = info
        .get("방파주인")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    if !guild_owner.is_empty() && guild_owner != body.get_string("소속") {
        return result_map("guild_forbidden");
    }

    let current_info = room_info(&current_zone, &current_room).unwrap_or_default();
    let current_key = format!("{current_zone}:{current_room}");
    let destination_key = format!("{destination_zone}:{destination_room}");
    let exit_script = world
        .room_attrs
        .get(&current_key)
        .and_then(|attrs| attrs.get("이동스크립:귀환"))
        .cloned()
        .unwrap_or_else(|| json_string(&current_info, "이동스크립:귀환"));
    let entry_script = world
        .room_attrs
        .get(&destination_key)
        .and_then(|attrs| attrs.get("진입스크립:귀환"))
        .cloned()
        .unwrap_or_else(|| json_string(&info, "진입스크립:귀환"));
    // Python schedules automatic movement after enterRoom().  It is a
    // follow-up action and must not prevent the primary 귀환 transition.

    let hazard = first_hazard(&properties);
    let (hazard_damage, hazard_message) = hazard.clone().unwrap_or((0, String::new()));

    // Floor contents do not block the transition. Python Room.update expires
    // individually represented items before updating mobs.

    // Python has already populated Room.objs before enterRoom calls update().
    // Rust instantiates the room's configured mobs lazily, so do that before
    // inspecting the update/event/aggression branches.
    world.spawn_mobs_for_room(destination_zone, destination_room);
    let now_millis = chrono::Utc::now().timestamp_millis();
    let room_update_due = destination_arc
        .read()
        .is_ok_and(|room| now_millis.saturating_sub(room.last_update_millis) >= 1_000);
    let room_update_players = world.get_players_in_room(destination_zone, destination_room);
    let room_expired_items = if room_update_due {
        world.expire_floor_items_at(
            &[(destination_zone.to_string(), destination_room.to_string())],
            now_millis as f64 / 1_000.0,
        )
    } else {
        Vec::new()
    };
    let room_update_messages = if room_update_due {
        world.update_occupied_room_mobs(&[(
            destination_zone.to_string(),
            destination_room.to_string(),
        )])
    } else {
        Vec::new()
    };
    let mob_metadata = world
        .mob_cache
        .get_all_mobs_in_room(destination_zone, destination_room)
        .into_iter()
        .filter_map(|mob| {
            world
                .mob_cache
                .get_mob(&mob.mob_key)
                .cloned()
                .map(|data| (mob.clone(), data))
        })
        .collect::<Vec<_>>();
    let mut has_entry_events = false;
    for (mob, data) in &mob_metadata {
        if data.events.contains_key(ENTRY_EVENT_KEY) {
            has_entry_events = true;
        }
        if !mob.alive
            || mob.act != 0
            || mob.hp != mob.max_hp
            || mob.mp != mob.max_mp
            || !mob.targets.is_empty()
            || !mob.skills.is_empty()
            || !mob.skill_effects.is_empty()
            || mob.str_modifier != 0
            || mob.dex_modifier != 0
            || mob.arm_modifier != 0
            || mob.mp_modifier != 0
            || mob.max_mp_modifier != 0
            || mob.hp_modifier != 0
            || mob.max_hp_modifier != 0
            || (room_update_due && mob_has_unrepresented_entry_update(&mob.mob_key, data))
        {
            // Entry proceeds; the normal room tick owns represented mob state.
        }
    }

    if room_update_due {
        // Every remaining Mob.update branch is now a no-op except the Body
        // tick increment: mobs are standing, full, target/effect free, not
        // talk-tick/item-regeneration/aggressive mobs.  Preserve that state and
        // Python Room.update's one-second gate before inserting the player.
        if let Ok(mut room) = destination_arc.write() {
            room.last_update_millis = now_millis;
        }
    }

    world.set_player_position(
        &player_name,
        PlayerPosition::new(destination_zone.to_string(), destination_room.to_string()),
    );
    drop(world);

    let position = format!("{destination_zone}:{destination_room}");
    body.set("위치", position.as_str());
    body.set("현재방", position.as_str());
    let mut result = result_map("ok");
    result.insert("exit_script".into(), Dynamic::from(exit_script));
    result.insert("entry_script".into(), Dynamic::from(entry_script));
    result.insert("has_hazard".into(), Dynamic::from(hazard.is_some()));
    result.insert("hazard_damage".into(), Dynamic::from(hazard_damage));
    result.insert("hazard_message".into(), Dynamic::from(hazard_message));
    result.insert("has_entry_events".into(), Dynamic::from(has_entry_events));
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

pub(super) fn register_return_home_efun(engine: &mut Engine, body_ptr: *mut Body) {
    let ptr = body_ptr;
    engine.register_fn("return_home", move |_ob: &mut Map| -> Map {
        return_home(unsafe { &mut *ptr })
    });
    let ptr = body_ptr;
    engine.register_fn("run_return_entry_events", move |_ob: &mut Map| -> Map {
        run_return_entry_events(unsafe { &mut *ptr })
    });
    let ptr = body_ptr;
    engine.register_fn(
        "apply_return_hazard",
        move |_ob: &mut Map, damage: i64| -> bool {
            apply_return_hazard(unsafe { &mut *ptr }, damage)
        },
    );
    let ptr = body_ptr;
    engine.register_fn("return_vitals", move |_ob: &mut Map| -> Map {
        return_vitals(unsafe { &*ptr })
    });
    engine.register_fn(
        "return_replace",
        |value: &str, from: &str, to: &str| -> String { value.replace(from, to) },
    );
    engine.register_fn("return_int_string", |value: i64| -> String {
        value.to_string()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(result: &Map) -> String {
        result
            .get("status")
            .and_then(|value| value.clone().into_string().ok())
            .unwrap_or_default()
    }

    #[test]
    fn python_integer_prefix_rules_are_used_for_room_limits() {
        assert_eq!(python_int(Some(&JsonValue::String("12명".to_string()))), 12);
        assert_eq!(python_int(Some(&JsonValue::String("-12".to_string()))), -12);
        assert_eq!(python_int(Some(&JsonValue::String("없음".to_string()))), 0);
        assert_eq!(python_int(Some(&JsonValue::from(7))), 7);
    }

    #[test]
    fn property_limit_matches_python_room_load_attr() {
        let properties = vec!["쉼금지".to_string(), "인원제한 2".to_string()];
        assert_eq!(property_limit(&properties, "인원제한"), 2);
    }

    #[test]
    fn hazard_split_matches_python_whitespace_maxsplit() {
        let properties = vec!["체력감소 1000  [공](이/가) 다칩니다".to_string()];
        assert_eq!(
            first_hazard(&properties),
            Some((1000, "[공](이/가) 다칩니다".to_string()))
        );
    }

    #[test]
    fn nonpositive_hazard_still_applies_python_minus_hp_semantics() {
        let mut body = Body::new();
        body.set("체력", 100);
        assert!(apply_return_hazard(&mut body, 0));
        assert_eq!(body.get_hp(), 100);
        assert!(apply_return_hazard(&mut body, -5));
        assert_eq!(body.get_hp(), 105);
    }

    #[test]
    fn lethal_hazard_moves_first_then_enters_python_death_state() {
        let name = "귀환치명함정차단검사";
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "5").unwrap();
            world.room_cache.get_room("호북성", "578").unwrap();
            world.set_player_position(
                name,
                PlayerPosition::new("산동성".to_string(), "5".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1);
        body.set("체력", 996);
        body.set("최고체력", 996);
        body.set("귀환지맵", "호북성:578");

        let result = return_home(&mut body);
        assert_eq!(status(&result), "ok");
        assert!(!apply_return_hazard(&mut body, 1000));
        assert_eq!(body.get_hp(), 0);
        assert_eq!(body.act, ActState::Death);
        let mut world = get_world_state().write().unwrap();
        let position = world.get_player_position(name).unwrap();
        assert_eq!(position.zone, "호북성");
        assert_eq!(position.room, "578");
        world.remove_player_position(name);
    }

    #[test]
    fn delayed_auto_move_room_still_completes_primary_return() {
        let name = "귀환자동이동차단검사";
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "6").unwrap();
            world.room_cache.get_room("귀주성", "171").unwrap();
            world.set_player_position(
                name,
                PlayerPosition::new("산동성".to_string(), "6".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1);
        body.set("체력", 10_000);
        body.set("귀환지맵", "귀주성:171");

        let result = return_home(&mut body);
        assert_eq!(status(&result), "ok");
        let mut world = get_world_state().write().unwrap();
        let position = world.get_player_position(name).unwrap();
        assert_eq!(position.zone, "귀주성");
        assert_eq!(position.room, "171");
        world.remove_player_position(name);
    }

    #[test]
    fn return_mode_never_runs_the_python_follower_branch() {
        let source = std::fs::read_to_string("objs/player.py").unwrap();
        assert!(source.contains("if f.env == prev and mode == '이동':"));
        let command = std::fs::read_to_string("cmds/귀환.py").unwrap();
        assert!(command.contains("ob.enterRoom(room, '귀환', '귀환')"));
    }

    #[test]
    fn return_updates_due_destination_corpse_before_player_insertion() {
        let name = "귀환리젠진입검사";
        let mob_key = "참회동:산딸기";
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "7").unwrap();
            world.room_cache.get_room("귀주성", "172").unwrap();
            let data = world.mob_cache.load_mob("참회동", "산딸기").unwrap();
            let mut mob = crate::world::MobInstance::new(
                mob_key.to_string(),
                "귀주성".to_string(),
                "172",
                &data,
            );
            mob.alive = false;
            mob.act = 2;
            mob.death_time = chrono::Utc::now().timestamp() - data.corpse_time - data.regen - 1;
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(
                name,
                PlayerPosition::new("산동성".to_string(), "7".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1_i64);
        body.set("체력", 10_000_i64);
        body.set("귀환지맵", "귀주성:172");

        assert_eq!(status(&return_home(&mut body)), "ok");
        let mut world = get_world_state().write().unwrap();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room("귀주성", "172")
            .into_iter()
            .find(|mob| mob.mob_key == mob_key)
            .unwrap();
        assert!(mob.alive);
        world.remove_player_position(name);
        world.mob_cache.remove_instance("귀주성", "172", mob_key);
    }

    #[test]
    fn return_runs_python_floor_item_update_before_destination_view() {
        let name = "귀환바닥만료검사";
        let item = std::sync::Arc::new(std::sync::Mutex::new(crate::object::Object::new()));
        {
            let mut object = item.lock().unwrap();
            object.set("이름", "낡은검");
            object.temp.insert(
                "timeofdrop".to_string(),
                crate::object::Value::Float(
                    chrono::Utc::now().timestamp_millis() as f64 / 1_000.0 - 601.0,
                ),
            );
        }
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "8").unwrap();
            world.room_cache.get_room("귀주성", "173").unwrap();
            world.get_room_objs_mut("귀주성", "173").push(item.clone());
            world.record_floor_item("귀주성", "173", &item);
            world.set_player_position(
                name,
                PlayerPosition::new("산동성".to_string(), "8".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", name);
        body.set("레벨", 1_i64);
        body.set("체력", 10_000_i64);
        body.set("귀환지맵", "귀주성:173");

        let result = return_home(&mut body);
        assert_eq!(status(&result), "ok");
        let expired = result["room_expired_items"]
            .clone()
            .try_cast::<Array>()
            .unwrap();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].clone().into_string().unwrap(), "낡은검");
        let mut world = get_world_state().write().unwrap();
        assert!(world.get_room_objs("귀주성", "173").is_empty());
        world.remove_player_position(name);
    }
}
