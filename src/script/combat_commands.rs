//! Python-compatible data/state efuns for `cmds/쳐.rhai` and `cmds/도망.rhai`.
//!
//! Player-visible strings and ANSI deliberately remain in the hot-reloaded
//! Rhai commands.  This module resolves room objects and performs only the
//! state transitions that the current Rust world model can prove equivalent.

use super::cast::{
    add_target_id, find_cast_target, room_player_level_dex, target_ids, with_room_player_body_mut,
};
use super::current_body_position;
use super::return_home::{
    entry_event_is_supported, first_hazard, json_string, property_limit, python_int,
};
use crate::object::Value;
use crate::player::{ActState, Body};
use crate::scheduler::CallOutScheduler;
use crate::world::{base_zone_name, get_world_state, PlayerPosition};
use rand::Rng;
use rhai::{Array, Dynamic, Engine, Map};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RUNAWAY_KEY: &str = "_runaway";
const RUNAWAY_STARTED_KEY: &str = "_runaway_started_millis";
const CONCEALED_THROW_TICK: &str = "_concealed_throw_tick";
pub(crate) const COMBAT_PRESENTATION_EVENTS: &str = "_combat_presentation_events";
pub(crate) const PVP_TARGET: &str = "_pvp_target";

pub(crate) fn pvp_target(body: &Body) -> Option<String> {
    body.temp()
        .get(PVP_TARGET)
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

pub(crate) fn clear_pvp_target(body: &mut Body) {
    body.temp_mut().remove(PVP_TARGET);
    if body.act == ActState::Fight && combat_target_ids(body).is_empty() {
        body.act = ActState::Stand;
        body.dex = 0;
        body.stop_skill();
    }
}

pub(crate) fn combat_target_ids(body: &Body) -> Vec<String> {
    target_ids(body)
}

pub(crate) fn combat_target_instance_ids(body: &Body) -> Vec<u64> {
    body.temp()
        .get("_combat_target_instance_ids")
        .and_then(Value::as_str)
        .map(|value| {
            value
                .split('\n')
                .filter_map(|entry| entry.parse::<u64>().ok())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn add_target_instance_id(body: &mut Body, id: u64) {
    let mut ids = combat_target_instance_ids(body);
    if !ids.contains(&id) {
        ids.push(id);
    }
    body.temp_mut().insert(
        "_combat_target_instance_ids".to_string(),
        Value::String(
            ids.iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    );
}

pub(crate) fn remove_combat_target_instance_id(body: &mut Body, removed: u64) {
    let mut ids = combat_target_instance_ids(body);
    ids.retain(|id| *id != removed);
    if ids.is_empty() {
        body.temp_mut().remove("_combat_target_instance_ids");
    } else {
        body.temp_mut().insert(
            "_combat_target_instance_ids".to_string(),
            Value::String(
                ids.iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        );
    }
}

pub(crate) fn remove_combat_target_id(body: &mut Body, removed: &str) -> Option<String> {
    let mut ids = target_ids(body);
    ids.retain(|id| id != removed);
    if ids.is_empty() {
        body.temp_mut().remove("_combat_target_ids");
        body.temp_mut().remove("_attack_target_key");
        return None;
    }
    let next = ids[0].clone();
    body.temp_mut().insert(
        "_combat_target_ids".to_string(),
        Value::String(ids.join("\n")),
    );
    body.temp_mut().insert(
        "_attack_target_key".to_string(),
        Value::String(next.clone()),
    );
    Some(next)
}

pub(crate) fn take_combat_presentation_events(body: &mut Body) -> Array {
    let Some(Value::String(serialized)) = body.temp_mut().remove(COMBAT_PRESENTATION_EVENTS) else {
        return Array::new();
    };
    let Ok(events) = serde_json::from_str::<Vec<serde_json::Map<String, JsonValue>>>(&serialized)
    else {
        return Array::new();
    };
    events
        .into_iter()
        .map(|event| {
            let mut map = Map::new();
            for (key, value) in event {
                map.insert(key.into(), combat_json_to_dynamic(value));
            }
            Dynamic::from(map)
        })
        .collect()
}

fn combat_json_to_dynamic(value: JsonValue) -> Dynamic {
    match value {
        JsonValue::String(value) => Dynamic::from(value),
        JsonValue::Number(value) => Dynamic::from(value.as_i64().unwrap_or(0)),
        JsonValue::Bool(value) => Dynamic::from(value),
        JsonValue::Array(values) => Dynamic::from(
            values
                .into_iter()
                .map(combat_json_to_dynamic)
                .collect::<Array>(),
        ),
        JsonValue::Object(values) => Dynamic::from(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), combat_json_to_dynamic(value)))
                .collect::<Map>(),
        ),
        JsonValue::Null => Dynamic::UNIT,
    }
}

pub(crate) fn queue_combat_presentation_event(body: &mut Body, event: serde_json::Value) {
    let mut events = body
        .temp()
        .get(COMBAT_PRESENTATION_EVENTS)
        .and_then(Value::as_str)
        .and_then(|serialized| serde_json::from_str::<Vec<serde_json::Value>>(serialized).ok())
        .unwrap_or_default();
    events.push(event);
    if let Ok(serialized) = serde_json::to_string(&events) {
        body.temp_mut().insert(
            COMBAT_PRESENTATION_EVENTS.to_string(),
            Value::String(serialized),
        );
    }
}

fn result_map(status: &str) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from(status.to_string()));
    result.insert("dir".into(), Dynamic::from(String::new()));
    result.insert("exit_script".into(), Dynamic::from(String::new()));
    result.insert("entry_script".into(), Dynamic::from(String::new()));
    result.insert("has_hazard".into(), Dynamic::from(false));
    result.insert("hazard_damage".into(), Dynamic::from(0_i64));
    result.insert("hazard_message".into(), Dynamic::from(String::new()));
    result.insert("has_entry_events".into(), Dynamic::from(false));
    result
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
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
    let dynamic = world
        .room_attrs
        .get(&format!("{zone}:{room}"))
        .and_then(|attrs| attrs.get(key));
    dynamic.map_or_else(
        || python_int(info.get(key)),
        |value| python_int(Some(&JsonValue::String(value.clone()))),
    )
}

pub(super) fn room_has_attr(body: &Body, key: &str) -> bool {
    let Some((zone, room)) = current_body_position(body) else {
        return false;
    };
    room_info(&zone, &room)
        .map(|info| room_properties(&info).iter().any(|attr| attr == key))
        .unwrap_or(false)
}

fn combat_target_count(body: &Body) -> i64 {
    let stored = target_ids(body).len();
    let count = stored.max(body.targets.len());
    if pvp_target(body).is_some() {
        count.max(1) as i64
    } else {
        count as i64
    }
}

fn attack_result(status: &str) -> Map {
    let mut result = result_map(status);
    result.insert("caster_was_stand".into(), Dynamic::from(false));
    result.insert("target_was_stand".into(), Dynamic::from(false));
    result.insert("player_skill_active".into(), Dynamic::from(false));
    result.insert("target_name".into(), Dynamic::from(String::new()));
    result.insert("target_start".into(), Dynamic::from(String::new()));
    result.insert("linked".into(), Dynamic::from(Array::new()));
    result.insert("immediate_skills".into(), Dynamic::from(Array::new()));
    result
}

fn start_pvp_attack(body: &mut Body, target_name: &str) -> Map {
    let mut result = attack_result("not_found");
    if target_name.is_empty() || target_name == body.get_name() || body.act == ActState::Death {
        return result;
    }
    let attacker_name = body.get_name();
    let attacker_was_stand = body.act == ActState::Stand;
    let Some(target_result) = with_room_player_body_mut(target_name, |target| {
        if target.act == ActState::Death || target.get_int("투명상태") == 1 {
            return None;
        }
        if pvp_target(target).is_some_and(|name| name != attacker_name)
            || !combat_target_ids(target).is_empty()
        {
            return Some(("busy", false, String::new()));
        }
        let target_was_stand = target.act == ActState::Stand;
        target
            .temp_mut()
            .insert(PVP_TARGET.to_string(), Value::String(attacker_name.clone()));
        target.act = ActState::Fight;
        target.dex = 0;
        Some(("ok", target_was_stand, target.get_weapon_name()))
    }) else {
        return result;
    };
    let Some((status, target_was_stand, target_weapon)) = target_result else {
        return result;
    };
    if status != "ok" {
        result.insert("status".into(), Dynamic::from(status));
        return result;
    }
    body.temp_mut().insert(
        PVP_TARGET.to_string(),
        Value::String(target_name.to_string()),
    );
    body.act = ActState::Fight;
    body.dex = 0;
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("caster_was_stand".into(), Dynamic::from(attacker_was_stand));
    result.insert("target_was_stand".into(), Dynamic::from(target_was_stand));
    result.insert("target_name".into(), Dynamic::from(target_name.to_string()));
    result.insert("target_weapon".into(), Dynamic::from(target_weapon));
    result
}

fn linked_event(name: &str, script: &str) -> Dynamic {
    let mut event = Map::new();
    event.insert("name".into(), Dynamic::from(name.to_string()));
    event.insert("script".into(), Dynamic::from(script.to_string()));
    Dynamic::from(event)
}

fn start_immediate_skills_for_targets(
    body: &mut Body,
    instances: &mut [crate::world::MobInstance],
    metadata: &HashMap<String, crate::world::RawMobData>,
) -> Array {
    let mut events = Array::new();
    for instance_id in combat_target_instance_ids(body) {
        let Some(index) = instances
            .iter()
            .position(|mob| mob.instance_id == instance_id && mob.alive)
        else {
            continue;
        };
        let Some(data) = metadata.get(&instances[index].mob_key) else {
            continue;
        };
        if instances[index].active_attack_skill.is_none() && !data.skills.is_empty() {
            let selected = rand::thread_rng().gen_range(0..data.skills.len());
            if crate::server::game_loop::try_start_mob_attack_skill(
                &mut instances[index],
                data,
                selected,
                rand::thread_rng().gen_range(0..=100),
            ) {
                if let Some(skill) = instances[index].active_attack_skill.as_ref() {
                    let mut event = Map::new();
                    event.insert("kind".into(), Dynamic::from("attack"));
                    event.insert("mob".into(), Dynamic::from(instances[index].name.clone()));
                    event.insert("script".into(), Dynamic::from(skill.mugong_script.clone()));
                    event.insert("amount".into(), Dynamic::from(0_i64));
                    events.push(Dynamic::from(event));
                }
            }
        }
        if let Some((script, absorbed)) = crate::server::game_loop::try_start_mob_defense_skill(
            &mut instances[index],
            data,
            body,
            &mut || rand::thread_rng().gen_range(0..=100),
        ) {
            let mut event = Map::new();
            event.insert("kind".into(), Dynamic::from("defense"));
            event.insert("mob".into(), Dynamic::from(instances[index].name.clone()));
            event.insert("script".into(), Dynamic::from(script));
            event.insert("amount".into(), Dynamic::from(absorbed));
            events.push(Dynamic::from(event));
        }
    }
    events
}

fn start_entry_aggression(body: &mut Body) -> Map {
    let mut result = attack_result("none");
    if body.get_int("투명상태") == 1 || body.act == ActState::Death {
        return result;
    }
    let Some((zone, room)) = current_body_position(body) else {
        return result;
    };
    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return result,
    };
    let metadata = world
        .mob_cache
        .get_all_mobs_in_room(&zone, &room)
        .into_iter()
        .filter_map(|mob| {
            world
                .mob_cache
                .get_mob(&mob.mob_key)
                .cloned()
                .map(|data| (mob.mob_key.clone(), data))
        })
        .collect::<HashMap<_, _>>();
    let Some(instances) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return result;
    };
    let Some(primary_index) = instances.iter().position(|mob| {
        mob.alive
            && mob.act == 0
            && metadata
                .get(&mob.mob_key)
                .is_some_and(|data| data.combat_type == 1)
    }) else {
        return result;
    };
    let player_name = body.get_name();
    let player_was_stand = body.act == ActState::Stand;
    let primary_name = instances[primary_index].name.clone();
    let primary_key = instances[primary_index].mob_key.clone();
    let primary_start = metadata
        .get(&primary_key)
        .map(|data| data.combat_start_script.clone())
        .unwrap_or_default();

    body.temp_mut()
        .insert("fightMode".to_string(), Value::Int(1));
    body.temp_mut()
        .insert("_skill_turn".to_string(), Value::Int(1));
    body.dex = 0;
    body.temp_mut().insert(
        "_attack_target_key".to_string(),
        Value::String(primary_key.clone()),
    );
    body.temp_mut().insert(
        "_attack_target".to_string(),
        Value::String(primary_name.clone()),
    );
    body.temp_mut().insert(
        "_attack_target_index".to_string(),
        Value::Int(primary_index as i64),
    );

    let mut linked = Array::new();
    for (index, mob) in instances.iter_mut().enumerate() {
        if !mob.alive || mob.act != 0 {
            continue;
        }
        let Some(data) = metadata.get(&mob.mob_key) else {
            continue;
        };
        if index != primary_index && !matches!(data.combat_type, 1 | 2) {
            continue;
        }
        add_target_id(body, &mob.mob_key);
        add_target_instance_id(body, mob.instance_id);
        if !mob.targets.iter().any(|name| name == &player_name) {
            mob.targets.push(player_name.clone());
        }
        mob.act = 1;
        if index != primary_index {
            linked.push(linked_event(&mob.name, &data.combat_start_script));
        }
    }
    let immediate_skills = start_immediate_skills_for_targets(body, instances, &metadata);
    body.act = ActState::Fight;
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("caster_was_stand".into(), Dynamic::from(player_was_stand));
    result.insert("target_was_stand".into(), Dynamic::from(true));
    result.insert("target_name".into(), Dynamic::from(primary_name));
    result.insert("target_start".into(), Dynamic::from(primary_start));
    result.insert("linked".into(), Dynamic::from(linked));
    result.insert("immediate_skills".into(), Dynamic::from(immediate_skills));
    result
}

