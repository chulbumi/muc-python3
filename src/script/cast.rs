//! Python-compatible state/data efuns for `cmds/시전.rhai`.
//!
//! User-facing text deliberately stays in Rhai.  This module only resolves
//! runtime objects and applies the state transitions performed by
//! `cmds/시전.py`.

use super::{current_body_position, reaction_names};
use crate::object::Value;
use crate::player::{ActState, ActiveSkill, Body, SkillTraining};
use crate::world::skill::Skill;
use crate::world::{get_skill, get_world_state, MobInstance, MobSkillEffect, RoomObjectRef};
use rhai::{Dynamic, Engine, Map};
use std::cell::RefCell;

/// Other players in the caster's room while one command is executing.
///
/// `handle_single_game_command` owns the clients mutex for the whole Rhai
/// call, so these pointers remain stable and no other task can mutate the
/// referenced bodies.  The current player is intentionally omitted to avoid
/// aliasing the `&mut Body` passed into the script engine.
#[derive(Clone, Copy)]
pub(crate) struct CastRoomPlayerRef {
    body: *mut Body,
    interactive: i32,
}

impl CastRoomPlayerRef {
    #[cfg(test)]
    pub(crate) fn new(body: &mut Body) -> Self {
        Self {
            body: body as *mut Body,
            interactive: 1,
        }
    }

    pub(crate) fn new_with_interactive(body: &mut Body, interactive: i32) -> Self {
        Self {
            body: body as *mut Body,
            interactive,
        }
    }
}

thread_local! {
    static CAST_ROOM_PLAYERS: RefCell<Option<Vec<CastRoomPlayerRef>>> = const { RefCell::new(None) };
}

pub(crate) fn set_cast_room_players(players: Vec<CastRoomPlayerRef>) {
    CAST_ROOM_PLAYERS.with(|cell| *cell.borrow_mut() = Some(players));
}

pub(crate) fn clear_cast_room_players() {
    CAST_ROOM_PLAYERS.with(|cell| *cell.borrow_mut() = None);
}

fn other_player_refs() -> Vec<CastRoomPlayerRef> {
    CAST_ROOM_PLAYERS
        .with(|cell| cell.borrow().clone())
        .unwrap_or_default()
}

pub(super) fn with_room_player_body_mut<R>(
    name: &str,
    visit: impl FnOnce(&mut Body) -> R,
) -> Option<R> {
    for player_ref in other_player_refs() {
        // SAFETY: refs are installed only while the clients mutex is held and
        // never include the command actor.
        let player = unsafe { &mut *player_ref.body };
        if player.get_name() == name {
            return Some(visit(player));
        }
    }
    None
}

/// Snapshot one live player from the already-scoped same-room command
/// context.  This deliberately never scans all connected players.
pub(super) fn room_player_level_dex(name: &str) -> Option<(i64, i64)> {
    other_player_refs().into_iter().find_map(|player_ref| {
        // SAFETY: `CastRoomPlayerRef` is installed while the clients mutex is
        // held for this command and cleared before that scope ends.
        let player = unsafe { &*player_ref.body };
        (player.get_name() == name).then(|| (player.get_int("레벨"), player.get_dex()))
    })
}