fn start_automatic_combat_skill(body: &mut Body) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from("none"));
    result.insert("name".into(), Dynamic::from(String::new()));
    result.insert("script".into(), Dynamic::from(String::new()));
    result.insert("target_name".into(), Dynamic::from(String::new()));
    result.insert("advance".into(), Dynamic::from(false));
    if body.skill.is_some()
        || !crate::script::config_is_enabled(&body.get_string("설정상태"), "자동무공시전")
    {
        return result;
    }
    let skill_name = body.get_string("자동무공");
    if skill_name.is_empty() {
        return result;
    }
    let Some(skill) = crate::world::skill::get_skill(&skill_name) else {
        return result;
    };
    let Some(training) = body.get_skill_training(&skill_name) else {
        return result;
    };
    body.get_skill(&skill_name);
    if body.get_mp() < skill.mp_cost {
        body.stop_skill();
        result.insert("status".into(), Dynamic::from("mp_fail"));
        return result;
    }
    let max_hp = body.get_max_hp();
    let hp_cost = (max_hp * skill.hp_cost).div_euclid(100);
    let hp_required = (max_hp * skill.hp_requirement).div_euclid(100);
    if body.get_hp() < hp_cost || body.get_hp() < hp_required {
        body.stop_skill();
        result.insert("status".into(), Dynamic::from("hp_fail"));
        return result;
    }
    let mp_cost = match training.level {
        11 => skill.mp_cost * 9 / 10,
        12 => skill.mp_cost * 8 / 10,
        _ => skill.mp_cost,
    };
    body.set("내공", body.get_mp() - mp_cost);
    body.set("체력", body.get_hp() - hp_cost);
    body.add_str(skill.bonus as i32, false);
    body.temp_mut()
        .insert("_skill_turn".to_string(), Value::Int(1));
    let target_name = body
        .temp()
        .get("_attack_target")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("name".into(), Dynamic::from(skill_name));
    result.insert("script".into(), Dynamic::from(skill.mugong_script));
    result.insert("target_name".into(), Dynamic::from(target_name));
    result.insert("advance".into(), Dynamic::from(body.get_dex() >= 4200));
    result
}

/// Apply the non-skill `Player.setFight(mob)` branch used by `쳐.py`.
///
/// Automatic player and mob skills continue from the one-second combat tick.
fn start_attack(body: &mut Body, target_id: &str, room_index: i64) -> Map {
    if body.act == ActState::Death {
        // Python `setFight` silently returns for a dead attacker.
        return attack_result("dead");
    }
    // Python starts combat first and attempts the configured automatic skill
    // from the first combat tick.  Do not reject the fight merely because the
    // skill continuation is handled by the tick runner.

    let Some((zone, room)) = current_body_position(body) else {
        return attack_result("missing_target");
    };
    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return attack_result("missing_target"),
    };
    let metadata = world
        .mob_cache
        .get_all_mobs_in_room(&zone, &room)
        .into_iter()
        .filter_map(|mob| {
            world
                .mob_cache
                .get_mob(&mob.mob_key)
                .cloned()
                .map(|data| (mob.mob_key.clone(), data))
        })
        .collect::<HashMap<_, _>>();
    let Some(instances) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return attack_result("missing_target");
    };
    let Ok(target_index) = usize::try_from(room_index) else {
        return attack_result("missing_target");
    };
    let Some(target) = instances.get(target_index) else {
        return attack_result("missing_target");
    };
    if target.mob_key != target_id || !target.alive {
        return attack_result("missing_target");
    }
    let Some(target_data) = metadata.get(target_id) else {
        return attack_result("missing_target");
    };
    // Mob skill rounds are advanced by the same one-second combat tick.  The
    // target must still enter FIGHT even when its skill list is non-empty.

    let caster_was_stand = body.act == ActState::Stand;
    let player_skill_active = body.skill.is_some();
    let target_was_stand = instances[target_index].act == 0;
    let target_name = instances[target_index].name.clone();
    let target_start = target_data.combat_start_script.clone();
    let player_name = body.get_name();

    body.temp_mut()
        .insert("fightMode".to_string(), Value::Int(0));
    body.temp_mut()
        .insert("_skill_turn".to_string(), Value::Int(1));
    body.dex = 0;
    add_target_id(body, target_id);
    add_target_instance_id(body, instances[target_index].instance_id);
    body.temp_mut().insert(
        "_attack_target_key".to_string(),
        Value::String(target_id.to_string()),
    );
    body.temp_mut().insert(
        "_attack_target".to_string(),
        Value::String(target_name.clone()),
    );
    body.temp_mut().insert(
        "_attack_target_index".to_string(),
        Value::Int(target_index as i64),
    );

    if !instances[target_index]
        .targets
        .iter()
        .any(|name| name == &player_name)
    {
        instances[target_index].targets.push(player_name.clone());
    }
    instances[target_index].act = 1;

    let mut linked = Array::new();
    for (index, mob) in instances.iter_mut().enumerate() {
        if index == target_index || !mob.alive || mob.act != 0 {
            continue;
        }
        let Some(data) = metadata.get(&mob.mob_key) else {
            continue;
        };
        if !matches!(data.combat_type, 1 | 2) {
            continue;
        }
        add_target_id(body, &mob.mob_key);
        add_target_instance_id(body, mob.instance_id);
        if !mob.targets.iter().any(|name| name == &player_name) {
            mob.targets.push(player_name.clone());
        }
        mob.act = 1;
        linked.push(linked_event(&mob.name, &data.combat_start_script));
    }
    let mut immediate_skills = Array::new();
    let started_ids = combat_target_instance_ids(body);
    for instance_id in started_ids {
        let Some(index) = instances
            .iter()
            .position(|mob| mob.instance_id == instance_id && mob.alive)
        else {
            continue;
        };
        let Some(data) = metadata.get(&instances[index].mob_key) else {
            continue;
        };
        if instances[index].active_attack_skill.is_none() && !data.skills.is_empty() {
            let selected = rand::thread_rng().gen_range(0..data.skills.len());
            let roll = rand::thread_rng().gen_range(0..=100);
            if crate::server::game_loop::try_start_mob_attack_skill(
                &mut instances[index],
                data,
                selected,
                roll,
            ) {
                if let Some(skill) = instances[index].active_attack_skill.as_ref() {
                    let mut event = Map::new();
                    event.insert("kind".into(), Dynamic::from("attack"));
                    event.insert("mob".into(), Dynamic::from(instances[index].name.clone()));
                    event.insert("script".into(), Dynamic::from(skill.mugong_script.clone()));
                    event.insert("amount".into(), Dynamic::from(0_i64));
                    immediate_skills.push(Dynamic::from(event));
                }
            }
        }
        if let Some((script, absorbed)) = crate::server::game_loop::try_start_mob_defense_skill(
            &mut instances[index],
            data,
            body,
            &mut || rand::thread_rng().gen_range(0..=100),
        ) {
            let mut event = Map::new();
            event.insert("kind".into(), Dynamic::from("defense"));
            event.insert("mob".into(), Dynamic::from(instances[index].name.clone()));
            event.insert("script".into(), Dynamic::from(script));
            event.insert("amount".into(), Dynamic::from(absorbed));
            immediate_skills.push(Dynamic::from(event));
        }
    }
    body.act = ActState::Fight;

    let mut result = attack_result("ok");
    result.insert("caster_was_stand".into(), Dynamic::from(caster_was_stand));
    result.insert("target_was_stand".into(), Dynamic::from(target_was_stand));
    result.insert(
        "player_skill_active".into(),
        Dynamic::from(player_skill_active),
    );
    result.insert("target_name".into(), Dynamic::from(target_name));
    result.insert("target_start".into(), Dynamic::from(target_start));
    result.insert("linked".into(), Dynamic::from(linked));
    result.insert("immediate_skills".into(), Dynamic::from(immediate_skills));
    result
}

fn concealed_throw_result(status: &str) -> Map {
    let mut result = attack_result(status);
    result.insert("started".into(), Dynamic::from(false));
    result.insert("surprise".into(), Dynamic::from(false));
    result.insert("damage".into(), Dynamic::from(0_i64));
    result.insert("item_name".into(), Dynamic::from(String::new()));
    result.insert("target_name".into(), Dynamic::from(String::new()));
    result.insert("died".into(), Dynamic::from(false));
    result.insert("death_script".into(), Dynamic::from(String::new()));
    result
}

fn map_string(map: &Map, key: &str) -> Option<String> {
    map.get(key)
        .and_then(|value| value.clone().try_cast::<String>())
}

fn map_i64(map: &Map, key: &str) -> Option<i64> {
    map.get(key).and_then(|value| value.as_int().ok())
}

/// Consume one concealed weapon and apply its data-defined combat damage.
///
/// The command line follows the Python parser's postfix form:
/// `[target] [concealed weapon] 투척`. Presentation remains in `cmds/투척.rhai`.
fn throw_concealed_weapon(body: &mut Body, line: &str) -> Map {
    let mut words = line.split_whitespace();
    let Some(target_query) = words.next() else {
        return concealed_throw_result("usage");
    };
    let item_query = words.collect::<Vec<_>>().join(" ");
    if item_query.is_empty() {
        return concealed_throw_result("usage");
    }
    if body.act == ActState::Death {
        return concealed_throw_result("dead");
    }

    let target = find_cast_target(body, target_query);
    let Some(target) = target.try_cast::<Map>() else {
        return concealed_throw_result("missing_target");
    };
    if map_string(&target, "kind").as_deref() != Some("mob")
        || map_i64(&target, "mob_type") != Some(1)
        || map_i64(&target, "act").is_some_and(|act| act > 1)
    {
        return concealed_throw_result("invalid_target");
    }
    let Some(target_id) = map_string(&target, "id") else {
        return concealed_throw_result("missing_target");
    };
    let Some(room_index) = map_i64(&target, "room_index") else {
        return concealed_throw_result("missing_target");
    };
    let target_current = target
        .get("current")
        .and_then(|value| value.as_bool().ok())
        .unwrap_or(false);

    let selected = body
        .object
        .objs
        .iter()
        .find_map(|item| {
            let object = item.lock().ok()?;
            let aliases = object
                .getString("반응이름")
                .split("\r\n")
                .map(str::trim)
                .any(|alias| alias == item_query);
            if object.getName() != item_query && !aliases {
                return None;
            }
            if object.getString("종류") != "암기" || object.getBool("inUse") {
                return None;
            }
            let surprise_damage = object.getInt("급습위력");
            let combat_damage = object.getInt("교전위력");
            if surprise_damage <= 0 || combat_damage <= 0 {
                return None;
            }
            Some((
                Some(item.clone()),
                None,
                object.getName(),
                surprise_damage,
                combat_damage,
            ))
        })
        .or_else(|| {
            let key = super::inventory_compat::find_counted_item_key(
                &body.object.inv_stack,
                &item_query,
            )?;
            let (item, _) = super::object_from_item_json(&key)?;
            let object = item.lock().ok()?;
            if object.getString("종류") != "암기" {
                return None;
            }
            let surprise_damage = object.getInt("급습위력");
            let combat_damage = object.getInt("교전위력");
            (surprise_damage > 0 && combat_damage > 0).then(|| {
                (
                    None,
                    Some(key),
                    object.getName(),
                    surprise_damage,
                    combat_damage,
                )
            })
        });
    let Some((item, stack_key, item_name, surprise_damage, combat_damage)) = selected else {
        return concealed_throw_result("missing_item");
    };

    let was_fighting = body.act == ActState::Fight;
    if was_fighting && !target_current {
        return concealed_throw_result("other_target");
    }
    if was_fighting
        && body
            .temp()
            .get(CONCEALED_THROW_TICK)
            .is_some_and(|value| matches!(value, Value::Int(tick) if *tick == body.tick as i64))
    {
        return concealed_throw_result("cooldown");
    }
    if !was_fighting && combat_target_count(body) != 0 {
        return concealed_throw_result("other_target");
    }

    let mut result = if was_fighting {
        concealed_throw_result("ok")
    } else {
        let start = start_attack(body, &target_id, room_index);
        if map_string(&start, "status").as_deref() != Some("ok") {
            return concealed_throw_result("start_failed");
        }
        start
    };

    let Some((zone, room)) = current_body_position(body) else {
        return concealed_throw_result("missing_target");
    };
    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return concealed_throw_result("missing_target"),
    };
    let Some(data) = world.mob_cache.get_mob(&target_id).cloned() else {
        return concealed_throw_result("missing_target");
    };
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return concealed_throw_result("missing_target");
    };
    let Ok(index) = usize::try_from(room_index) else {
        return concealed_throw_result("missing_target");
    };
    let Some(mob) = mobs.get_mut(index) else {
        return concealed_throw_result("missing_target");
    };
    if mob.mob_key != target_id || !mob.alive {
        return concealed_throw_result("missing_target");
    }

    let damage = if was_fighting {
        combat_damage
    } else {
        surprise_damage
    }
    .max(1);
    let applied = damage.min(mob.hp.max(0));
    mob.hp = mob.hp.saturating_sub(applied);
    mob.record_player_damage(&body.get_name(), applied);
    let target_name = mob.name.clone();
    let died = mob.hp <= 0;
    let death_script = data.death_script.clone();
    let instance_id = mob.instance_id;
    if died {
        let _ = crate::server::game_loop::queue_admin_mob_death(mob, &data);
    }
    drop(world);

    if let Some(item) = item {
        body.object.remove(&item);
    } else if let Some(stack_key) = stack_key {
        let _ = super::inventory_compat::remove_pristine_count(&mut body.object, &stack_key, 1);
    }
    let current_tick = body.tick as i64;
    body.temp_mut()
        .insert(CONCEALED_THROW_TICK.to_string(), Value::Int(current_tick));
    if died {
        remove_combat_target_instance_id(body, instance_id);
        let next = remove_combat_target_id(body, &target_id);
        if next.is_none() && combat_target_instance_ids(body).is_empty() {
            body.act = ActState::Stand;
            body.dex = 0;
            body.stop_skill();
        }
    }
    let _ = super::save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));

    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("started".into(), Dynamic::from(!was_fighting));
    result.insert("surprise".into(), Dynamic::from(!was_fighting));
    result.insert("damage".into(), Dynamic::from(applied));
    result.insert("item_name".into(), Dynamic::from(item_name));
    result.insert("target_name".into(), Dynamic::from(target_name));
    result.insert("died".into(), Dynamic::from(died));
    result.insert("death_script".into(), Dynamic::from(death_script));
    result
}