pub(super) fn target_ids(body: &Body) -> Vec<String> {
    let mut ids = body
        .temp()
        .get("_combat_target_ids")
        .and_then(Value::as_str)
        .map(|value| {
            value
                .split('\n')
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(value) = body
        .temp()
        .get("_attack_target_key")
        .and_then(Value::as_str)
    {
        if !value.is_empty() && !ids.iter().any(|entry| entry == value) {
            ids.push(value.to_string());
        }
    }
    ids
}

pub(super) fn add_target_id(body: &mut Body, id: &str) {
    let mut ids = target_ids(body);
    if !ids.iter().any(|entry| entry == id) {
        ids.push(id.to_string());
    }
    body.temp_mut().insert(
        "_combat_target_ids".to_string(),
        Value::String(ids.join("\n")),
    );
}

fn player_target_names(body: &Body) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(name) = body.temp().get("_attack_target").and_then(Value::as_str) {
        if !name.is_empty() {
            names.push(name.to_string());
        }
    }
    for target in &body.targets {
        if let Some(target) = target.upgrade() {
            if let Ok(target) = target.lock() {
                let name = target.getName();
                if !name.is_empty() && !names.iter().any(|entry| entry == &name) {
                    names.push(name);
                }
            }
        }
    }
    names
}

fn target_instance_ids(body: &Body) -> Vec<u64> {
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

fn leading_order(value: &str) -> i64 {
    value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn split_target_query(line: &str) -> (String, i64, bool) {
    let mut name = line.split_whitespace().next().unwrap_or("").to_string();
    if name.trim() == "." {
        name = "1".to_string();
    }
    let only_number = !name.is_empty() && name.chars().all(|character| character.is_ascii_digit());
    let parsed = leading_order(&name);
    if parsed != 0 && !only_number {
        name = name
            .trim_start_matches(|character: char| character.is_ascii_digit())
            .to_string();
    }
    (name, if parsed == 0 { 1 } else { parsed }, only_number)
}

fn parse_cast_line(line: &str) -> Map {
    let words = line.split_whitespace().collect::<Vec<_>>();
    let mut map = Map::new();
    map.insert("explicit".into(), Dynamic::from(words.len() > 1));
    map.insert(
        "skill".into(),
        Dynamic::from(if words.len() > 1 {
            words[1].to_string()
        } else {
            line.to_string()
        }),
    );
    map.insert(
        "target".into(),
        Dynamic::from(words.first().copied().unwrap_or("").to_string()),
    );
    map
}

fn player_reactions(body: &Body) -> Vec<String> {
    reaction_names(&body.get_string("반응이름"))
}

fn player_matches(body: &Body, query: &str) -> (bool, i64) {
    let name = body.get_name();
    let reactions = player_reactions(body);
    let exact = name == query || reactions.iter().any(|alias| alias == query);
    let prefix_count = if exact {
        0
    } else {
        reactions
            .iter()
            .filter(|alias| alias.starts_with(query))
            .count() as i64
    };
    (exact, prefix_count)
}

fn mob_matches(
    instance: &MobInstance,
    data: &crate::world::RawMobData,
    query: &str,
) -> (bool, i64) {
    let exact = instance.name == query || data.reaction_names.iter().any(|alias| alias == query);
    let prefix_count = if exact {
        0
    } else {
        data.reaction_names
            .iter()
            .filter(|alias| alias.starts_with(query))
            .count() as i64
    };
    (exact, prefix_count)
}

fn item_matches(item: &crate::object::Object, query: &str) -> bool {
    let (exact, prefixes) = item_match_counts(item, query);
    exact || prefixes > 0
}

fn item_match_counts(item: &crate::object::Object, query: &str) -> (bool, i64) {
    if item.getInt("투명상태") == 1 {
        return (false, 0);
    }
    let reactions = reaction_names(&item.getString("반응이름"));
    let exact = item.getName() == query || reactions.iter().any(|alias| alias == query);
    let prefixes = if exact {
        0
    } else {
        reactions
            .iter()
            .filter(|alias| alias.starts_with(query))
            .count() as i64
    };
    (exact, prefixes)
}

fn string_array(values: impl IntoIterator<Item = String>) -> rhai::Array {
    values.into_iter().map(Dynamic::from).collect()
}

fn body_target_map(body: &Body, id: &str, current_ids: &[String]) -> Map {
    let mut map = Map::new();
    map.insert("kind".into(), Dynamic::from("player"));
    map.insert("id".into(), Dynamic::from(id.to_string()));
    map.insert("name".into(), Dynamic::from(body.get_name()));
    map.insert("act".into(), Dynamic::from(body.act.to_i32() as i64));
    map.insert("mob_type".into(), Dynamic::from(0i64));
    map.insert("instance_id".into(), Dynamic::from(0_i64));
    map.insert(
        "targets".into(),
        Dynamic::from(string_array(player_target_names(body))),
    );
    map.insert(
        "current".into(),
        Dynamic::from(current_ids.iter().any(|entry| entry == id)),
    );
    map
}

fn mob_target_map(
    instance: &MobInstance,
    data: &crate::world::RawMobData,
    current_instance_ids: &[u64],
    room_index: usize,
) -> Map {
    let mut map = Map::new();
    map.insert("kind".into(), Dynamic::from("mob"));
    map.insert("id".into(), Dynamic::from(instance.mob_key.clone()));
    map.insert(
        "instance_id".into(),
        Dynamic::from(instance.instance_id as i64),
    );
    map.insert("name".into(), Dynamic::from(instance.name.clone()));
    map.insert("act".into(), Dynamic::from(instance.act as i64));
    map.insert("mob_type".into(), Dynamic::from(data.mob_type));
    let attack_forbidden = data
        .attributes
        .get("공격금지")
        .is_some_and(|value| match value {
            serde_json::Value::Null => false,
            serde_json::Value::Bool(value) => *value,
            serde_json::Value::Number(value) => value.as_f64() != Some(0.0),
            serde_json::Value::String(value) => !value.is_empty(),
            serde_json::Value::Array(value) => !value.is_empty(),
            serde_json::Value::Object(value) => !value.is_empty(),
        });
    map.insert("attack_forbidden".into(), Dynamic::from(attack_forbidden));
    map.insert("room_index".into(), Dynamic::from(room_index as i64));
    map.insert(
        "targets".into(),
        Dynamic::from(string_array(instance.targets.clone())),
    );
    map.insert(
        "current".into(),
        Dynamic::from(current_instance_ids.contains(&instance.instance_id)),
    );
    map
}

/// Resolve Python `Room.findObjName` for objects accepted by `시전.py`.
///
/// The old room stored players, mobs and items in one insertion-ordered list.
/// Rust does not yet retain that unified order.  Numeric selection is safe
/// because Python explicitly visits mobs only.  For named lookup, candidates
/// are first classified without a cross-type priority.  If more than one of
/// mob/player/item matches, the result remains unresolved.  Mob insertion
/// order is retained by `MobCache`; players are resolved only when one player
/// object matches because the clients map does not retain `room.objs` order.
pub(super) fn find_cast_target(caster: &Body, query: &str) -> Dynamic {
    let (name, order, only_number) = split_target_query(query);
    if name.is_empty() {
        return Dynamic::UNIT;
    }
    let mut current_ids = target_ids(caster);
    if let Some(target) = super::combat_commands::pvp_target(caster) {
        if !current_ids.contains(&target) {
            current_ids.push(target);
        }
    }
    let current_instance_ids = target_instance_ids(caster);
    let (zone, room) = match current_body_position(caster) {
        Some(position) => position,
        None => return Dynamic::UNIT,
    };
    let world = match get_world_state().read() {
        Ok(world) => world,
        Err(_) => return Dynamic::UNIT,
    };

    if only_number {
        let mut count = 0i64;
        let mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
        let mut ordered = world.get_room_object_order(&zone, &room);
        if ordered.is_empty() {
            ordered.extend(mobs.iter().map(|mob| RoomObjectRef::Mob(mob.instance_id)));
        }
        for object in ordered {
            let RoomObjectRef::Mob(instance_id) = object else {
                continue;
            };
            let Some((python_index, mob)) = mobs
                .iter()
                .enumerate()
                .find(|(_, mob)| mob.instance_id == instance_id)
            else {
                continue;
            };
            let room_index = mobs.len() - 1 - python_index;
            let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                continue;
            };
            if data.mob_type == 7 || mob.act == 2 || mob.act == 3 || !mob.alive {
                continue;
            }
            count += 1;
            if count == order {
                return Dynamic::from(mob_target_map(mob, data, &current_instance_ids, room_index));
            }
        }
        return Dynamic::UNIT;
    }

    // When the world has recorded Python's unified Room.objs order, resolve
    // named targets in that order before applying the legacy ambiguity guard.
    // A matching floor item is intentionally a hard stop: Python would select
    // it first, while `시전` cannot cast at an item.
    let ordered_objects = world.get_room_object_order(&zone, &room);
    if !ordered_objects.is_empty() {
        let mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
        let floor_items = world.get_room_objs(&zone, &room);
        let mut exact_count = 0i64;
        let mut prefix_count = 0i64;
        let mut reaches_order = |exact: bool, prefixes: i64| {
            if exact {
                exact_count += 1;
                exact_count == order
            } else {
                let previous = prefix_count;
                prefix_count += prefixes;
                previous < order && order <= prefix_count
            }
        };
        for object in ordered_objects {
            match object {
                RoomObjectRef::Mob(instance_id) => {
                    let Some((python_index, mob)) = mobs
                        .iter()
                        .enumerate()
                        .find(|(_, mob)| mob.instance_id == instance_id)
                    else {
                        continue;
                    };
                    let room_index = mobs.len() - 1 - python_index;
                    let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                        continue;
                    };
                    if !mob.alive || mob.act == 2 || mob.act == 3 {
                        continue;
                    }
                    let matches = mob_matches(mob, data, &name);
                    if reaches_order(matches.0, matches.1) {
                        return Dynamic::from(mob_target_map(
                            mob,
                            data,
                            &current_instance_ids,
                            room_index,
                        ));
                    }
                }
                RoomObjectRef::FloorItem(pointer) => {
                    let Some(item) = floor_items
                        .iter()
                        .find(|item| std::sync::Arc::as_ptr(item) as usize == pointer)
                    else {
                        continue;
                    };
                    if let Ok(item) = item.lock() {
                        let matches = item_match_counts(&item, &name);
                        if reaches_order(matches.0, matches.1) {
                            return Dynamic::UNIT;
                        }
                    }
                }
                RoomObjectRef::Player(player_name) => {
                    let mut matching_player: Option<&Body> = None;
                    if caster.get_name() == player_name {
                        matching_player = Some(caster);
                    } else {
                        for player_ref in other_player_refs() {
                            let player = unsafe { &*player_ref.body };
                            if player.get_name() == player_name {
                                matching_player = Some(player);
                                break;
                            }
                        }
                    }
                    let Some(player) = matching_player else {
                        continue;
                    };
                    if player.get_int("투명상태") == 1 {
                        continue;
                    }
                    let (exact, prefix_count) = player_matches(player, &name);
                    let corpse_match = name == "시체" && player.act == ActState::Death;
                    if reaches_order(corpse_match || exact, prefix_count) {
                        return Dynamic::from(body_target_map(
                            player,
                            &player.get_name(),
                            &current_ids,
                        ));
                    }
                }
                RoomObjectRef::SummonedUser(id) => {
                    let Some(user) = world.summoned_users().iter().find(|user| user.id == id)
                    else {
                        continue;
                    };
                    if user.body.get_int("투명상태") == 1 {
                        continue;
                    }
                    let (exact, prefix_count) = player_matches(&user.body, &name);
                    let corpse_match = name == "시체" && user.body.act == ActState::Death;
                    if reaches_order(corpse_match || exact, prefix_count) {
                        return Dynamic::from(body_target_map(
                            &user.body,
                            &user.body.get_name(),
                            &current_ids,
                        ));
                    }
                }
                RoomObjectRef::InstalledBox(ordinal) => {
                    let Some(boxes) = super::box_commands::installed_boxes_for_room(&zone, &room)
                    else {
                        continue;
                    };
                    let Some(box_object) = boxes.get(ordinal).cloned() else {
                        continue;
                    };
                    if let Ok(box_value) = box_object.lock() {
                        let matches = item_match_counts(&box_value, &name);
                        if reaches_order(matches.0, matches.1) {
                            return Dynamic::UNIT;
                        }
                    };
                }
                RoomObjectRef::Box(pointer) => {
                    let Some(boxes) = super::box_commands::installed_boxes_for_room(&zone, &room)
                    else {
                        continue;
                    };
                    let Some(box_object) = boxes
                        .iter()
                        .find(|object| std::sync::Arc::as_ptr(object) as usize == pointer)
                    else {
                        continue;
                    };
                    if let Ok(box_value) = box_object.lock() {
                        let matches = item_match_counts(&box_value, &name);
                        if reaches_order(matches.0, matches.1) {
                            return Dynamic::UNIT;
                        }
                    };
                }
                RoomObjectRef::Fixture(id) => {
                    let (exact, prefixes) = world
                        .get_fixture(id)
                        .map(|fixture| fixture.match_counts(&name))
                        .unwrap_or((false, 0));
                    if reaches_order(exact, prefixes as i64) {
                        return Dynamic::UNIT;
                    }
                }
            }
        }
    }

    let mut mob_exact_count = 0i64;
    let mut mob_prefix_count = 0i64;
    let mut mob_matched = false;
    let mut mob_result = None;
    for (room_index, mob) in world
        .mob_cache
        .get_all_mobs_in_room(&zone, &room)
        .iter()
        .enumerate()
    {
        let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
            continue;
        };
        if name != "시체" && (mob.act == 2 || mob.act == 3 || !mob.alive) {
            continue;
        }
        let corpse_match = name == "시체" && mob.act == 2;
        let (mut exact, prefix_count) = mob_matches(mob, data, &name);
        exact |= corpse_match;
        if exact {
            mob_matched = true;
            mob_exact_count += 1;
            if mob_result.is_none() && mob_exact_count == order {
                mob_result = Some(mob_target_map(mob, data, &current_instance_ids, room_index));
            }
        } else if prefix_count > 0 {
            mob_matched = true;
            let previous = mob_prefix_count;
            mob_prefix_count += prefix_count;
            if mob_result.is_none() && previous < order && order <= mob_prefix_count {
                mob_result = Some(mob_target_map(mob, data, &current_instance_ids, room_index));
            }
        }
    }

    // A floor item can be the object Python would encounter before a player
    // or mob.  Items are not valid cast targets, but their matching presence
    // must therefore participate in ambiguity detection.
    let mut item_matched = false;
    for item in world.get_room_objs(&zone, &room) {
        if let Ok(item) = item.lock() {
            if item_matches(&item, &name) {
                item_matched = true;
                break;
            }
        }
    }
    if !item_matched {
        for (key, count) in world.get_room_objs_stack(&zone, &room) {
            if count <= 0 {
                continue;
            }
            let Some((item, _)) = super::object_from_item_json(&key) else {
                continue;
            };
            let Ok(item) = item.lock() else {
                continue;
            };
            if item_matches(&item, &name) {
                item_matched = true;
                break;
            }
        }
    }
    drop(world);

    // The caster is a normal room object too and can be named explicitly.
    // Multiple matching players cannot be ordered faithfully because the
    // connected-client map is not Python's insertion-ordered `room.objs`.
    let mut matching_players = Vec::new();
    if caster.get_int("투명상태") != 1 {
        let (exact, prefix_count) = player_matches(caster, &name);
        let corpse_match = name == "시체" && caster.act == ActState::Death;
        if corpse_match || exact || prefix_count > 0 {
            matching_players.push((caster, corpse_match || exact, prefix_count));
        }
    }

    for player_ref in other_player_refs() {
        // SAFETY: see `CastRoomPlayerRef`; the clients mutex remains held.
        let player = unsafe { &*player_ref.body };
        if player.get_int("투명상태") == 1 {
            continue;
        }
        let (exact, prefix_count) = player_matches(player, &name);
        let corpse_match = name == "시체" && player.act == ActState::Death;
        if corpse_match || exact || prefix_count > 0 {
            matching_players.push((player, corpse_match || exact, prefix_count));
        }
    }

    let player_matched = !matching_players.is_empty();
    let matched_kinds =
        i32::from(mob_matched) + i32::from(player_matched) + i32::from(item_matched);
    if matched_kinds != 1 {
        return Dynamic::UNIT;
    }
    if mob_matched {
        return mob_result.map_or(Dynamic::UNIT, Dynamic::from);
    }
    if item_matched || matching_players.len() != 1 {
        return Dynamic::UNIT;
    }

    let (player, exact, prefix_count) = matching_players[0];
    if (exact && order == 1) || (!exact && order <= prefix_count) {
        Dynamic::from(body_target_map(player, &player.get_name(), &current_ids))
    } else {
        Dynamic::UNIT
    }
}

fn current_cast_target(caster: &Body) -> Dynamic {
    let ids = target_ids(caster);
    let instance_ids = target_instance_ids(caster);
    let Some(id) = ids.first() else {
        return Dynamic::UNIT;
    };
    let (zone, room) = match current_body_position(caster) {
        Some(position) => position,
        None => return Dynamic::UNIT,
    };
    if let Ok(world) = get_world_state().read() {
        for (room_index, mob) in world
            .mob_cache
            .get_all_mobs_in_room(&zone, &room)
            .iter()
            .enumerate()
        {
            if (instance_ids
                .first()
                .is_some_and(|instance_id| *instance_id == mob.instance_id)
                || (instance_ids.is_empty() && (&mob.mob_key == id || &mob.name == id)))
                && mob.alive
                && mob.act != 2
                && mob.act != 3
            {
                if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
                    return Dynamic::from(mob_target_map(mob, data, &instance_ids, room_index));
                }
            }
        }
    }
    if id == &caster.get_name() {
        return Dynamic::from(body_target_map(caster, id, &ids));
    }
    for player_ref in other_player_refs() {
        let player = unsafe { &*player_ref.body };
        if id == &player.get_name() && player.act != ActState::Death {
            return Dynamic::from(body_target_map(player, id, &ids));
        }
    }
    Dynamic::UNIT
}

fn effect_conflicts(name: &str, category: &str, effect_name: &str, anti_type: &str) -> bool {
    name == effect_name || category == anti_type
}

fn body_has_conflict(body: &Body, skill: &Skill) -> bool {
    body.active_skills.iter().any(|effect| {
        effect_conflicts(
            &skill.name,
            &skill.category,
            &effect.name,
            &effect.anti_type,
        )
    })
}

fn mob_has_conflict(mob: &MobInstance, skill: &Skill) -> bool {
    mob.skill_effects.iter().any(|effect| {
        effect_conflicts(
            &skill.name,
            &skill.category,
            &effect.name,
            &effect.anti_type,
        )
    })
}

fn active_skill(skill: &Skill, duration: i64) -> ActiveSkill {
    let mut effect = ActiveSkill::new(skill.name.clone(), duration as i32);
    effect.str_bonus = skill.str_bonus as i32;
    effect.dex_bonus = skill.dex_bonus as i32;
    effect.arm_bonus = skill.arm_bonus as i32;
    effect.mp_bonus = skill.mp_bonus as i32;
    effect.max_mp_bonus = skill.max_mp_bonus as i32;
    effect.hp_bonus = skill.hp_bonus as i32;
    effect.max_hp_bonus = skill.max_hp_bonus as i32;
    effect.anti_type = skill.deny.clone();
    effect.category = skill.category.clone();
    effect.recovery_percent = skill.recovery_percent;
    effect.recovery_script = skill.recovery_script.clone();
    effect.release_script = skill.release_script.clone();
    effect
}

fn mob_skill_effect(skill: &Skill, duration: i64) -> MobSkillEffect {
    MobSkillEffect {
        name: skill.name.clone(),
        anti_type: skill.deny.clone(),
        expires_at: chrono::Utc::now().timestamp() + duration,
        str_bonus: skill.str_bonus,
        dex_bonus: skill.dex_bonus,
        arm_bonus: skill.arm_bonus,
        mp_bonus: skill.mp_bonus,
        max_mp_bonus: skill.max_mp_bonus,
        hp_bonus: skill.hp_bonus,
        max_hp_bonus: skill.max_hp_bonus,
    }
}

fn apply_body_modifiers(body: &mut Body, skill: &Skill) {
    body._str += skill.str_bonus as i32;
    body._dex += skill.dex_bonus as i32;
    body._arm += skill.arm_bonus as i32;
    body._mp += skill.mp_bonus as i32;
    body._maxmp += skill.max_mp_bonus as i32;
    body._hp += skill.hp_bonus as i32;
    body._maxhp += skill.max_hp_bonus as i32;
}

fn apply_mob_modifiers(mob: &mut MobInstance, skill: &Skill) {
    mob.str_modifier += skill.str_bonus;
    mob.dex_modifier += skill.dex_bonus;
    mob.arm_modifier += skill.arm_bonus;
    mob.mp_modifier += skill.mp_bonus;
    mob.max_mp_modifier += skill.max_mp_bonus;
    mob.hp_modifier += skill.hp_bonus;
    mob.max_hp_modifier += skill.max_hp_bonus;
}

fn skill_rank(name: &str) -> i64 {
    let Ok(content) = std::fs::read_to_string("data/config/murim.json") else {
        return 1;
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) else {
        return 1;
    };
    let Some(config) = root.get("메인설정") else {
        return 1;
    };
    for (index, rank) in ["초급", "중급", "상급", "고급", "특급", "절정", "초절정"]
        .iter()
        .enumerate()
    {
        let key = format!("{}무공", rank);
        if config
            .get(&key)
            .and_then(serde_json::Value::as_array)
            .is_some_and(|skills| skills.iter().any(|skill| skill.as_str() == Some(name)))
        {
            return index as i64 + 1;
        }
    }
    1
}

fn rank_number(name: &str) -> i64 {
    match name {
        "초급" => 1,
        "중급" => 2,
        "상급" => 3,
        "고급" => 4,
        "특급" => 5,
        "절정" => 6,
        "초절정" => 7,
        _ => 1,
    }
}

pub(crate) fn skill_up_python(body: &mut Body, skill: &Skill) -> (i64, bool, bool) {
    let current = body
        .skill_map
        .get(&skill.name)
        .copied()
        .unwrap_or_else(|| SkillTraining::new(1, 0));
    let mut level = current.level as i64;
    let mut exp = current.exp as i64 + 1;
    let required_rank = skill_rank(&skill.name);
    let exp1 = 10_000 * required_rank;
    // Python compares `종류` with the literal "공격". Current data uses
    // "전투"/"방어"/..., therefore every shipped skill takes this /10 path.
    let divisor = 10;
    let mut gated_up = false;
    if (level == 10 && exp > exp1 / divisor) || (level == 11 && exp > exp1 * 2 / divisor) {
        let mut achieved = body.get_string("무공달성레벨");
        if achieved.is_empty() {
            achieved = "초급".to_string();
            body.set("무공달성레벨", achieved.as_str());
        }
        if required_rank <= rank_number(&achieved) {
            gated_up = true;
        }
    }

    let mut leveled = false;
    let mut achievement_up = false;
    if (level < 10 && exp >= skill.prob_increase) || gated_up {
        level += 1;
        exp = 0;
        if level <= 12 {
            leveled = true;
            if level == 12 {
                let achieved = body.get_string("무공달성레벨");
                let achieved_number = rank_number(&achieved);
                // Python calls checkSkillLvUp before writing the newly raised
                // `(s1, s2)` pair back to skillMap, so the current skill still
                // has its old level during this count.
                let completed = body
                    .skill_map
                    .iter()
                    .filter(|(name, training)| {
                        skill_rank(name) == achieved_number && training.level == 12
                    })
                    .count();
                if completed >= 12 {
                    let names = ["초급", "중급", "상급", "고급", "특급", "절정", "초절정"];
                    if let Some(next) = names.get(achieved_number as usize) {
                        body.set("무공달성레벨", *next);
                        achievement_up = true;
                    }
                }
            }
        } else {
            // Python returns before writing the over-12 value.
            return (current.level as i64, false, false);
        }
    }
    body.skill_map
        .insert(skill.name.clone(), SkillTraining::new(level, exp as u32));
    (level, leveled, achievement_up)
}