fn reset_runaway(body: &mut Body) {
    body.set(RUNAWAY_KEY, 0_i64);
    body.temp_mut().remove(RUNAWAY_STARTED_KEY);
}

fn runaway_cooling_down(body: &mut Body) -> bool {
    if body.get_int(RUNAWAY_KEY) != 1 {
        return false;
    }
    // Production uses the one-second scheduler below.  The timestamp keeps
    // the same observed state in isolated tests or during scheduler startup.
    let started = body
        .temp()
        .get(RUNAWAY_STARTED_KEY)
        .and_then(|value| match value {
            Value::Int(value) => Some(*value),
            _ => None,
        });
    // Python getStart always resets `_runaway` to zero.  A persisted legacy
    // value without this process-local timestamp is therefore stale.
    let expired = started
        .map(|started| now_millis().saturating_sub(started) >= 1_000)
        .unwrap_or(true);
    if expired {
        reset_runaway(body);
        return false;
    }
    true
}

fn begin_runaway(body: &mut Body, scheduler: Option<&CallOutScheduler>) {
    body.set(RUNAWAY_KEY, 1_i64);
    body.temp_mut()
        .insert(RUNAWAY_STARTED_KEY.to_string(), Value::Int(now_millis()));
    if let Some(scheduler) = scheduler {
        scheduler.call_out(
            &body.get_name(),
            "cool",
            Duration::from_secs(1),
            Vec::new(),
            Some("도망".to_string()),
        );
    }
}

fn ability_bonus(body: &Body) -> i64 {
    let active = body
        .temp()
        .get("_cooltime:능파미보")
        .and_then(|value| match value {
            Value::Int(value) => Some(*value),
            _ => None,
        })
        .is_some_and(|value| value == 1)
        || body.get_int("_cooltime:능파미보") == 1;
    if active {
        40
    } else {
        0
    }
}

fn flee_chance(player_level: i64, player_dex: i64, target_level: i64, target_dex: i64) -> i64 {
    // Python integers do not overflow.  Use a wider intermediate so every
    // representable Rust attribute pair follows the same comparison path.
    let mut chance = i128::from(target_level) * (i128::from(target_dex) + 1)
        - i128::from(player_level) * (i128::from(player_dex) + 1);
    if chance < 1 {
        chance = 1;
    }
    chance = 100_i128 - chance;
    chance.max(10).min(i128::from(i64::MAX)) as i64
}