fn discounted_mp_cost(body: &Body, skill: &Skill) -> i64 {
    match body
        .skill_map
        .get(&skill.name)
        .map(|training| training.level)
    {
        Some(11) => skill.mp_cost * 9 / 10,
        Some(12) => skill.mp_cost * 8 / 10,
        _ => skill.mp_cost,
    }
}

fn base_result(status: &str) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from(status.to_string()));
    result.insert("plus".into(), Dynamic::from(0i64));
    result.insert("level_up".into(), Dynamic::from(false));
    result.insert("achievement_up".into(), Dynamic::from(false));
    result
}

fn check_and_pay_cost(body: &mut Body, skill: &Skill) -> Result<(), &'static str> {
    if body.get_mp() < skill.mp_cost {
        return Err("mp");
    }
    let hp = body.get_hp();
    let base_max_hp = body.get_int("최고체력");
    if hp < base_max_hp * skill.hp_cost / 100 || hp < base_max_hp * skill.hp_requirement / 100 {
        return Err("hp");
    }
    body.set(
        "내공",
        body.get_int("내공") - discounted_mp_cost(body, skill),
    );
    body.set(
        "체력",
        body.get_int("체력") - base_max_hp * skill.hp_cost / 100,
    );
    Ok(())
}

fn attack_chance_against_body(caster: &Body, target: &Body) -> f64 {
    let level_delta = target.get_int("레벨") - caster.get_int("레벨");
    if level_delta >= 400 {
        return -1.0;
    }
    100.0 - (level_delta + 90).div_euclid(3) as f64 + caster.get_hit() as f64 * 0.2
        - target.get_miss() as f64 * 0.2
}

fn attack_chance_against_mob(
    caster: &Body,
    mob: &MobInstance,
    data: &crate::world::RawMobData,
) -> f64 {
    let level_delta = mob.level - caster.get_int("레벨");
    if level_delta >= 400 {
        return -1.0;
    }
    100.0 - (level_delta + 90).div_euclid(3) as f64 + caster.get_hit() as f64 * 0.2
        - data.miss as f64 * 0.2
}

fn duration(skill: &Skill, level: i64) -> i64 {
    skill.defense_time + skill.defense_time_increase * (level - 1)
}

fn python_floor_div(value: i64, divisor: i64) -> i64 {
    let quotient = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && ((remainder < 0) != (divisor < 0)) {
        quotient - 1
    } else {
        quotient
    }
}

fn finish_noncombat_on_body(
    caster: &mut Body,
    target: &mut Body,
    skill: &Skill,
    level: i64,
) -> i64 {
    apply_body_modifiers(target, skill);
    let effect_duration = duration(skill, level);
    let Some(against_name) = skill.against_skill.as_deref() else {
        target
            .active_skills
            .push(active_skill(skill, effect_duration));
        return 0;
    };
    let Some(against) = get_skill(against_name) else {
        caster
            .active_skills
            .push(active_skill(skill, effect_duration));
        return 0;
    };
    let mut plus = 0;
    match skill.category.as_str() {
        "내공흡수" if target.get_mp() > 0 => {
            if attack_chance_against_body(caster, target) >= fastrand::i32(0..=100) as f64 {
                plus = -python_floor_div(target.get_int("내공") * against.mp_bonus, 100);
                plus = plus.min(caster.get_int("최고내공") - caster.get_int("내공"));
                plus = plus.max(0);
                caster.set("내공", caster.get_int("내공") + plus);
                target.set("내공", target.get_int("내공") - plus);
            }
        }
        "내공감소" => {
            target._mp += against.mp_bonus as i32;
            target._maxmp += against.max_mp_bonus as i32;
            target
                .active_skills
                .push(active_skill(&against, duration(&against, level)));
        }
        "체력흡수" if target.get_hp() > 0 => {
            if attack_chance_against_body(caster, target) >= fastrand::i32(0..=100) as f64 {
                plus = -python_floor_div(target.get_int("체력") * against.hp_bonus, 100);
                plus = plus.min(caster.get_int("최고체력") - caster.get_int("체력"));
                plus = plus.max(0);
                caster.set("체력", caster.get_int("체력") + plus);
                target.set("체력", target.get_int("체력") - plus);
            }
        }
        "체력감소" => {
            target._hp += against.hp_bonus as i32;
            target._maxhp += against.max_hp_bonus as i32;
            target
                .active_skills
                .push(active_skill(&against, duration(&against, level)));
        }
        _ => {}
    }
    caster
        .active_skills
        .push(active_skill(skill, effect_duration));
    plus
}

fn finish_noncombat_on_mob(
    caster: &mut Body,
    mob: &mut MobInstance,
    data: &crate::world::RawMobData,
    skill: &Skill,
    level: i64,
) -> i64 {
    apply_mob_modifiers(mob, skill);
    let effect_duration = duration(skill, level);
    let Some(against_name) = skill.against_skill.as_deref() else {
        mob.skills.push(skill.name.clone());
        mob.skill_effects
            .push(mob_skill_effect(skill, effect_duration));
        return 0;
    };
    let Some(against) = get_skill(against_name) else {
        caster
            .active_skills
            .push(active_skill(skill, effect_duration));
        return 0;
    };
    let mut plus = 0;
    match skill.category.as_str() {
        "내공흡수" if mob.mp > 0 => {
            if attack_chance_against_mob(caster, mob, data) >= fastrand::i32(0..=100) as f64 {
                plus = -python_floor_div(mob.mp * against.mp_bonus, 100);
                plus = plus.min(caster.get_int("최고내공") - caster.get_int("내공"));
                plus = plus.max(0);
                caster.set("내공", caster.get_int("내공") + plus);
                mob.mp -= plus;
            }
        }
        "내공감소" => {
            mob.mp_modifier += against.mp_bonus;
            mob.max_mp_modifier += against.max_mp_bonus;
            mob.skills.push(against.name.clone());
            mob.skill_effects
                .push(mob_skill_effect(&against, duration(&against, level)));
        }
        "체력흡수" if mob.hp > 0 => {
            if attack_chance_against_mob(caster, mob, data) >= fastrand::i32(0..=100) as f64 {
                plus = -python_floor_div(mob.hp * against.hp_bonus, 100);
                plus = plus.min(caster.get_int("최고체력") - caster.get_int("체력"));
                plus = plus.max(0);
                caster.set("체력", caster.get_int("체력") + plus);
                mob.hp -= plus;
            }
        }
        "체력감소" => {
            mob.hp_modifier += against.hp_bonus;
            mob.max_hp_modifier += against.max_hp_bonus;
            mob.skills.push(against.name.clone());
            mob.skill_effects
                .push(mob_skill_effect(&against, duration(&against, level)));
        }
        _ => {}
    }
    caster
        .active_skills
        .push(active_skill(skill, effect_duration));
    plus
}