fn first_target_stats(
    body: &Body,
    world: &crate::world::WorldState,
    zone: &str,
    room: &str,
) -> Option<(i64, i64)> {
    if let Some(name) = pvp_target(body) {
        if let Some(stats) = room_player_level_dex(&name) {
            return Some(stats);
        }
    }
    let ids = target_ids(body);
    let id = ids
        .first()
        .cloned()
        .or_else(|| {
            body.temp()
                .get("_attack_target_key")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            body.temp()
                .get("_attack_target")
                .and_then(Value::as_str)
                .map(str::to_string)
        })?;
    let preferred_index = body
        .temp()
        .get("_attack_target_index")
        .and_then(|value| match value {
            Value::Int(value) => Some(*value),
            _ => None,
        })
        .and_then(|value| usize::try_from(value).ok());

    let mobs = world.mob_cache.get_all_mobs_in_room(zone, room);
    if let Some(index) = preferred_index {
        if let Some(mob) = mobs.get(index) {
            if mob.mob_key == id || mob.name == id {
                return Some((mob.level, (mob.agility + mob.dex_modifier).max(0)));
            }
        }
    }
    if let Some(mob) = mobs
        .into_iter()
        .find(|mob| mob.mob_key == id || mob.name == id)
    {
        return Some((mob.level, (mob.agility + mob.dex_modifier).max(0)));
    }
    room_player_level_dex(&id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExitChoice {
    raw_name: String,
    destinations: Vec<String>,
    random_destination: bool,
}

/// Recreate Python `Room.initExit` → `sortExit` → `setHiddenExit` ordering.
fn python_exit_choices(info: &JsonMap<String, JsonValue>) -> Vec<ExitChoice> {
    let mut insertion_order = Vec::<String>::new();
    let mut values = HashMap::<String, (Vec<String>, bool)>::new();
    for line in json_strings(info.get("출구")) {
        let words = line.split_whitespace().collect::<Vec<_>>();
        if words.len() < 2 {
            continue;
        }
        let name = words[0].to_string();
        if !values.contains_key(&name) {
            insertion_order.push(name.clone());
        }
        values.insert(
            name,
            (
                words[1..].iter().map(|word| (*word).to_string()).collect(),
                words.len() > 2,
            ),
        );
    }

    let mut ordered = Vec::new();
    for direction in [
        "동", "서", "남", "북", "위", "아래", "남동", "남서", "북동", "북서",
    ] {
        if let Some(index) = insertion_order.iter().position(|name| name == direction) {
            let name = insertion_order.remove(index);
            ordered.push(name);
        }
    }
    ordered.extend(insertion_order);

    // setHiddenExit removes the `$` key.  Keeping the raw name in exitList is
    // intentional: if getRandomExit selects it, getExit(raw_name) returns None.
    ordered
        .into_iter()
        .map(|raw_name| {
            let (destinations, random_destination) =
                values.get(&raw_name).cloned().unwrap_or_default();
            ExitChoice {
                destinations,
                random_destination,
                raw_name,
            }
        })
        .collect()
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
    if let Some(difficulty) = current_zone.chars().last().filter(char::is_ascii_digit) {
        if destination.contains(':') {
            zone.push(difficulty);
        }
    }
    (!zone.is_empty() && !room.is_empty()).then_some((zone, room))
}

fn clear_fight_for_move(
    body: &mut Body,
    world: &mut crate::world::WorldState,
    zone: &str,
    room: &str,
) {
    let player_name = body.get_name();
    if let Some(target_name) = pvp_target(body) {
        let _ = with_room_player_body_mut(&target_name, |target| {
            if pvp_target(target).as_deref() == Some(player_name.as_str()) {
                clear_pvp_target(target);
            }
        });
        body.temp_mut().remove(PVP_TARGET);
    }
    if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) {
        for mob in mobs {
            mob.targets.retain(|name| name != &player_name);
            if mob.act == 1 && mob.targets.is_empty() {
                mob.act = 0;
            }
        }
    }
    body.targets.clear();
    body.act = ActState::Stand;
    body.dex = 0;
    body.stop_skill();
    for key in [
        "_combat_target_ids",
        "_combat_target_instance_ids",
        "_attack_target_key",
        "_attack_target",
        "_attack_target_index",
    ] {
        body.temp_mut().remove(key);
    }
}

fn attempt_flee_with_roller(body: &mut Body, roll: &mut impl FnMut(usize) -> usize) -> Map {
    let player_name = body.get_name();
    let Some((current_zone, current_room)) = current_body_position(body) else {
        return result_map("unsupported_target");
    };
    let current_info = match room_info(&current_zone, &current_room) {
        Some(info) => info,
        None => return result_map("failed"),
    };

    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return result_map("failed"),
    };
    let Some((target_level, target_dex)) =
        first_target_stats(body, &world, &current_zone, &current_room)
    else {
        // Python raises at target[0] here; its command wrapper logs the error
        // and produces no invented player-facing text.
        return result_map("unsupported_target");
    };
    let chance = flee_chance(
        body.get_int("레벨"),
        body.get_dex(),
        target_level,
        target_dex,
    ) + ability_bonus(body);
    // Python `randint(0, 100)` consumes this draw before any exit draw.
    if roll(101) as i64 > chance {
        return result_map("failed");
    }

    let exits = python_exit_choices(&current_info);
    if exits.is_empty() {
        return result_map("failed");
    }
    let choice = &exits[roll(exits.len()) % exits.len()];
    // Python setHiddenExit removed this key while leaving it in exitList.
    if choice.raw_name.ends_with('$') || choice.destinations.is_empty() {
        return result_map("failed");
    }
    let destination_index = if choice.random_destination {
        roll(choice.destinations.len()) % choice.destinations.len()
    } else {
        0
    };
    let destination_value = &choice.destinations[destination_index];
    let Some((destination_zone, destination_room)) =
        resolve_exit_destination(&current_zone, &current_info, destination_value)
    else {
        return result_map("failed");
    };
    if destination_zone == current_zone && destination_room == current_room {
        return result_map("failed");
    }

    let destination_arc = match world
        .room_cache
        .get_room(&destination_zone, &destination_room)
    {
        Ok(room) => room,
        Err(_) => return result_map("failed"),
    };
    let destination_info = match room_info(&destination_zone, &destination_room) {
        Some(info) => info,
        None => return result_map("failed"),
    };
    let properties = room_properties(&destination_info);

    // `도망.py` performs these four checks itself and collapses each failure
    // to its single catch message before calling enterRoom.
    let level = body.get_int("레벨");
    if effective_room_int(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "레벨제한",
    ) > level
    {
        return result_map("failed");
    }
    let player_limit = property_limit(&properties, "인원제한");
    if player_limit > 0
        && world
            .get_players_in_room(&destination_zone, &destination_room)
            .len() as i64
            >= player_limit
    {
        return result_map("failed");
    }
    let personality = body.get_string("성격");
    if properties.iter().any(|attr| attr == "사파출입금지") && personality == "사파" {
        return result_map("failed");
    }
    if properties.iter().any(|attr| attr == "정파출입금지") && personality == "정파" {
        return result_map("failed");
    }

    // Player.enterRoom checks these again and prints its own reason before
    // 도망.py appends the common failure line.
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
    if player_limit > 0
        && world
            .get_players_in_room(&destination_zone, &destination_room)
            .len() as i64
            >= player_limit
    {
        return result_map("room_full");
    }
    if properties.iter().any(|attr| attr == "사파출입금지") && personality == "사파" {
        return result_map("evil_forbidden");
    }
    if properties.iter().any(|attr| attr == "정파출입금지") && personality == "정파" {
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
        &format!("이동스크립:{}", choice.raw_name),
    );
    let entry_script = effective_room_string(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        &format!("진입스크립:{}", choice.raw_name),
    );
    let room_auto_move = effective_room_string(
        &world,
        &destination_zone,
        &destination_room,
        &destination_info,
        "자동이동",
    );
    // Python enterRoom never rejects a destination because it contains floor
    // items.  Their lifecycle belongs to Room.update/the global room tick.
    // Likewise, automatic-route continuation and the room's automatic move
    // are follow-up commands after this successful flee, not preconditions.

    let hazard = first_hazard(&properties);
    let (hazard_damage, hazard_message) = hazard.clone().unwrap_or((0, String::new()));
    world.spawn_mobs_for_room(&destination_zone, &destination_room);
    let update_now = chrono::Utc::now().timestamp_millis();
    let room_update_due = destination_arc
        .read()
        .is_ok_and(|room| update_now.saturating_sub(room.last_update_millis) >= 1_000);
    let room_update_players = world.get_players_in_room(&destination_zone, &destination_room);
    let room_expired_items = if room_update_due {
        world.expire_floor_items_at(
            &[(destination_zone.clone(), destination_room.clone())],
            update_now as f64 / 1_000.0,
        )
    } else {
        Vec::new()
    };
    let room_update_messages = if room_update_due {
        world.update_occupied_room_mobs(&[(destination_zone.clone(), destination_room.clone())])
    } else {
        Vec::new()
    };
    let mob_metadata = world
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
    let mut has_entry_events = false;
    for (_mob, data) in &mob_metadata {
        if data.events.contains_key("이벤트 $%입장이벤트%") {
            has_entry_events = true;
            if !entry_event_is_supported(data) {
                return result_map("unsupported_entry_event");
            }
        }
    }

    if room_update_due {
        if let Ok(mut room) = destination_arc.write() {
            room.last_update_millis = update_now;
        }
    }

    clear_fight_for_move(body, &mut world, &current_zone, &current_room);
    // Python enterRoom(mode="도망") returns in Stand state before the next
    // heartbeat is rendered, so Player.update() can apply the Stand recovery
    // in the same wire response. Mirror that ordering here.
    let hp = body.get_hp();
    let max_hp = body.get_max_hp();
    if hp < max_hp {
        body.set("체력", (hp + max_hp / 10).min(max_hp));
    }
    world.set_player_position(
        &player_name,
        PlayerPosition::new(destination_zone.clone(), destination_room.clone()),
    );
    drop(world);

    let position = format!("{destination_zone}:{destination_room}");
    body.set("위치", position.as_str());
    body.set("현재방", position.as_str());
    let mut result = result_map("ok");
    result.insert("dir".into(), Dynamic::from(choice.raw_name.clone()));
    result.insert("exit_script".into(), Dynamic::from(exit_script));
    result.insert("entry_script".into(), Dynamic::from(entry_script));
    result.insert("has_hazard".into(), Dynamic::from(hazard.is_some()));
    result.insert("hazard_damage".into(), Dynamic::from(hazard_damage));
    result.insert("hazard_message".into(), Dynamic::from(hazard_message));
    result.insert("has_entry_events".into(), Dynamic::from(has_entry_events));
    result.insert("auto_move".into(), Dynamic::from(room_auto_move));
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

fn attempt_flee(body: &mut Body) -> Map {
    let mut rng = rand::thread_rng();
    attempt_flee_with_roller(body, &mut |upper| rng.gen_range(0..upper))
}

fn fight_scripts(key: &str) -> Array {
    let Ok(source) = std::fs::read_to_string("data/config/script.json") else {
        return Array::new();
    };
    let Ok(root) = serde_json::from_str::<JsonValue>(&source) else {
        return Array::new();
    };
    let table = root.get("메인설정").unwrap_or(&root);
    json_strings(table.get(key))
        .into_iter()
        .map(Dynamic::from)
        .collect()
}

pub(super) fn register_combat_command_efuns(
    engine: &mut Engine,
    body_ptr: *mut Body,
    scheduler: Option<Arc<CallOutScheduler>>,
) {
    let ptr = body_ptr;
    engine.register_fn("start_entry_aggression", move |_ob: &mut Map| -> Map {
        start_entry_aggression(unsafe { &mut *ptr })
    });
    let ptr = body_ptr;
    engine.register_fn(
        "take_combat_presentation_events",
        move |_ob: &mut Map| -> Array { take_combat_presentation_events(unsafe { &mut *ptr }) },
    );
    engine.register_fn("get_fight_scripts", fight_scripts);
    let ptr = body_ptr;
    engine.register_fn(
        "start_automatic_combat_skill",
        move |_ob: &mut Map| -> Map { start_automatic_combat_skill(unsafe { &mut *ptr }) },
    );
    let ptr = body_ptr;
    engine.register_fn("combat_room_has_attr", move |_ob: &mut Map, key: &str| {
        room_has_attr(unsafe { &*ptr }, key)
    });
    let ptr = body_ptr;
    engine.register_fn("find_attack_target", move |_ob: &mut Map, query: &str| {
        find_cast_target(unsafe { &*ptr }, query)
    });
    let ptr = body_ptr;
    engine.register_fn("combat_target_count", move |_ob: &mut Map| -> i64 {
        combat_target_count(unsafe { &*ptr })
    });
    let ptr = body_ptr;
    engine.register_fn(
        "start_attack",
        move |_ob: &mut Map, target_id: &str, room_index: i64| -> Map {
            start_attack(unsafe { &mut *ptr }, target_id, room_index)
        },
    );
    let ptr = body_ptr;
    engine.register_fn(
        "throw_concealed_weapon",
        move |_ob: &mut Map, line: &str| -> Map {
            throw_concealed_weapon(unsafe { &mut *ptr }, line)
        },
    );
    let ptr = body_ptr;
    engine.register_fn(
        "start_pvp_attack",
        move |_ob: &mut Map, target_name: &str| -> Map {
            start_pvp_attack(unsafe { &mut *ptr }, target_name)
        },
    );
    let ptr = body_ptr;
    engine.register_fn("runaway_cooling_down", move |_ob: &mut Map| -> bool {
        runaway_cooling_down(unsafe { &mut *ptr })
    });
    let ptr = body_ptr;
    let scheduler_for_begin = scheduler;
    engine.register_fn("begin_runaway", move |_ob: &mut Map| {
        begin_runaway(unsafe { &mut *ptr }, scheduler_for_begin.as_deref())
    });
    let ptr = body_ptr;
    engine.register_fn("reset_runaway", move |_ob: &mut Map| {
        reset_runaway(unsafe { &mut *ptr })
    });
    let ptr = body_ptr;
    engine.register_fn("attempt_flee", move |_ob: &mut Map| -> Map {
        attempt_flee(unsafe { &mut *ptr })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concealed_weapon_is_strong_on_entry_weak_in_combat_and_once_per_tick() {
        let suffix = std::process::id();
        let player_name = format!("암기검사자-{suffix}");
        let zone = format!("암기검사존-{suffix}");
        let room = "1";
        let mob_key = format!("{zone}:암기표적");
        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = "암기표적".into();
            data.zone = zone.clone();
            data.mob_type = 1;
            data.max_hp = 500;
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                room,
                &data,
            ));
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), room.to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.object.inv_stack.insert("비황석".to_string(), 2);

        let output = crate::script::ScriptStorage::default()
            .execute("투척", &mut body, "암기표적 비황석", None, None, None)
            .unwrap()
            .0;
        assert!(output[0].contains("허점을 꿰뚫습니다"), "{output:?}");
        assert_eq!(body.object.inv_stack.get("비황석"), Some(&1));
        assert!(body.object.objs.is_empty());
        {
            let world = get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room(&zone, room)
                .into_iter()
                .find(|mob| mob.mob_key == mob_key)
                .unwrap();
            assert_eq!(mob.hp, 420);
        }

        let blocked = throw_concealed_weapon(&mut body, "암기표적 비황석");
        assert_eq!(map_string(&blocked, "status").as_deref(), Some("cooldown"));
        assert_eq!(body.object.inv_stack.get("비황석"), Some(&1));

        body.tick += 1;
        let combat = throw_concealed_weapon(&mut body, "암기표적 비황석");
        assert_eq!(map_string(&combat, "status").as_deref(), Some("ok"));
        assert_eq!(map_i64(&combat, "damage"), Some(12));
        assert!(!combat["surprise"].as_bool().unwrap());
        assert!(!body.object.inv_stack.contains_key("비황석"));
        assert!(body.object.objs.is_empty());
        {
            let world = get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room(&zone, room)
                .into_iter()
                .find(|mob| mob.mob_key == mob_key)
                .unwrap();
            assert_eq!(mob.hp, 408);
        }

        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn pvp_start_sets_reciprocal_live_player_state_and_rejects_busy_target() {
        let mut attacker = Body::new();
        attacker.set("이름", "비무공격자");
        let mut defender = Body::new();
        defender.set("이름", "비무방어자");
        super::super::cast::set_cast_room_players(vec![
            super::super::cast::CastRoomPlayerRef::new(&mut defender),
        ]);
        let started = start_pvp_attack(&mut attacker, "비무방어자");
        assert_eq!(started["status"].clone_cast::<String>(), "ok");
        assert_eq!(pvp_target(&attacker).as_deref(), Some("비무방어자"));
        assert_eq!(pvp_target(&defender).as_deref(), Some("비무공격자"));
        assert_eq!(attacker.act, ActState::Fight);
        assert_eq!(defender.act, ActState::Fight);
        clear_pvp_target(&mut attacker);
        clear_pvp_target(&mut defender);
        super::super::cast::clear_cast_room_players();
    }

    #[test]
    fn rhai_attack_honors_user_combat_forbidden_room_then_starts_pvp() {
        let suffix = std::process::id();
        let attacker_name = format!("비무명령공격자-{suffix}");
        let defender_name = format!("비무명령방어자-{suffix}");
        let mut attacker = Body::new();
        attacker.set("이름", attacker_name.as_str());
        let mut defender = Body::new();
        defender.set("이름", defender_name.as_str());
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &attacker_name,
                PlayerPosition::new("낙양성".to_string(), "56".to_string()),
            );
            world.set_player_position(
                &defender_name,
                PlayerPosition::new("낙양성".to_string(), "56".to_string()),
            );
        }
        super::super::cast::set_cast_room_players(vec![
            super::super::cast::CastRoomPlayerRef::new(&mut defender),
        ]);
        let storage = crate::script::ScriptStorage::default();
        let denied = storage
            .execute("쳐", &mut attacker, &defender_name, None, None, None)
            .unwrap();
        assert_eq!(
            denied.0,
            vec![
                "☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네"
            ]
        );
        assert!(pvp_target(&attacker).is_none());
        assert!(pvp_target(&defender).is_none());

        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &attacker_name,
                PlayerPosition::new("낙양성".to_string(), "1000".to_string()),
            );
            world.set_player_position(
                &defender_name,
                PlayerPosition::new("낙양성".to_string(), "1000".to_string()),
            );
        }
        super::super::cast::set_cast_room_players(vec![
            super::super::cast::CastRoomPlayerRef::new(&mut defender),
        ]);
        let started = storage
            .execute("쳐", &mut attacker, &defender_name, None, None, None)
            .unwrap();
        assert_eq!(started.0, vec!["당신이 주먹을 쥐며 공격 합니다."]);
        assert_eq!(
            pvp_target(&attacker).as_deref(),
            Some(defender_name.as_str())
        );
        assert_eq!(
            pvp_target(&defender).as_deref(),
            Some(attacker_name.as_str())
        );
        super::super::cast::set_cast_room_players(vec![
            super::super::cast::CastRoomPlayerRef::new(&mut defender),
        ]);
        let repeated = storage
            .execute("쳐", &mut attacker, &defender_name, None, None, None)
            .unwrap();
        assert_eq!(repeated.0, vec!["☞ 이미 공격중이에요. ^_^"]);
        super::super::cast::clear_cast_room_players();
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&attacker_name);
        world.remove_player_position(&defender_name);
    }
    use crate::script::ScriptStorage;
    use crate::world::MobInstance;

    #[test]
    fn entry_aggression_selects_first_type_one_and_links_type_two() {
        let name = "선공진입검사";
        let zone = "선공진입검사존";
        let room = "1";
        let first_key = format!("{zone}:선공");
        let linked_key = format!("{zone}:합공");
        {
            let mut world = get_world_state().write().unwrap();
            let mut first = crate::world::RawMobData::new();
            first.name = "선공몹".to_string();
            first.zone = zone.to_string();
            first.mob_type = 1;
            first.combat_type = 1;
            first.combat_start_script = "덤벼듭니다.".to_string();
            let mut linked = first.clone();
            linked.name = "합공몹".to_string();
            linked.combat_type = 2;
            world
                .mob_cache
                .insert_mob_data(first_key.clone(), first.clone());
            world
                .mob_cache
                .insert_mob_data(linked_key.clone(), linked.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                first_key.clone(),
                zone.to_string(),
                room,
                &first,
            ));
            world.mob_cache.add_mob_instance(MobInstance::new(
                linked_key.clone(),
                zone.to_string(),
                room,
                &linked,
            ));
            world.set_player_position(
                name,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", name);
        let result = start_entry_aggression(&mut body);
        assert_eq!(result["status"].clone().into_string().unwrap(), "ok");
        assert_eq!(body.act, ActState::Fight);
        assert!(matches!(body.temp().get("fightMode"), Some(Value::Int(1))));
        assert_eq!(
            target_ids(&body),
            vec![first_key.clone(), linked_key.clone()]
        );
        let runtime_ids = combat_target_instance_ids(&body);
        assert_eq!(runtime_ids.len(), 2);
        assert_ne!(runtime_ids[0], runtime_ids[1]);
        {
            let world = get_world_state().read().unwrap();
            let mobs = world.mob_cache.get_all_mobs_in_room(zone, room);
            assert!(mobs.iter().all(|mob| mob.act == 1));
            assert!(mobs.iter().all(|mob| mob.targets == vec![name.to_string()]));
        }
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(name);
        world.mob_cache.remove_instance(zone, room, &first_key);
        world.mob_cache.remove_instance(zone, room, &linked_key);
        world.mob_cache.remove_mob(&first_key);
        world.mob_cache.remove_mob(&linked_key);
    }

    #[test]
    fn removing_primary_combat_target_promotes_next_in_insertion_order() {
        let mut body = Body::new();
        body.temp_mut().insert(
            "_combat_target_ids".to_string(),
            Value::String("첫째\n둘째\n셋째".to_string()),
        );
        body.temp_mut().insert(
            "_attack_target_key".to_string(),
            Value::String("첫째".to_string()),
        );
        assert_eq!(
            remove_combat_target_id(&mut body, "첫째"),
            Some("둘째".to_string())
        );
        assert_eq!(combat_target_ids(&body), vec!["둘째", "셋째"]);
        assert_eq!(body.temp()["_attack_target_key"].as_str(), Some("둘째"));
    }

    #[test]
    fn entry_auto_skill_applies_python_cost_discount_and_initial_state() {
        crate::world::skill::reload_skill_cache().unwrap();
        let mut body = Body::new();
        body.set("이름", "자동무공진입검사");
        body.set("설정상태", "자동무공시전 1");
        body.set("자동무공", "가의신공");
        body.set("체력", 1_000_i64);
        body.set("최고체력", 1_000_i64);
        body.set("내공", 500_i64);
        body.set("최고내공", 500_i64);
        body.temp_mut().insert(
            "_attack_target".to_string(),
            Value::String("선공몹".to_string()),
        );
        body.set_skill_training("가의신공", 11, 0);

        let result = start_automatic_combat_skill(&mut body);
        assert_eq!(result["status"].clone().into_string().unwrap(), "ok");
        assert_eq!(body.skill.as_deref(), Some("가의신공"));
        // 내공소모 240, level 11 => int(240 * 0.9) == 216.
        assert_eq!(body.get_mp(), 284);
        assert_eq!(body.get_int("힘경험치"), 4);
        assert_eq!(
            result["target_name"].clone().into_string().unwrap(),
            "선공몹"
        );
        assert!(matches!(
            body.temp().get("_skill_turn"),
            Some(Value::Int(1))
        ));
    }

    #[test]
    fn flee_formula_matches_python_boundaries() {
        assert_eq!(flee_chance(10, 15, 10, 15), 99);
        assert_eq!(flee_chance(1, 0, 100, 100), 10);
    }

    #[test]
    fn combat_presentation_events_are_structured_and_consumed_once() {
        let mut body = Body::new();
        queue_combat_presentation_event(
            &mut body,
            serde_json::json!({
                "kind": "mob_attack",
                "mob": "산적",
                "player": "검사",
                "script": "[공](이/가) [방](을/를) 공격합니다",
                "damage": 17
            }),
        );
        let events = take_combat_presentation_events(&mut body);
        assert_eq!(events.len(), 1);
        let event = events[0].clone().cast::<Map>();
        assert_eq!(event["kind"].clone().into_string().unwrap(), "mob_attack");
        assert_eq!(event["damage"].as_int().unwrap(), 17);
        assert!(take_combat_presentation_events(&mut body).is_empty());
    }

    #[test]
    fn random_exit_order_matches_python_direction_order_and_hidden_behavior() {
        let mut info = JsonMap::new();
        info.insert(
            "출구".to_string(),
            serde_json::json!(["북 2", "문 3", "동 4 5", "서$ 6"]),
        );
        let exits = python_exit_choices(&info);
        assert_eq!(
            exits
                .iter()
                .map(|exit| exit.raw_name.as_str())
                .collect::<Vec<_>>(),
            vec!["동", "북", "문", "서$"]
        );
        assert_eq!(exits[0].destinations, vec!["4", "5"]);
        assert!(exits[3].raw_name.ends_with('$'));
    }

    #[test]
    fn one_second_runaway_state_resets_without_persisting_a_fake_message() {
        let mut body = Body::new();
        body.set("이름", "도망재사용검사");
        begin_runaway(&mut body, None);
        assert!(runaway_cooling_down(&mut body));
        body.temp_mut().insert(
            RUNAWAY_STARTED_KEY.to_string(),
            Value::Int(now_millis() - 1_001),
        );
        assert!(!runaway_cooling_down(&mut body));
        assert_eq!(body.get_int(RUNAWAY_KEY), 0);
    }

    #[test]
    fn source_is_anchored_to_python_randint_and_enter_room_calls() {
        let source = std::fs::read_to_string("cmds/도망.py").unwrap();
        assert!(source.contains("c2 = randint(0, 100)"));
        assert!(source.contains("room, dir = ob.env.getRandomExit()"));
        assert!(source.contains("ob.enterRoom(room, dir, '도망')"));
        assert!(source.contains("reactor.callLater(1, self.cool, ob)"));
    }

    #[test]
    fn successful_flee_clears_reverse_target_and_moves_through_selected_exit() {
        let player_name = "도망성공전이검사";
        let (mob_key, target_index) = {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("절강성", "1").unwrap();
            world.room_cache.get_room("절강성", "2").unwrap();
            world
                .get_room_attrs_mut("절강성", "2")
                .insert("자동이동".to_string(), "남".to_string());
            world
                .get_room_objs_stack_mut("절강성", "2")
                .insert("회귀검사스택".to_string(), 2);
            let data = world
                .mob_cache
                .load_mob("참회동", "산딸기")
                .expect("repository mob fixture");
            let mob_key = "참회동:산딸기".to_string();
            let target_index = world.mob_cache.get_all_mobs_in_room("절강성", "1").len();
            let mut mob = MobInstance::new(mob_key.clone(), "절강성".to_string(), "1", &data);
            mob.act = 1;
            mob.targets.push(player_name.to_string());
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(
                player_name,
                PlayerPosition::new("절강성".to_string(), "1".to_string()),
            );
            (mob_key, target_index)
        };

        let mut body = Body::new();
        body.set("이름", player_name);
        body.set("레벨", 1_000_i64);
        body.set("민첩성", 1_000_i64);
        body.set("체력", 10_000_i64);
        body.set("최고체력", 10_000_i64);
        body.act = ActState::Fight;
        add_target_id(&mut body, &mob_key);
        body.temp_mut().insert(
            "_attack_target_index".to_string(),
            Value::Int(target_index as i64),
        );

        // Draws: randint(0,100)=0, then `북`. Python sortExit orders this
        // room's `남` before `북`, so the second exit index is one.
        let mut draws = vec![0_usize, 1_usize].into_iter();
        let result =
            attempt_flee_with_roller(&mut body, &mut |upper| draws.next().unwrap_or(0) % upper);
        assert_eq!(result["status"].clone().into_string().unwrap(), "ok");
        assert_eq!(result["dir"].clone().into_string().unwrap(), "북");
        assert_eq!(result["auto_move"].clone().into_string().unwrap(), "남");
        assert_eq!(body.act, ActState::Stand);
        assert!(target_ids(&body).is_empty());
        let mut world = get_world_state().write().unwrap();
        let position = world.get_player_position(player_name).unwrap();
        assert_eq!(
            (position.zone.as_str(), position.room.as_str()),
            ("절강성", "2")
        );
        let old_mob = &world.mob_cache.get_all_mobs_in_room("절강성", "1")[target_index];
        assert_eq!(old_mob.act, 0);
        assert!(!old_mob.targets.iter().any(|name| name == player_name));
        world.remove_player_position(player_name);
        world.get_room_attrs_mut("절강성", "2").remove("자동이동");
        world
            .get_room_objs_stack_mut("절강성", "2")
            .remove("회귀검사스택");
        if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut("절강성", "1") {
            mobs.remove(target_index);
        }
    }

    #[test]
    fn rhai_attack_usage_and_start_state_match_python() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "공격명령회귀검사");

        let (output, special) = storage
            .execute("쳐", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 사용법: [대상] 공격"]);
        assert!(special.is_none());

        let zone = "공격명령회귀검사존";
        let room = "1";
        let mut observer = Body::new();
        observer.set("이름", "공격관전자");
        observer.set("체력", 41_i64);
        observer.set("최고체력", 42_i64);
        observer.set("내공", 8_i64);
        observer.set("최고내공", 9_i64);
        let mut rejecting = Body::new();
        rejecting.set("이름", "공격출력거부관전자");
        rejecting.set("설정상태", "타인전투출력거부 1");
        rejecting.set("체력", 11_i64);
        rejecting.set("최고체력", 12_i64);
        rejecting.set("내공", 3_i64);
        rejecting.set("최고내공", 4_i64);
        super::super::cast::set_cast_room_players(vec![
            super::super::cast::CastRoomPlayerRef::new(&mut observer),
            super::super::cast::CastRoomPlayerRef::new(&mut rejecting),
        ]);
        let (mob_key, mob_name, mob_start) = {
            let mut world = get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("참회동", "산딸기")
                .expect("repository mob fixture");
            let mob_key = "참회동:산딸기".to_string();
            let mob_name = data.name.clone();
            let mob_start = data.combat_start_script.clone();
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.to_string(),
                room,
                &data,
            ));
            world.set_player_position(
                "공격명령회귀검사",
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            world.set_player_position(
                "공격관전자",
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            world.set_player_position(
                "공격출력거부관전자",
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            (mob_key, mob_name, mob_start)
        };

        let (output, special) = storage
            .execute("쳐", &mut body, &mob_name, None, None, None)
            .unwrap();
        let sends = match special {
            Some(crate::command::CommandResult::OutputAndSendToUsers(_, sends)) => sends,
            other => panic!("expected attack room deliveries, got {other:?}"),
        };
        for (name, prompt) in [
            (
                "공격관전자",
                "\0MUC_RAW_USER\0\r\n\x1b[0;37;40m[ 41/42, 8/9 ] ",
            ),
            (
                "공격출력거부관전자",
                "\0MUC_RAW_USER\0\r\n\x1b[0;37;40m[ 11/12, 3/4 ] ",
            ),
        ] {
            let messages = sends
                .iter()
                .filter(|(recipient, _)| recipient == name)
                .map(|(_, message)| message.as_str())
                .collect::<Vec<_>>();
            assert_eq!(messages.len(), 2, "{name}: {messages:?}");
            assert!(messages[0].contains("주먹을 쥐며 공격 합니다."));
            assert!(messages[0].contains(&mob_name));
            assert_eq!(messages[1], prompt);
        }
        assert_eq!(
            output,
            vec![
                "당신이 주먹을 쥐며 공격 합니다.".to_string(),
                format!(
                    "\x1b[33m{}\x1b[37m{} {}",
                    mob_name,
                    crate::hangul::han_iga(&mob_name),
                    mob_start
                ),
            ]
        );
        assert_eq!(body.act, ActState::Fight);
        assert_eq!(
            body.temp()["_attack_target_key"].as_str(),
            Some(mob_key.as_str())
        );

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position("공격명령회귀검사");
        world.remove_player_position("공격관전자");
        world.remove_player_position("공격출력거부관전자");
        drop(world);
        super::super::cast::clear_cast_room_players();
    }

    #[test]
    fn rhai_attack_distinguishes_same_template_mob_instances() {
        let storage = ScriptStorage::default();
        let player = "동명몹공격회귀검사";
        let zone = "동명몹공격회귀검사존";
        let room = "1";
        let (mob_key, current_id) = {
            let mut world = get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("참회동", "산딸기")
                .expect("repository mob fixture");
            let mob_key = "참회동:산딸기".to_string();
            let first = MobInstance::new(mob_key.clone(), zone.to_string(), room, &data);
            let first_id = first.instance_id;
            let second = MobInstance::new(mob_key.clone(), zone.to_string(), room, &data);
            let second_id = second.instance_id;
            world.mob_cache.add_mob_instance(first);
            world.record_test_room_object(zone, room, crate::world::RoomObjectRef::Mob(first_id));
            world.mob_cache.add_mob_instance(second);
            world.record_test_room_object(zone, room, crate::world::RoomObjectRef::Mob(second_id));
            world.set_player_position(
                player,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            (mob_key, second_id)
        };

        let mut body = Body::new();
        body.set("이름", player);
        body.act = ActState::Fight;
        body.temp_mut()
            .insert("_combat_target_ids".to_string(), Value::String(mob_key));
        body.temp_mut().insert(
            "_combat_target_instance_ids".to_string(),
            Value::String(current_id.to_string()),
        );

        // Python Room.findObjName(".") rewrites the selector to "1", so the
        // complete `. 쳐` command attacks the first eligible room mob.
        let first = storage
            .execute("쳐", &mut body, ".", None, None, None)
            .unwrap();
        assert_eq!(first.0, vec!["☞ 이미 공격중이에요. ^_^"]);
        let second = storage
            .execute("쳐", &mut body, "2", None, None, None)
            .unwrap();
        assert_eq!(second.0, vec!["☞ 현재의 비무에 신경을 집중하세요. @_@"]);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(player);
        if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) {
            mobs.clear();
        }
    }

    #[test]
    fn rhai_flee_guards_match_python_before_random_state() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "도망명령문구검사");

        let (output, special) = storage
            .execute("도망", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 무림인은 아무때나 도망가는것이 아니라네"]);
        assert!(special.is_none());

        body.act = ActState::Fight;
        body.set(RUNAWAY_KEY, 1_i64);
        body.temp_mut()
            .insert(RUNAWAY_STARTED_KEY.to_string(), Value::Int(now_millis()));
        let (output, special) = storage
            .execute("도망", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 도망 갈려다 잡혔어요. '흑흑~~ T_T'"]);
        assert!(special.is_none());
    }

    #[test]
    fn flee_observer_prompt_uses_live_vitals_and_python_visibility_rules() {
        let mut visible = rhai::Map::new();
        visible.insert("이름".into(), Dynamic::from("도망관전자"));
        visible.insert("show_prompt".into(), Dynamic::from(true));
        visible.insert("현재체력".into(), Dynamic::from(31_i64));
        visible.insert("현재최고체력".into(), Dynamic::from(45_i64));
        visible.insert("현재내공".into(), Dynamic::from(7_i64));
        visible.insert("현재최고내공".into(), Dynamic::from(9_i64));
        let mut hidden = visible.clone();
        hidden.insert("이름".into(), Dynamic::from("프롬프트거부관전자"));
        hidden.insert("show_prompt".into(), Dynamic::from(false));
        let online = vec![Dynamic::from(visible), Dynamic::from(hidden)];
        let mut engine = rhai::Engine::new();
        engine.register_fn("get_all_online_players", move || online.clone());
        let source = std::fs::read_to_string("cmds/도망.rhai").unwrap();
        let shown = engine
            .eval::<String>(&format!("{source}\nflee_prompt(\"도망관전자\")"))
            .unwrap();
        assert_eq!(shown, "\r\n\x1b[0;37;40m[ 31/45, 7/9 ] ");
        let suppressed = engine
            .eval::<String>(&format!("{source}\nflee_prompt(\"프롬프트거부관전자\")"))
            .unwrap();
        assert_eq!(suppressed, "");
    }

    #[test]
    fn combat_tick_room_event_uses_python_send_fight_script_prompt_wire() {
        let mut actor = Body::new();
        actor.set("이름", "틱공격자");
        let mut observer = Body::new();
        observer.set("이름", "틱관전자");
        observer.set("설정상태", "타인전투출력거부 0");
        observer.set("체력", 31_i64);
        observer.set("최고체력", 45_i64);
        observer.set("내공", 7_i64);
        observer.set("최고내공", 9_i64);
        super::super::cast::set_cast_room_players(vec![
            super::super::cast::CastRoomPlayerRef::new(&mut observer),
        ]);
        let mut details = rhai::Map::new();
        details.insert("이름".into(), Dynamic::from("틱관전자"));
        details.insert("show_prompt".into(), Dynamic::from(true));
        details.insert("현재체력".into(), Dynamic::from(31_i64));
        details.insert("현재최고체력".into(), Dynamic::from(45_i64));
        details.insert("현재내공".into(), Dynamic::from(7_i64));
        details.insert("현재최고내공".into(), Dynamic::from(9_i64));
        crate::script::set_precomputed_all_online(vec![Dynamic::from(details)]);
        queue_combat_presentation_event(&mut actor, serde_json::json!({"kind": "anger_100"}));

        let result = ScriptStorage::default()
            .execute("__combat_tick", &mut actor, "", None, None, None)
            .unwrap();
        let sends = match result.1.unwrap() {
            crate::command::CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected combat tick delivery: {other:?}"),
        };
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].0, "틱관전자");
        assert_eq!(
            sends[0].1,
            format!(
                "{}\r\n\x1b[1m틱공격자\x1b[0;37m 갑자기 \x1b[1;40;31m괴성\x1b[0;40;37m을 지르며 \x1b[1;40;31m난동\x1b[0;40;37m을 부립니다. '끄오오오오오~~'\r\n\r\n\x1b[0;37;40m[ 31/45, 7/9 ] ",
                crate::script::RAW_USER_MESSAGE_PREFIX,
            )
        );
        super::super::cast::clear_cast_room_players();
        crate::script::clear_precomputed_all_online();
    }

    #[test]
    fn combat_tick_defers_raw_self_prompt_until_after_death_rewards() {
        let mut actor = Body::new();
        actor.set("이름", "보상프롬프트검사");
        actor.set("체력", 1333_i64);
        actor.set("최고체력", 1594_i64);
        actor.set("내공", 19_i64);
        actor.set("최고내공", 19_i64);
        queue_combat_presentation_event(
            &mut actor,
            serde_json::json!({
                "kind": "mob_death", "mob": "청년",
                "script": "청년이 고통스런 신음소리를 지르며 쓰러집니다."
            }),
        );
        queue_combat_presentation_event(&mut actor, serde_json::json!({ "kind": "combat_prompt" }));
        queue_combat_presentation_event(
            &mut actor,
            serde_json::json!({
                "kind": "reward", "mob": "청년", "exp": 32,
                "bonus_exp": 0, "gold": 18, "bonus_gold": 0,
                "difficulty": 0
            }),
        );

        let result = ScriptStorage::default()
            .execute("__combat_tick", &mut actor, "", None, None, None)
            .unwrap();
        let (output, sends) = match result.1.unwrap() {
            crate::command::CommandResult::OutputAndSendToUsers(output, sends) => (output, sends),
            other => panic!("unexpected combat tick result: {other:?}"),
        };
        let death = output.find("청년이 고통스런").unwrap();
        let exp = output.find("당신이 32의 경험치를").unwrap();
        let gold = output.find("은전 18개를 획득합니다.").unwrap();
        assert!(death < exp && exp < gold, "{output:?}");
        assert!(!output.contains("[ 1333/1594, 19/19 ]"));
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].0, "보상프롬프트검사");
        assert_eq!(
            sends[0].1,
            format!(
                "{}\r\n\x1b[0;37;40m[ 1333/1594, 19/19 ] ",
                crate::script::RAW_USER_MESSAGE_PREFIX
            )
        );
    }

    #[test]
    fn combat_tick_respawn_event_uses_python_write_room_then_prompt_wire() {
        let mut actor = Body::new();
        actor.set("이름", "리젠목격자");
        let mut details = rhai::Map::new();
        details.insert("이름".into(), Dynamic::from("리젠목격자"));
        details.insert("show_prompt".into(), Dynamic::from(true));
        details.insert("현재체력".into(), Dynamic::from(21_i64));
        details.insert("현재최고체력".into(), Dynamic::from(30_i64));
        details.insert("현재내공".into(), Dynamic::from(4_i64));
        details.insert("현재최고내공".into(), Dynamic::from(6_i64));
        crate::script::set_precomputed_all_online(vec![Dynamic::from(details)]);
        queue_combat_presentation_event(
            &mut actor,
            serde_json::json!({
                "kind": "room_mob_respawn",
                "text": "귀소몹이 원래 자리에서 다시 나타납니다.",
            }),
        );

        let result = ScriptStorage::default()
            .execute("__combat_tick", &mut actor, "", None, None, None)
            .unwrap();
        // End the actor's already-visible prompt before the asynchronous
        // room respawn notification is delivered.
        assert_eq!(result.0, vec![""]);
        let sends = match result.1.unwrap() {
            crate::command::CommandResult::SendToUsers(sends) => sends,
            crate::command::CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected respawn delivery: {other:?}"),
        };
        assert_eq!(
            sends,
            [(
                "리젠목격자".to_string(),
                format!(
                    "{}\r\n귀소몹이 원래 자리에서 다시 나타납니다.\r\n\r\n\x1b[0;37;40m[ 21/30, 4/6 ] ",
                    crate::script::RAW_USER_MESSAGE_PREFIX,
                ),
            )]
        );
        crate::script::clear_precomputed_all_online();
    }

    #[tokio::test]
    async fn attack_and_flee_are_hot_reloadable_rhai_not_rust_builtins() {
        let storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::default()));
        let mut registry = crate::command::CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        assert!(registry.get("쳐").is_none());
        assert!(registry.get("도망").is_none());

        crate::command::commands::register_script_commands(
            &mut registry,
            storage,
            None,
            None,
            None,
        )
        .await;
        assert_eq!(registry.get("쳐").unwrap().description, "쳐 명령어");
        assert_eq!(registry.get("도망").unwrap().description, "도망 명령어");
        assert_eq!(registry.get("공격").unwrap().name, "쳐");
        assert_eq!(registry.get("도").unwrap().name, "도망");
        for invented in ["attack", "kill", "flee", "run", "duel", "pvp"] {
            assert!(registry.get(invented).is_none(), "{invented}");
        }
    }
}