fn apply_noncombat(caster: &mut Body, skill_name: &str, kind: &str, id: &str) -> Map {
    let Some(skill) = get_skill(skill_name) else {
        return base_result("missing_skill");
    };
    if body_has_conflict(caster, &skill) {
        return base_result("duplicate");
    }

    if kind == "player" {
        if id == caster.get_name() {
            if body_has_conflict(caster, &skill) {
                return base_result("duplicate");
            }
            if let Err(status) = check_and_pay_cost(caster, &skill) {
                return base_result(status);
            }
            let (level, leveled, achievement_up) = skill_up_python(caster, &skill);
            // Avoid two mutable references to the same Body while retaining the
            // exact self-target state transitions.
            apply_body_modifiers(caster, &skill);
            let effect_duration = duration(&skill, level);
            caster
                .active_skills
                .push(active_skill(&skill, effect_duration));
            let mut result = base_result("ok");
            result.insert("level_up".into(), Dynamic::from(leveled));
            result.insert("achievement_up".into(), Dynamic::from(achievement_up));
            return result;
        }
        let Some(player_ref) = other_player_refs()
            .into_iter()
            .find(|player_ref| unsafe { (&*player_ref.body).get_name() == id })
        else {
            return base_result("missing_target");
        };
        let target = unsafe { &mut *player_ref.body };
        if body_has_conflict(target, &skill) {
            return base_result("duplicate");
        }
        if let Err(status) = check_and_pay_cost(caster, &skill) {
            return base_result(status);
        }
        let (level, leveled, achievement_up) = skill_up_python(caster, &skill);
        let plus = finish_noncombat_on_body(caster, target, &skill, level);
        let mut result = base_result("ok");
        result.insert("plus".into(), Dynamic::from(plus));
        result.insert("level_up".into(), Dynamic::from(leveled));
        result.insert("achievement_up".into(), Dynamic::from(achievement_up));
        return result;
    }

    if kind != "mob" {
        return base_result("missing_target");
    }
    let Some((zone, room)) = current_body_position(caster) else {
        return base_result("missing_target");
    };
    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return base_result("missing_target"),
    };
    let Some(data) = world.mob_cache.get_mob(id).cloned() else {
        return base_result("missing_target");
    };
    let Some(mob) = world.mob_cache.get_mob_instance_mut(&zone, &room, id) else {
        return base_result("missing_target");
    };
    if mob_has_conflict(mob, &skill) {
        return base_result("duplicate");
    }
    if let Err(status) = check_and_pay_cost(caster, &skill) {
        return base_result(status);
    }
    let (level, leveled, achievement_up) = skill_up_python(caster, &skill);
    let plus = finish_noncombat_on_mob(caster, mob, &data, &skill, level);
    let mut result = base_result("ok");
    result.insert("plus".into(), Dynamic::from(plus));
    result.insert("level_up".into(), Dynamic::from(leveled));
    result.insert("achievement_up".into(), Dynamic::from(achievement_up));
    result
}

fn combat_start_event(instance: &MobInstance, data: &crate::world::RawMobData) -> Dynamic {
    let mut event = Map::new();
    event.insert("name".into(), Dynamic::from(instance.name.clone()));
    event.insert(
        "script".into(),
        Dynamic::from(data.combat_start_script.clone()),
    );
    Dynamic::from(event)
}

fn apply_combat(
    caster: &mut Body,
    skill_name: &str,
    target_id: &str,
    target_instance_id: i64,
) -> Map {
    let Some(skill) = get_skill(skill_name) else {
        return base_result("missing_skill");
    };
    if caster.skill.is_some() {
        return base_result("busy");
    }
    if let Some(result) = with_room_player_body_mut(target_id, |target| {
        if target.act == ActState::Death || target.get_int("투명상태") == 1 {
            return base_result("missing_target");
        }
        if super::combat_commands::pvp_target(target).is_some_and(|name| name != caster.get_name())
            || !target_ids(target).is_empty()
        {
            return base_result("target_busy");
        }
        if let Err(status) = check_and_pay_cost(caster, &skill) {
            return base_result(status);
        }
        let caster_was_stand = caster.act == ActState::Stand;
        let target_was_stand = target.act == ActState::Stand;
        let new_target = super::combat_commands::pvp_target(caster).as_deref() != Some(target_id);
        caster.get_skill(skill_name);
        for key in ["_skill_end", "_skill_step", "_skill_turn"] {
            caster.temp_mut().insert(key.to_string(), Value::Int(0));
        }
        caster.set("힘경험치", caster.get_int("힘경험치") + skill.bonus);
        caster.temp_mut().insert(
            super::combat_commands::PVP_TARGET.to_string(),
            Value::String(target_id.to_string()),
        );
        target.temp_mut().insert(
            super::combat_commands::PVP_TARGET.to_string(),
            Value::String(caster.get_name()),
        );
        caster.act = ActState::Fight;
        target.act = ActState::Fight;
        if new_target {
            caster.dex = 0;
            target.dex = 0;
        }
        let mut result = base_result("ok");
        result.insert("new_target".into(), Dynamic::from(new_target));
        result.insert("caster_was_stand".into(), Dynamic::from(caster_was_stand));
        result.insert("target_was_stand".into(), Dynamic::from(target_was_stand));
        result.insert("target_name".into(), Dynamic::from(target_id.to_string()));
        result.insert("target_start".into(), Dynamic::from(String::new()));
        result.insert("linked".into(), Dynamic::from(rhai::Array::new()));
        result.insert("advance".into(), Dynamic::from(caster.get_dex() >= 4200));
        result
    }) {
        return result;
    }
    let Some((zone, room)) = current_body_position(caster) else {
        return base_result("missing_target");
    };
    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return base_result("missing_target"),
    };
    let metadata = world
        .mob_cache
        .get_all_mobs_in_room(&zone, &room)
        .into_iter()
        .filter_map(|mob| {
            world
                .mob_cache
                .get_mob(&mob.mob_key)
                .map(|data| (mob.mob_key.clone(), data.clone()))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let Some(instances) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return base_result("missing_target");
    };
    let Some(target_index) = instances.iter().position(|mob| {
        mob.alive
            && if target_instance_id > 0 {
                mob.instance_id == target_instance_id as u64
            } else {
                mob.mob_key == target_id
            }
    }) else {
        return base_result("missing_target");
    };
    if let Err(status) = check_and_pay_cost(caster, &skill) {
        return base_result(status);
    }

    let caster_was_stand = caster.act == ActState::Stand;
    let target_was_stand = instances[target_index].act == 0;
    let selected_instance_id = instances[target_index].instance_id;
    let new_target = !target_instance_ids(caster).contains(&selected_instance_id);

    caster.get_skill(skill_name);
    for key in ["_skill_end", "_skill_step", "_skill_turn"] {
        caster.temp_mut().insert(key.to_string(), Value::Int(0));
    }
    caster.set("힘경험치", caster.get_int("힘경험치") + skill.bonus);

    let target_name = instances[target_index].name.clone();
    let target_script = metadata
        .get(target_id)
        .map(|data| data.combat_start_script.clone())
        .unwrap_or_default();
    let mut linked_events = rhai::Array::new();
    if new_target {
        add_target_id(caster, target_id);
        super::combat_commands::add_target_instance_id(caster, selected_instance_id);
        caster.temp_mut().insert(
            "_attack_target_key".to_string(),
            Value::String(target_id.to_string()),
        );
        caster.temp_mut().insert(
            "_attack_target".to_string(),
            Value::String(target_name.clone()),
        );
        if !instances[target_index]
            .targets
            .iter()
            .any(|name| name == &caster.get_name())
        {
            instances[target_index].targets.push(caster.get_name());
        }
        instances[target_index].act = 1;

        for (index, mob) in instances.iter_mut().enumerate() {
            if index == target_index || !mob.alive || mob.act != 0 {
                continue;
            }
            let Some(data) = metadata.get(&mob.mob_key) else {
                continue;
            };
            if data.combat_type != 1 && data.combat_type != 2 {
                continue;
            }
            add_target_id(caster, &mob.mob_key);
            super::combat_commands::add_target_instance_id(caster, mob.instance_id);
            if !mob.targets.iter().any(|name| name == &caster.get_name()) {
                mob.targets.push(caster.get_name());
            }
            linked_events.push(combat_start_event(mob, data));
            mob.act = 1;
        }
    }
    caster.act = ActState::Fight;
    let advance = caster.get_dex() >= 4200;
    if advance {
        // Python sets this flag and then `doFight(True)` immediately returns
        // because its first guard sees the same flag. Preserve that observable
        // state instead of inventing an extra attack round.
        caster
            .temp_mut()
            .insert("_advance".to_string(), Value::Int(1));
    }

    let mut result = base_result("ok");
    result.insert("new_target".into(), Dynamic::from(new_target));
    result.insert("caster_was_stand".into(), Dynamic::from(caster_was_stand));
    result.insert("target_was_stand".into(), Dynamic::from(target_was_stand));
    result.insert("target_name".into(), Dynamic::from(target_name));
    result.insert("target_start".into(), Dynamic::from(target_script));
    result.insert("linked".into(), Dynamic::from(linked_events));
    result.insert("advance".into(), Dynamic::from(advance));
    result
}

fn weapon_view(body: &Body) -> Map {
    let mut map = Map::new();
    if let Some(weapon) = body
        .weapon_item
        .as_ref()
        .and_then(|weapon| weapon.upgrade())
    {
        if let Ok(weapon) = weapon.lock() {
            map.insert("name".into(), Dynamic::from(weapon.getName()));
            map.insert("ansi".into(), Dynamic::from(weapon.getString("안시")));
            map.insert(
                "fight_start".into(),
                Dynamic::from(weapon.getString("전투시작")),
            );
            map.insert("fist".into(), Dynamic::from(false));
            return map;
        }
    }
    map.insert("name".into(), Dynamic::from("주먹"));
    map.insert("ansi".into(), Dynamic::from(""));
    map.insert(
        "fight_start".into(),
        Dynamic::from("주먹을 쥐며 공격 합니다."),
    );
    map.insert("fist".into(), Dynamic::from(true));
    map
}

fn cast_room_players(caster: &Body) -> rhai::Array {
    let mut players = rhai::Array::new();
    let mut own = Map::new();
    own.insert("name".into(), Dynamic::from(caster.get_name()));
    own.insert(
        "reject_fight".into(),
        Dynamic::from(super::config_is_enabled(
            &caster.get_string("설정상태"),
            "타인전투출력거부",
        )),
    );
    own.insert("show_prompt".into(), Dynamic::from(false));
    players.push(Dynamic::from(own));
    for player_ref in other_player_refs() {
        let player = unsafe { &*player_ref.body };
        let mut map = Map::new();
        map.insert("name".into(), Dynamic::from(player.get_name()));
        map.insert(
            "reject_fight".into(),
            Dynamic::from(super::config_is_enabled(
                &player.get_string("설정상태"),
                "타인전투출력거부",
            )),
        );
        map.insert(
            "show_prompt".into(),
            Dynamic::from(
                player_ref.interactive == 1
                    && !super::config_is_enabled(&player.get_string("설정상태"), "엘피출력"),
            ),
        );
        map.insert("hp".into(), Dynamic::from(player.get_hp()));
        map.insert("max_hp".into(), Dynamic::from(player.get_max_hp()));
        map.insert("mp".into(), Dynamic::from(player.get_mp()));
        map.insert("max_mp".into(), Dynamic::from(player.get_max_mp()));
        players.push(Dynamic::from(map));
    }
    players
}

fn cast_room_has_attr(caster: &Body, key: &str) -> bool {
    super::combat_commands::room_has_attr(caster, key)
}

pub(super) fn register_cast_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    engine.register_fn("parse_cast_line", parse_cast_line);
    let ptr = body_ptr;
    engine.register_fn("find_cast_target", move |_ob: &mut Map, query: &str| {
        find_cast_target(unsafe { &*ptr }, query)
    });
    let ptr = body_ptr;
    engine.register_fn("get_cast_current_target", move |_ob: &mut Map| {
        current_cast_target(unsafe { &*ptr })
    });
    let ptr = body_ptr;
    engine.register_fn("get_cast_self_target", move |_ob: &mut Map| -> Map {
        let body = unsafe { &*ptr };
        body_target_map(body, &body.get_name(), &target_ids(body))
    });
    let ptr = body_ptr;
    engine.register_fn("cast_has_current_skill", move |_ob: &mut Map| -> bool {
        unsafe { (&*ptr).skill.is_some() }
    });
    let ptr = body_ptr;
    engine.register_fn(
        "cast_apply_noncombat",
        move |_ob: &mut Map, skill: &str, kind: &str, id: &str| -> Map {
            apply_noncombat(unsafe { &mut *ptr }, skill, kind, id)
        },
    );
    let ptr = body_ptr;
    engine.register_fn(
        "cast_apply_combat",
        move |_ob: &mut Map, skill: &str, target_id: &str, instance_id: i64| -> Map {
            apply_combat(unsafe { &mut *ptr }, skill, target_id, instance_id)
        },
    );
    let ptr = body_ptr;
    engine.register_fn("get_cast_weapon", move |_ob: &mut Map| -> Map {
        weapon_view(unsafe { &*ptr })
    });
    let ptr = body_ptr;
    engine.register_fn(
        "get_cast_room_players",
        move |_ob: &mut Map| -> rhai::Array { cast_room_players(unsafe { &*ptr }) },
    );
    let ptr = body_ptr;
    engine.register_fn(
        "cast_room_has_attr",
        move |_ob: &mut Map, key: &str| -> bool { cast_room_has_attr(unsafe { &*ptr }, key) },
    );
    engine.register_fn("post_position_once", crate::hangul::post_position1);
    engine.register_fn(
        "cast_replace",
        |value: &str, from: &str, to: &str| -> String { value.replace(from, to) },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptStorage;
    use crate::world::PlayerPosition;

    #[test]
    fn python_skill_up_uses_probability_increase_and_no_output_side_effect() {
        let skill = get_skill("금강불괴").expect("skill fixture");
        let mut body = Body::new();
        body.set("이름", "숙련검사");
        body.skill_map
            .insert(skill.name.clone(), SkillTraining::new(1, 99));
        let (level, leveled, achievement) = skill_up_python(&mut body, &skill);
        assert_eq!(level, 2);
        assert!(leveled);
        assert!(!achievement);
        assert_eq!(body.skill_map[&skill.name], SkillTraining::new(2, 0));
    }

    #[test]
    fn target_query_keeps_python_numeric_and_dot_rules() {
        assert_eq!(split_target_query("."), ("1".to_string(), 1, true));
        assert_eq!(
            split_target_query("2왕 extra"),
            ("왕".to_string(), 2, false)
        );
        assert_eq!(split_target_query("왕"), ("왕".to_string(), 1, false));
        assert_eq!(python_floor_div(-40, 100), -1);
        assert_eq!(python_floor_div(-500, 100), -5);
    }

    #[test]
    fn numeric_target_uses_integrated_room_order_for_same_template_clones() {
        let caster_name = "동일몹숫자선택검사";
        let zone = "동일몹숫자선택존";
        let room = "1";
        {
            let mut world = get_world_state().write().unwrap();
            world.spawn_mob_at("낙양성:12", zone, room).unwrap();
            world.spawn_mob_at("낙양성:12", zone, room).unwrap();
            world.set_player_position(
                caster_name,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
        }
        let mut caster = Body::new();
        caster.set("이름", caster_name);
        let first = find_cast_target(&caster, "1").cast::<Map>();
        let second = find_cast_target(&caster, "2").cast::<Map>();
        assert_eq!(first["room_index"].as_int().unwrap(), 1);
        assert_eq!(second["room_index"].as_int().unwrap(), 0);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(caster_name);
        world.mob_cache.remove_instance(zone, room, "낙양성:12");
        world.mob_cache.remove_instance(zone, room, "낙양성:12");
    }

    #[test]
    fn absorption_attack_chance_uses_python_floor_for_negative_level_delta() {
        let mut caster = Body::new();
        caster.set("이름", "흡수확률시전자");
        caster.set("레벨", 200i64);
        caster.set("명중", 0i64);

        let mut player = Body::new();
        player.set("이름", "흡수확률대상자");
        player.set("레벨", 100i64);
        player.set("회피", 0i64);
        // Python: (-100 + 90) // 3 == -4, so 100 - (-4) == 104.
        assert_eq!(attack_chance_against_body(&caster, &player), 104.0);

        let mut data = crate::world::RawMobData::new();
        data.name = "흡수확률대상몹".to_string();
        data.level = 100;
        data.miss = 0;
        let mob = MobInstance::new(
            "흡수확률검사:1".to_string(),
            "흡수확률검사".to_string(),
            "1",
            &data,
        );
        assert_eq!(attack_chance_against_mob(&caster, &mob, &data), 104.0);
    }

    #[test]
    fn named_target_does_not_invent_mob_player_or_floor_item_priority() {
        let caster_name = "시전교차종류시전자";
        let zone = "시전교차종류검사존";
        let room = "1";
        let mob_name = {
            let mut world = get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("절강성", "7")
                .expect("repository mob fixture");
            world.mob_cache.add_mob_instance(MobInstance::new(
                "절강성:7".to_string(),
                zone.to_string(),
                room,
                &data,
            ));
            world.set_player_position(
                caster_name,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            data.name
        };
        let mut caster = Body::new();
        caster.set("이름", caster_name);

        let target = find_cast_target(&caster, &mob_name);
        assert!(!target.is_unit(), "a sole mob candidate is resolvable");
        assert_eq!(
            target.cast::<Map>()["kind"].clone().into_string().unwrap(),
            "mob"
        );

        // Python may encounter this item before the mob in its unified
        // room.objs. Rust has no corresponding cross-type insertion order.
        let mut exact_item = crate::object::Object::new();
        exact_item.set("이름", mob_name.as_str());
        get_world_state()
            .write()
            .unwrap()
            .get_room_objs_mut(zone, room)
            .push(std::sync::Arc::new(std::sync::Mutex::new(exact_item)));
        assert!(find_cast_target(&caster, &mob_name).is_unit());

        {
            let mut world = get_world_state().write().unwrap();
            world.get_room_objs_mut(zone, room).clear();
            let mut reaction_item = crate::object::Object::new();
            reaction_item.set("이름", "교차충돌바닥물건");
            reaction_item.set("반응이름", mob_name.as_str());
            world
                .get_room_objs_mut(zone, room)
                .push(std::sync::Arc::new(std::sync::Mutex::new(reaction_item)));
        }
        assert!(find_cast_target(&caster, &mob_name).is_unit());

        get_world_state()
            .write()
            .unwrap()
            .get_room_objs_mut(zone, room)
            .clear();
        let mut player = Body::new();
        player.set("이름", "시전교차종류대상자");
        player.set("반응이름", mob_name.as_str());
        set_cast_room_players(vec![CastRoomPlayerRef::new(&mut player)]);
        assert!(find_cast_target(&caster, &mob_name).is_unit());

        clear_cast_room_players();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(caster_name);
    }

    #[test]
    fn named_target_honors_runtime_box_and_mob_integrated_order() {
        let caster_name = "시전상자순서시전자";
        let zone = "시전상자순서존";
        let room = "1";
        let (mob_name, mob_id) = {
            let mut world = get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("절강성", "7")
                .expect("repository mob fixture");
            let mob = MobInstance::new("절강성:7".to_string(), zone.to_string(), room, &data);
            let id = mob.instance_id;
            world.mob_cache.add_mob_instance(mob);
            world.record_test_room_object(zone, room, RoomObjectRef::Mob(id));
            world.set_player_position(
                caster_name,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            (data.name, id)
        };
        let runtime_box = std::sync::Arc::new(std::sync::Mutex::new(crate::object::Object::new()));
        runtime_box.lock().unwrap().set("이름", mob_name.as_str());
        super::super::box_commands::register_installed_box(zone, room, runtime_box);
        let mut caster = Body::new();
        caster.set("이름", caster_name);

        assert!(
            find_cast_target(&caster, &mob_name).is_unit(),
            "Python selects the newer same-name runtime Box before 시전 checks target type"
        );
        get_world_state().write().unwrap().record_test_room_object(
            zone,
            room,
            RoomObjectRef::Mob(mob_id),
        );
        let selected = find_cast_target(&caster, &mob_name).cast::<Map>();
        assert_eq!(selected["kind"].clone().into_string().unwrap(), "mob");
        assert_eq!(selected["instance_id"].as_int().unwrap(), mob_id as i64);

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(caster_name);
    }

    #[test]
    fn named_target_uses_only_provable_single_type_order() {
        let caster_name = "시전단일종류시전자";
        let zone = "시전단일종류검사존";
        let room = "1";
        let mob_name = {
            let mut world = get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("절강성", "7")
                .expect("repository mob fixture");
            for _ in 0..2 {
                world.mob_cache.add_mob_instance(MobInstance::new(
                    "절강성:7".to_string(),
                    zone.to_string(),
                    room,
                    &data,
                ));
            }
            world.set_player_position(
                caster_name,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            data.name
        };
        let mut caster = Body::new();
        caster.set("이름", caster_name);

        // MobCache retains its Vec insertion order, so an ordinal among mobs
        // is the only named multi-object order that can be reproduced here.
        assert!(!find_cast_target(&caster, &format!("2{mob_name}")).is_unit());

        let mut first = Body::new();
        first.set("이름", "시전순서미상대상일");
        first.set("반응이름", "순서미상");
        let mut second = Body::new();
        second.set("이름", "시전순서미상대상이");
        second.set("반응이름", "순서미상");
        set_cast_room_players(vec![
            CastRoomPlayerRef::new(&mut first),
            CastRoomPlayerRef::new(&mut second),
        ]);
        assert!(find_cast_target(&caster, "순서미상").is_unit());

        clear_cast_room_players();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(caster_name);
    }

    #[test]
    fn rhai_cast_usage_and_rest_messages_match_python() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "시전문구검사");

        let (output, special) = storage
            .execute("시전", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 사용법: [대상|무공이름] 시전"]);
        assert!(special.is_none());

        body.act = ActState::Rest;
        let (output, special) = storage
            .execute("시전", &mut body, "금강불괴", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 운기조식중엔 무공을 사용할 수 없습니다."]);
        assert!(special.is_none());
    }

    #[test]
    fn rhai_self_defense_cast_applies_python_cost_training_and_effects() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "방어시전검사");
        body.set("내공", 1_000i64);
        body.set("최고내공", 1_000i64);
        body.set("체력", 1_000i64);
        body.set("최고체력", 1_000i64);
        body.skill_list.push("금강불괴".to_string());

        let (output, special) = storage
            .execute("시전", &mut body, "금강불괴", None, None, None)
            .unwrap();

        assert!(special.is_none());
        assert_eq!(body.get_int("내공"), 850);
        assert_eq!(body._arm, 100);
        assert_eq!(body._str, 15);
        assert_eq!(body.active_skills.len(), 1);
        assert_eq!(body.active_skills[0].name, "금강불괴");
        assert_eq!(body.active_skills[0].anti_type, "방어");
        assert_eq!(body.skill_map["금강불괴"], SkillTraining::new(1, 1));
        assert_eq!(output.len(), 1);
        assert!(output[0].starts_with("당신이 不動의 자세로 合掌하며"));
        assert!(output[0].contains("\r\n당신의 온몸이"));
    }

    #[test]
    fn active_defense_state_round_trips_as_python_array_without_user_data() {
        let path = std::env::temp_dir().join(format!(
            "muc_cast_effect_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut body = Body::new();
        body.set("이름", "방어저장검사");
        body.active_skills
            .push(active_skill(&get_skill("금강불괴").unwrap(), 35));
        body._arm = 100;
        body._str = 15;
        assert!(crate::script::save_body_to_json(
            &mut body,
            &path.to_string_lossy()
        ));

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            json["사용자오브젝트"]["방어무공시전"],
            serde_json::json!(["금강불괴 35"])
        );

        let mut loaded = Body::new();
        assert!(crate::script::load_body_from_json(
            &mut loaded,
            &path.to_string_lossy()
        ));
        assert_eq!(loaded.active_skills.len(), 1);
        assert_eq!(loaded.active_skills[0].name, "금강불괴");
        assert_eq!(loaded.active_skills[0].start_time, 35);
        assert_eq!(loaded._arm, 100);
        assert_eq!(loaded._str, 15);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rhai_other_player_buff_mutates_target_and_uses_same_room_recipients() {
        let caster_name = "타인시전자";
        let target_name = "타인대상";
        let observer_name = "출력거부관전자";
        let mut target = Body::new();
        target.set("이름", target_name);
        target.set("체력", 321_i64);
        target.set("최고체력", 456_i64);
        target.set("내공", 12_i64);
        target.set("최고내공", 34_i64);
        let mut observer = Body::new();
        observer.set("이름", observer_name);
        observer.set("설정상태", "타인전투출력거부 1");
        let mut accepting_observer = Body::new();
        accepting_observer.set("이름", "시전관전자");
        accepting_observer.set("설정상태", "타인전투출력거부 0");
        accepting_observer.set("체력", 77_i64);
        accepting_observer.set("최고체력", 88_i64);
        accepting_observer.set("내공", 5_i64);
        accepting_observer.set("최고내공", 6_i64);
        set_cast_room_players(vec![
            CastRoomPlayerRef::new(&mut target),
            CastRoomPlayerRef::new(&mut observer),
            CastRoomPlayerRef::new(&mut accepting_observer),
        ]);
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                caster_name,
                PlayerPosition::new("타인시전검사존".to_string(), "1".to_string()),
            );
        }

        let storage = ScriptStorage::default();
        let mut caster = Body::new();
        caster.set("이름", caster_name);
        caster.set("내공", 1_000i64);
        caster.set("최고내공", 1_000i64);
        caster.set("체력", 1_000i64);
        caster.set("최고체력", 1_000i64);
        caster.skill_list.push("무극강기".to_string());
        let line = format!("{} 무극강기", target_name);
        let (output, special) = storage
            .execute("시전", &mut caster, &line, None, None, None)
            .unwrap();

        assert_eq!(caster.get_int("내공"), 850);
        assert_eq!(target._arm, 120);
        assert_eq!(target.active_skills.len(), 1);
        assert_eq!(target.active_skills[0].name, "무극강기");
        assert_eq!(output.len(), 1);
        assert!(output[0].starts_with("당신이 \x1b[1;36m진기"));
        assert!(
            matches!(
                special,
                Some(crate::command::CommandResult::OutputAndSendToUsers(ref own, ref sends))
                    if own == &output[0]
                        && sends.len() == 2
                        && sends[0].0 == target_name
                        && sends[0].1.contains("당신의 주위에")
                        && sends[0].1.ends_with("\r\n\x1b[0;37;40m[ 321/4056, 12/34 ] ")
                        && sends[1].0 == "시전관전자"
                        && sends[1].1.ends_with("\r\n\x1b[0;37;40m[ 77/88, 5/6 ] ")
                        && sends.iter().all(|(name, _)| name != observer_name)
            ),
            "unexpected cast deliveries: {special:?}"
        );

        clear_cast_room_players();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(caster_name);
    }

    #[test]
    fn absorption_and_debuff_state_match_python_formulas() {
        let mut caster = Body::new();
        caster.set("이름", "흡수시전자");
        caster.set("레벨", 100i64);
        caster.set("명중", 1_000i64); // chance > every randint(0, 100)
        caster.set("내공", 1_000i64);
        caster.set("최고내공", 1_000i64);
        caster.set("체력", 1_000i64);
        caster.set("최고체력", 1_000i64);
        let mut target = Body::new();
        target.set("이름", "흡수대상");
        target.set("레벨", 1i64);
        target.set("내공", 1i64);
        target.set("최고내공", 1_000i64);
        target.set("체력", 1_000i64);
        target.set("최고체력", 1_000i64);
        set_cast_room_players(vec![CastRoomPlayerRef::new(&mut target)]);

        let result = apply_noncombat(&mut caster, "흡성대법", "player", "흡수대상");
        assert_eq!(result["status"].clone().into_string().unwrap(), "ok");
        // Python `1 * -40 // 100 * -1` is 1 (floor division, not truncation).
        assert_eq!(result["plus"].as_int().unwrap(), 1);
        assert_eq!(target.get_int("내공"), 0);
        assert_eq!(caster.get_int("내공"), 851); // 1000 - 150 + 1
        assert_eq!(caster.active_skills[0].name, "흡성대법");

        caster.active_skills.clear();
        caster.set("체력", 1_000i64);
        target.active_skills.clear();
        let result = apply_noncombat(&mut caster, "태청신공", "player", "흡수대상");
        assert_eq!(result["status"].clone().into_string().unwrap(), "ok");
        assert_eq!(caster.get_int("체력"), 490); // 최고체력의 51%
        assert_eq!(target._mp, -100);
        assert_eq!(target._maxmp, -100);
        assert_eq!(target.active_skills[0].name, "태청신공대상");
        assert_eq!(caster.active_skills[0].name, "태청신공");
        clear_cast_room_players();
    }

    #[test]
    fn rhai_combat_cast_starts_target_and_keeps_script_format_in_rhai() {
        let player_name = "전투시전검사";
        let zone = "시전회귀검사존";
        let room = "1";
        let (mob_key, mob_name) = {
            let mut world = get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("절강성", "7")
                .expect("repository mob fixture");
            let mob_key = "절강성:7".to_string();
            let mob_name = data.name.clone();
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.to_string(),
                room,
                &data,
            ));
            world.set_player_position(
                player_name,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            (mob_key, mob_name)
        };

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player_name);
        body.set("내공", 1_000i64);
        body.set("최고내공", 1_000i64);
        body.set("체력", 1_000i64);
        body.set("최고체력", 1_000i64);
        body.skill_list.push("강룡십팔장".to_string());
        let mut observer = Body::new();
        observer.set("이름", "전투시전관전자");
        observer.set("체력", 55_i64);
        observer.set("최고체력", 66_i64);
        observer.set("내공", 7_i64);
        observer.set("최고내공", 8_i64);
        let mut rejecting_observer = Body::new();
        rejecting_observer.set("이름", "전투시전거부관전자");
        rejecting_observer.set("설정상태", "타인전투출력거부 1");
        rejecting_observer.set("체력", 11_i64);
        rejecting_observer.set("최고체력", 22_i64);
        rejecting_observer.set("내공", 3_i64);
        rejecting_observer.set("최고내공", 4_i64);
        set_cast_room_players(vec![
            CastRoomPlayerRef::new(&mut observer),
            CastRoomPlayerRef::new(&mut rejecting_observer),
        ]);

        let line = format!("{} 강룡십팔장", mob_name);
        let (output, special) = storage
            .execute("시전", &mut body, &line, None, None, None)
            .unwrap();

        let sends = match special {
            Some(crate::command::CommandResult::OutputAndSendToUsers(_, sends)) => sends,
            other => panic!("expected room deliveries, got {other:?}"),
        };
        for (name, prompt) in [
            (
                "전투시전관전자",
                "\0MUC_RAW_USER\0\r\n\x1b[0;37;40m[ 55/66, 7/8 ] ",
            ),
            (
                "전투시전거부관전자",
                "\0MUC_RAW_USER\0\r\n\x1b[0;37;40m[ 11/22, 3/4 ] ",
            ),
        ] {
            let messages = sends
                .iter()
                .filter(|(recipient, _)| recipient == name)
                .map(|(_, message)| message.as_str())
                .collect::<Vec<_>>();
            assert_eq!(messages.len(), 4, "{name}: {messages:?}");
            assert!(messages[0].contains("雙手"));
            assert!(messages[1].contains("주먹을 쥐며 공격 합니다."));
            assert!(messages[2]
                .starts_with(&format!("\0MUC_RAW_USER\0\r\n\x1b[33m{}\x1b[37m", mob_name)));
            assert_eq!(messages[3], prompt);
        }
        assert_eq!(body.get_int("내공"), 700);
        assert_eq!(body.skill.as_deref(), Some("강룡십팔장"));
        assert_eq!(body.act, ActState::Fight);
        assert_eq!(
            target_instance_ids(&body).len(),
            1,
            "a cast-started fight must retain the exact mob instance"
        );
        assert_eq!(
            body.temp()
                .get("_attack_target_key")
                .and_then(Value::as_str),
            Some(mob_key.as_str())
        );
        assert!(output[0].starts_with("당신의 \x1b[1;32m雙手"));
        assert_eq!(output[1], "당신이 주먹을 쥐며 공격 합니다.");
        assert!(output[2].starts_with(&format!("\x1b[33m{}\x1b[37m", mob_name)));
        {
            let world = get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room(zone, room)
                .into_iter()
                .find(|mob| mob.mob_key == mob_key)
                .unwrap();
            assert_eq!(mob.act, 1);
            assert_eq!(mob.targets, vec![player_name.to_string()]);
        }

        // On a later cast in an existing fight Python uses
        // sendRoomFightScript: rejectors are skipped and normal observers get
        // the script and lpPrompt in one delivery.
        body.skill = None;
        let (repeat_output, repeat_special) = storage
            .execute("시전", &mut body, "강룡십팔장", None, None, None)
            .unwrap();
        let repeat_sends = match repeat_special {
            Some(crate::command::CommandResult::OutputAndSendToUsers(_, sends)) => sends,
            other => panic!("expected existing-fight room delivery, got {other:?}, output={repeat_output:?}, temp={:?}", body.temp()),
        };
        assert_eq!(repeat_sends.len(), 1, "{repeat_sends:?}");
        assert_eq!(repeat_sends[0].0, "전투시전관전자");
        assert!(repeat_sends[0].1.contains("雙手"));
        assert!(repeat_sends[0]
            .1
            .ends_with("\r\n\x1b[0;37;40m[ 55/66, 7/8 ] "));

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(player_name);
        world.kill_mob(zone, room, &mob_key);
        clear_cast_room_players();
    }

    #[test]
    fn rhai_combat_cast_starts_reciprocal_pvp_and_pays_cost_once() {
        let suffix = std::process::id();
        let caster_name = format!("비무시전자-{suffix}");
        let target_name = format!("비무시전대상-{suffix}");
        let mut caster = Body::new();
        caster.set("이름", caster_name.as_str());
        caster.set("내공", 1_000_i64);
        caster.set("최고내공", 1_000_i64);
        caster.set("체력", 1_000_i64);
        caster.set("최고체력", 1_000_i64);
        caster.skill_list.push("강룡십팔장".to_string());
        let mut target = Body::new();
        target.set("이름", target_name.as_str());
        target.set("체력", 1_000_i64);
        target.set("최고체력", 1_000_i64);
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &caster_name,
                PlayerPosition::new("낙양성".to_string(), "56".to_string()),
            );
            world.set_player_position(
                &target_name,
                PlayerPosition::new("낙양성".to_string(), "56".to_string()),
            );
        }
        set_cast_room_players(vec![CastRoomPlayerRef::new(&mut target)]);
        let storage = ScriptStorage::default();
        let line = format!("{target_name} 강룡십팔장");
        let (denied, _) = storage
            .execute("시전", &mut caster, &line, None, None, None)
            .unwrap();
        assert_eq!(
            denied,
            vec!["☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네"]
        );
        assert_eq!(caster.get_mp(), 1_000);
        assert!(super::super::combat_commands::pvp_target(&caster).is_none());
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &caster_name,
                PlayerPosition::new("낙양성".to_string(), "1000".to_string()),
            );
            world.set_player_position(
                &target_name,
                PlayerPosition::new("낙양성".to_string(), "1000".to_string()),
            );
        }
        set_cast_room_players(vec![CastRoomPlayerRef::new(&mut target)]);
        let (output, _) = storage
            .execute("시전", &mut caster, &line, None, None, None)
            .unwrap();
        assert_eq!(caster.get_mp(), 700);
        assert_eq!(caster.skill.as_deref(), Some("강룡십팔장"));
        assert_eq!(
            super::super::combat_commands::pvp_target(&caster).as_deref(),
            Some(target_name.as_str())
        );
        assert_eq!(
            super::super::combat_commands::pvp_target(&target).as_deref(),
            Some(caster_name.as_str())
        );
        assert_eq!(caster.act, ActState::Fight);
        assert_eq!(target.act, ActState::Fight);
        assert!(output[0].starts_with("당신의 \x1b[1;32m雙手"));
        clear_cast_room_players();
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&caster_name);
        world.remove_player_position(&target_name);
    }
}
