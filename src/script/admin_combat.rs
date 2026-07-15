//! State efuns for Python combat-administration commands.
//! Rhai retains all user-visible output.

use super::current_body_position;
use crate::player::Body;
use crate::world::get_world_state;
use rhai::{Array, Dynamic, Engine, Map};
use std::collections::HashMap;

pub(super) fn python_named_room_selection_is_nonmob(
    world: &crate::world::WorldState,
    zone: &str,
    room: &str,
    raw_query: &str,
) -> bool {
    let mut query = raw_query.split_whitespace().next().unwrap_or("");
    if query.is_empty() || query == "." || query.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    let digits = query.chars().take_while(|ch| ch.is_ascii_digit()).count();
    let order = if digits == 0 {
        1
    } else {
        let parsed = query[..digits].parse::<usize>().unwrap_or(0);
        query = &query[digits..];
        parsed
    };
    if order == 0 || query.is_empty() {
        return false;
    }
    let floor = world.get_room_objs(zone, room);
    let mobs = world.mob_cache.get_all_mobs_in_room(zone, room);
    let installed = super::box_commands::installed_boxes_for_room(zone, room).unwrap_or_default();
    let players = super::room_view_player_snapshots(zone, room)
        .into_iter()
        .filter_map(|value| value.try_cast::<Map>())
        .collect::<Vec<_>>();
    let match_counts = |name: &str, aliases: &str| {
        let aliases = aliases.split_whitespace().collect::<Vec<_>>();
        let exact = name == query || aliases.contains(&query);
        let prefixes = if exact {
            0
        } else {
            aliases
                .iter()
                .filter(|alias| alias.starts_with(query))
                .count()
        };
        (exact, prefixes)
    };
    let mut exact_count = 0usize;
    let mut prefix_count = 0usize;
    for object in world.get_room_object_order(zone, room) {
        let ((exact, prefixes), nonmob) = match object {
            crate::world::RoomObjectRef::Mob(id) => {
                let Some(mob) = mobs.iter().find(|mob| mob.instance_id == id) else {
                    continue;
                };
                let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                    continue;
                };
                if query != "시체" && (mob.act == 2 || mob.act == 3) {
                    continue;
                }
                let exact = (query == "시체" && mob.act == 2)
                    || data.name == query
                    || data.reaction_names.iter().any(|alias| alias == query);
                let prefixes = if exact {
                    0
                } else {
                    data.reaction_names
                        .iter()
                        .filter(|alias| alias.starts_with(query))
                        .count()
                };
                ((exact, prefixes), false)
            }
            crate::world::RoomObjectRef::FloorItem(pointer) => {
                let Some(item) = floor
                    .iter()
                    .find(|item| std::sync::Arc::as_ptr(item) as usize == pointer)
                else {
                    continue;
                };
                let Ok(item) = item.lock() else { continue };
                let matched = match_counts(&item.getName(), &item.getString("반응이름"));
                (
                    if item.getInt("투명상태") != 1 {
                        matched
                    } else {
                        (false, 0)
                    },
                    true,
                )
            }
            crate::world::RoomObjectRef::Box(pointer) => {
                let Some(item) = installed
                    .iter()
                    .find(|item| std::sync::Arc::as_ptr(item) as usize == pointer)
                else {
                    continue;
                };
                let Ok(item) = item.lock() else { continue };
                let matched = match_counts(&item.getName(), &item.getString("반응이름"));
                (
                    if item.getInt("투명상태") != 1 {
                        matched
                    } else {
                        (false, 0)
                    },
                    true,
                )
            }
            crate::world::RoomObjectRef::InstalledBox(ordinal) => {
                let Some(item) = installed.get(ordinal) else {
                    continue;
                };
                let Ok(item) = item.lock() else { continue };
                let matched = match_counts(&item.getName(), &item.getString("반응이름"));
                (
                    if item.getInt("투명상태") != 1 {
                        matched
                    } else {
                        (false, 0)
                    },
                    true,
                )
            }
            crate::world::RoomObjectRef::Player(name) => {
                let matched = players.iter().find(|player| {
                    player
                        .get("이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .as_deref()
                        == Some(name.as_str())
                });
                let visible = matched
                    .and_then(|player| player.get("transparent"))
                    .and_then(|value| value.as_bool().ok())
                    != Some(true);
                let reactions = matched
                    .and_then(|player| player.get("반응이름"))
                    .and_then(|value| value.clone().into_string().ok())
                    .unwrap_or_default();
                let matched = match_counts(&name, &reactions);
                (if visible { matched } else { (false, 0) }, true)
            }
            crate::world::RoomObjectRef::SummonedUser(id) => {
                let matched = world
                    .summoned_users()
                    .iter()
                    .find(|user| user.id == id)
                    .filter(|user| user.body.get_int("투명상태") != 1)
                    .map(|user| {
                        match_counts(&user.body.get_name(), &user.body.get_string("반응이름"))
                    })
                    .unwrap_or((false, 0));
                (matched, true)
            }
            crate::world::RoomObjectRef::Fixture(id) => {
                let matched = world
                    .get_fixture(id)
                    .map(|fixture| fixture.match_counts(query))
                    .unwrap_or((false, 0));
                (matched, true)
            }
        };
        if exact {
            exact_count += 1;
            if exact_count == order {
                return nonmob;
            }
        } else {
            for _ in 0..prefixes {
                prefix_count += 1;
                if prefix_count == order {
                    return nonmob;
                }
            }
        }
    }
    false
}

pub(super) fn python_room_mob_index(
    mobs: &[crate::world::MobInstance],
    metadata: &HashMap<String, crate::world::RawMobData>,
    raw_query: &str,
) -> Option<usize> {
    let mut name = if raw_query.trim() == "." {
        "1"
    } else {
        raw_query.split_whitespace().next().unwrap_or("")
    };
    if name.is_empty() {
        return None;
    }

    // Room.findObjName treats a pure number as the Nth visible live mob and
    // excludes mob type 7 from that particular enumeration.
    if name.chars().all(|ch| ch.is_ascii_digit()) {
        let order = name.parse::<usize>().ok()?;
        if order == 0 {
            return None;
        }
        return mobs
            .iter()
            .enumerate()
            .filter(|(_, mob)| {
                mob.act != 2
                    && mob.act != 3
                    && metadata
                        .get(&mob.mob_key)
                        .is_some_and(|data| data.mob_type != 7)
            })
            .nth(order - 1)
            .map(|(index, _)| index);
    }

    let digit_count = name.chars().take_while(|ch| ch.is_ascii_digit()).count();
    let order = if digit_count == 0 {
        1
    } else {
        let parsed = name[..digit_count].parse::<usize>().unwrap_or(0);
        name = &name[digit_count..];
        parsed
    };
    if order == 0 || name.is_empty() {
        return None;
    }

    let mut exact_count = 0usize;
    let mut prefix_count = 0usize;
    for (index, mob) in mobs.iter().enumerate() {
        let Some(data) = metadata.get(&mob.mob_key) else {
            continue;
        };
        if name != "시체" && (mob.act == 2 || mob.act == 3) {
            continue;
        }
        if name == "시체" && mob.act == 2 {
            exact_count += 1;
            if exact_count == order {
                return Some(index);
            }
        } else if data.name == name || data.reaction_names.iter().any(|alias| alias == name) {
            exact_count += 1;
            if exact_count == order {
                return Some(index);
            }
        } else if data
            .reaction_names
            .iter()
            .any(|alias| alias.starts_with(name))
        {
            prefix_count += 1;
            if prefix_count == order {
                return Some(index);
            }
        }
    }
    None
}

fn regen_mobs(body: &Body, query: &str) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from("unknown"));
    result.insert("messages".into(), Dynamic::from(Array::new()));
    let Some((zone, room)) = current_body_position(body) else {
        return result;
    };
    let Ok(mut world) = get_world_state().write() else {
        return result;
    };
    if !query.is_empty() && python_named_room_selection_is_nonmob(&world, &zone, &room, query) {
        return result;
    }
    let metadata = world
        .mob_cache
        .get_all_mobs_in_room(&zone, &room)
        .into_iter()
        .filter_map(|mob| {
            world
                .mob_cache
                .get_mob(&mob.mob_key)
                .cloned()
                .map(|d| (mob.mob_key.clone(), d))
        })
        .collect::<HashMap<_, _>>();
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return result;
    };

    let mut selected = Vec::new();
    if query.is_empty() {
        selected.extend(
            mobs.iter()
                .enumerate()
                .filter(|(_, mob)| mob.act == 2 || mob.act == 3)
                .map(|(index, _)| index),
        );
        if selected.is_empty() {
            result.insert("status".into(), Dynamic::from("empty"));
            return result;
        }
    } else {
        let Some(index) = python_room_mob_index(mobs, &metadata, query) else {
            result.insert("status".into(), Dynamic::from("unknown"));
            return result;
        };
        if mobs[index].act != 2 {
            result.insert("status".into(), Dynamic::from("not_corpse"));
            return result;
        }
        selected.push(index);
    }

    let mut messages = Array::new();
    for index in selected {
        let Some(mob) = mobs.get_mut(index) else {
            continue;
        };
        let Some(data) = metadata.get(&mob.mob_key) else {
            continue;
        };
        mob.respawn(data);
        messages.push(Dynamic::from(data.desc3.clone()));
    }
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("messages".into(), Dynamic::from(messages));
    result
}

fn kill_mob(body: &Body, target_id: &str, room_index: i64) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from("missing"));
    let Some((zone, room)) = current_body_position(body) else {
        return result;
    };
    let Ok(mut world) = get_world_state().write() else {
        return result;
    };
    let data = world.mob_cache.get_mob(target_id).cloned();
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return result;
    };
    let Ok(index) = usize::try_from(room_index) else {
        return result;
    };
    let Some(mob) = mobs.get_mut(index) else {
        return result;
    };
    let Some(data) = data else { return result };
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
    if attack_forbidden {
        return result;
    }
    if mob.mob_key != target_id || !mob.alive {
        return result;
    }
    let name = mob.name.clone();
    if !crate::server::game_loop::queue_admin_mob_death(mob, &data) {
        return result;
    }
    mob.skills.clear();
    mob.skill_effects.clear();
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("name".into(), Dynamic::from(name));
    result.insert("death_script".into(), Dynamic::from(data.death_script));
    result
}

fn recover_mob(body: &Body, target_id: &str, room_index: i64) -> bool {
    let Some((zone, room)) = current_body_position(body) else {
        return false;
    };
    let Ok(mut world) = get_world_state().write() else {
        return false;
    };
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return false;
    };
    let Ok(index) = usize::try_from(room_index) else {
        return false;
    };
    let Some(mob) = mobs.get_mut(index) else {
        return false;
    };
    if mob.mob_key != target_id || !mob.alive || mob.act == 2 || mob.act == 3 {
        return false;
    }
    mob.hp = mob.max_hp;
    true
}

fn recover_named_mob(body: &Body, query: &str) -> bool {
    let Some((zone, room)) = current_body_position(body) else {
        return false;
    };
    let Ok(mut world) = get_world_state().write() else {
        return false;
    };
    if python_named_room_selection_is_nonmob(&world, &zone, &room, query) {
        return false;
    }
    let ordered = world.get_room_object_order(&zone, &room);
    let metadata = world
        .mob_cache
        .ordered_mob_templates()
        .map(|(key, data)| (key.to_string(), data.clone()))
        .collect::<HashMap<_, _>>();
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return false;
    };
    let mut first = if query.trim() == "." {
        "1"
    } else {
        query.split_whitespace().next().unwrap_or("")
    };
    if first.is_empty() {
        return false;
    }
    let pure_numeric = first.chars().all(|ch| ch.is_ascii_digit());
    let numeric = pure_numeric
        .then(|| first.parse::<usize>().ok())
        .flatten()
        .filter(|number| *number != 0);
    let digit_count = first.chars().take_while(|ch| ch.is_ascii_digit()).count();
    let order = if pure_numeric || digit_count == 0 {
        1
    } else {
        let order = first[..digit_count].parse::<usize>().unwrap_or(0);
        first = &first[digit_count..];
        order
    };
    if order == 0 || first.is_empty() {
        return false;
    }
    let mut count = 0usize;
    let mut prefix_count = 0usize;
    for object in ordered {
        let crate::world::RoomObjectRef::Mob(instance_id) = object else {
            // A matching non-mob would be a hard stop in Room.findObjName,
            // but cross-type name data is not represented in this efun.
            continue;
        };
        let Some(index) = mobs.iter().position(|mob| mob.instance_id == instance_id) else {
            continue;
        };
        let mob = &mobs[index];
        let Some(data) = metadata.get(&mob.mob_key) else {
            continue;
        };
        let matched = if let Some(wanted) = numeric {
            if data.mob_type == 7 || mob.act == 2 || mob.act == 3 {
                false
            } else {
                count += 1;
                count == wanted
            }
        } else if first == "시체" {
            if mob.act == 2 {
                count += 1;
                count == order
            } else {
                false
            }
        } else {
            if mob.act == 2 || mob.act == 3 {
                false
            } else if data.name == first || data.reaction_names.iter().any(|name| name == first) {
                count += 1;
                count == order
            } else if data
                .reaction_names
                .iter()
                .any(|alias| alias.starts_with(first))
            {
                prefix_count += 1;
                prefix_count == order
            } else {
                false
            }
        };
        if matched {
            mobs[index].hp = data.hp;
            return true;
        }
    }
    false
}

fn create_items(body: &mut Body, key: &str, count: i64) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from("missing"));
    let Some((template, display_name)) = super::object_from_item_json(key) else {
        return result;
    };
    let Ok(template_guard) = template.lock() else {
        return result;
    };
    let one_item = template_guard.checkAttr("아이템속성", "단일아이템");
    let index = template_guard.getString("인덱스");
    let particle_source = template_guard.getName();
    if one_item
        && (!crate::oneitem::oneitem_get(&index).is_empty()
            || super::inventory_compat::inventory_contains_index(&body.object, &index))
    {
        result.insert("status".into(), Dynamic::from("one_exists"));
        result.insert("name".into(), Dynamic::from(display_name));
        let ansi = template_guard.getString("안시");
        let name_a = if ansi.is_empty() {
            format!("\x1b[0;36m{}\x1b[37m", template_guard.getName())
        } else {
            format!("{ansi}{}\x1b[0;37m", template_guard.getName())
        };
        result.insert("name_a".into(), Dynamic::from(name_a));
        result.insert("particle_source".into(), Dynamic::from(particle_source));
        return result;
    }
    if one_item {
        crate::oneitem::oneitem_have(&index, &body.get_name());
    }
    let count = if one_item {
        count.clamp(0, 1)
    } else {
        count.max(0)
    };
    if super::is_stackable(&index) {
        *body.object.inv_stack.entry(index).or_insert(0) += count;
    } else {
        for _ in 0..count {
            body.object
                .objs
                .push(std::sync::Arc::new(std::sync::Mutex::new(
                    template_guard.deepclone(),
                )));
        }
    }
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("name".into(), Dynamic::from(display_name));
    result
}

pub(super) fn body_status(body: &Body) -> Map {
    let mut map = Map::new();
    for key in ["이름", "성격", "성별", "소속", "직위", "배우자"] {
        map.insert(key.into(), Dynamic::from(body.get_string(key)));
    }
    for (key, value) in [
        ("level", body.get_int("레벨")),
        ("age", body.get_int("나이")),
        ("hp", body.get_hp()),
        ("max_hp", body.get_max_hp()),
        ("attack", body.get_attack_power() as i64),
        ("strength", body.get_str()),
        ("armor", body.get_armor() as i64),
        ("arm", body.get_arm()),
        ("dex", body.get_dex()),
        ("mp", body.get_mp()),
        ("max_mp", body.get_max_mp()),
        ("weight", body.get_item_weight()),
        ("current_exp", body.get_int("현재경험치")),
        ("total_exp", body.get_total_exp()),
        ("hit", body.get_hit()),
        ("miss", body.get_miss()),
        ("critical", body.get_critical()),
        ("luck", body.get_critical_chance()),
        ("silver", body.get_int("은전")),
        ("feature", body.get_int("특성치")),
        ("insurance_premium", body.get_int("보험료")),
    ] {
        map.insert(key.into(), Dynamic::from(value));
    }
    let unit = std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|root| root.get("메인설정").cloned())
        .and_then(|main| main.get("보험료단가").and_then(|value| value.as_i64()))
        .unwrap_or(0);
    let denominator = body.get_int("레벨").saturating_mul(unit);
    map.insert(
        "insurance".into(),
        Dynamic::from(if denominator > 0 {
            body.get_int("보험료") / denominator
        } else {
            0
        }),
    );
    map.insert(
        "hp_script".into(),
        Dynamic::from(super::hp_status_script(
            body.get_hp(),
            body.get_int("최고체력"),
        )),
    );
    map.insert(
        "mp_script".into(),
        Dynamic::from(super::mp_status_script(body.get_mp())),
    );
    map
}

fn insurance_count(level: i64, premium: i64) -> i64 {
    let unit = std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|root| root.get("메인설정").cloned())
        .and_then(|main| main.get("보험료단가").and_then(|value| value.as_i64()))
        .unwrap_or(0);
    let denominator = level.saturating_mul(unit);
    if denominator > 0 {
        premium / denominator
    } else {
        0
    }
}

pub(crate) fn clear_summon_combat(body: &mut Body) {
    let name = body.get_name();
    crate::script::combat_commands::clear_pvp_target(body);
    body.targets.clear();
    body.temp_mut().remove("_combat_target_ids");
    body.temp_mut().remove("_combat_target_instance_ids");
    body.temp_mut().remove("_attack_target");
    body.temp_mut().remove("_attack_target_key");
    body.act = crate::player::ActState::Stand;
    if let Ok(mut world) = get_world_state().write() {
        world.mob_cache.remove_target_everywhere(&name);
    }
}

pub(super) fn register_admin_combat_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    let ptr = body_ptr;
    engine.register_fn("regen_room_mobs", move |_ob: &mut Map, query: &str| {
        regen_mobs(unsafe { &*ptr }, query)
    });
    engine.register_fn(
        "admin_kill_mob",
        move |_ob: &mut Map, id: &str, index: i64| kill_mob(unsafe { &*body_ptr }, id, index),
    );
    let ptr = body_ptr;
    engine.register_fn(
        "recover_room_mob",
        move |_ob: &mut Map, id: &str, index: i64| recover_mob(unsafe { &*ptr }, id, index),
    );
    let ptr = body_ptr;
    engine.register_fn(
        "recover_named_room_mob",
        move |_ob: &mut Map, query: &str| recover_named_mob(unsafe { &*ptr }, query),
    );
    engine.register_fn(
        "admin_create_items",
        move |_ob: &mut Map, key: &str, count: i64| {
            create_items(unsafe { &mut *body_ptr }, key, count)
        },
    );
    let ptr = body_ptr;
    engine.register_fn("admin_self_status", move |_ob: &mut Map| {
        body_status(unsafe { &*ptr })
    });
    engine.register_fn("admin_insurance_count", insurance_count);
    engine.register_fn("clear_summon_combat", move |_ob: &mut Map| {
        clear_summon_combat(unsafe { &mut *body_ptr })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::handler::CommandResult;
    use crate::script::party::set_precomputed_party_context;
    use crate::script::ScriptStorage;

    #[test]
    fn admin_combat_rhai_commands_keep_python_permission_guard() {
        let storage = ScriptStorage::default();
        for command in ["죽여", "리젠", "몹회복", "생성"] {
            let mut body = Body::new();
            let (output, special) = storage
                .execute(command, &mut body, "", None, None, None)
                .unwrap();
            assert_eq!(output, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
            assert!(special.is_none());
        }
    }

    #[test]
    fn mob_recovery_updates_the_selected_runtime_instance_to_its_maximum() {
        let suffix = std::process::id();
        let player = format!("몹회복검사{suffix}");
        let zone = format!("몹회복존{suffix}");
        let room = "1";
        let key = format!("{zone}:대상");
        let mut data = crate::world::RawMobData::new();
        data.name = "회복대상".to_string();
        data.max_hp = 777;
        let mut instance = crate::world::MobInstance::new(key.clone(), zone.clone(), room, &data);
        instance.hp = 12;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(instance);
            world.set_player_position(
                &player,
                crate::world::PlayerPosition::new(zone.clone(), room.to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player.clone());
        assert!(recover_mob(&body, &key, 0));
        {
            let mut world = get_world_state().write().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room(&zone, room)
                .into_iter()
                .find(|mob| mob.mob_key == key)
                .unwrap();
            assert_eq!(mob.hp, 777);
            world.remove_player_position(&player);
            world.mob_cache.remove_instance(&zone, room, &key);
            world.mob_cache.remove_mob(&key);
        }
    }

    #[test]
    fn named_admin_mob_commands_stop_at_first_matching_nonmob_like_python() {
        use crate::object::Object;
        use crate::world::{MobInstance, PlayerPosition, RawMobData, RoomObjectRef};
        use std::sync::{Arc, Mutex};

        let suffix = std::process::id();
        let player = format!("관리몹충돌-{suffix}");
        let zone = format!("관리몹충돌존-{suffix}");
        let room = "1";
        let key = format!("{zone}:충돌몹");
        let mut data = RawMobData::new();
        data.name = "충돌몹".into();
        data.reaction_names = vec!["충돌대상".into()];
        data.hp = 500;
        data.max_hp = 500;
        let mut mob = MobInstance::new(key.clone(), zone.clone(), room, &data);
        mob.hp = 10;
        let mob_id = mob.instance_id;
        let mut item = Object::new();
        item.set("이름", "충돌패");
        item.set("반응이름", "충돌대상 시체");
        let item = Arc::new(Mutex::new(item));
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
            world.mob_cache.add_mob_instance(mob);
            world.record_test_room_object(&zone, room, RoomObjectRef::Mob(mob_id));
            world.get_room_objs_mut(&zone, room).push(item.clone());
            world.record_floor_item(&zone, room, &item);
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("관리자등급", 2000_i64);
        let storage = ScriptStorage::default();

        let recovery = storage
            .execute("몹회복", &mut body, "충돌대상", None, None, None)
            .unwrap();
        assert_eq!(recovery.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, room)[0]
                .hp,
            10
        );
        let removal = storage
            .execute("몹제거", &mut body, "충돌대상", None, None, None)
            .unwrap();
        assert_eq!(removal.0, vec!["그런 몹이 없어요!"]);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, room)
                .len(),
            1
        );

        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room_mut(&zone, room)
            .unwrap()[0]
            .kill();
        let regen = storage
            .execute("리젠", &mut body, "시체", None, None, None)
            .unwrap();
        assert_eq!(regen.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, room)[0]
                .act,
            2
        );

        // Runtime-installed Boxes live in the shared Box registry rather
        // than `room_objs`, but Python still places them in the same ordered
        // Room.objs sequence.
        {
            let mut world = get_world_state().write().unwrap();
            world.remove_floor_item_record(&zone, room, &item);
            world.get_room_objs_mut(&zone, room).clear();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room_mut(&zone, room)
                .unwrap()
                .iter_mut()
                .find(|mob| mob.instance_id == mob_id)
                .unwrap();
            mob.alive = true;
            mob.act = 0;
            mob.hp = 10;
            world.record_test_room_object(&zone, room, RoomObjectRef::Mob(mob_id));
        }
        let runtime_box = Arc::new(Mutex::new(Object::new()));
        runtime_box.lock().unwrap().set("이름", "충돌대상");
        super::super::box_commands::register_installed_box(&zone, room, runtime_box);
        let blocked = storage
            .execute("몹회복", &mut body, "충돌대상", None, None, None)
            .unwrap();
        assert_eq!(blocked.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
        get_world_state().write().unwrap().record_test_room_object(
            &zone,
            room,
            RoomObjectRef::Mob(mob_id),
        );
        let recovered = storage
            .execute("몹회복", &mut body, "충돌대상", None, None, None)
            .unwrap();
        assert_eq!(recovered.0, vec!["* 회복되었습니다."]);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, room)
                .into_iter()
                .find(|mob| mob.instance_id == mob_id)
                .unwrap()
                .hp,
            500
        );

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&key);
    }

    #[test]
    fn admin_kill_honors_attack_forbidden_and_broadcasts_death_once_to_room() {
        use crate::script::party::{set_precomputed_party_context, take_party_requests};
        use crate::world::{MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let admin = format!("죽여관리자-{suffix}");
        let witness = format!("죽여목격자-{suffix}");
        let zone = format!("죽여존-{suffix}");
        let room = "1";
        let key = format!("{zone}:대상");
        let mut data = RawMobData::new();
        data.name = "죽임대상".into();
        data.reaction_names = vec!["대상".into()];
        data.death_script = "죽임대상이 조용히 사라집니다.".into();
        data.attributes
            .insert("공격금지".into(), serde_json::json!(1));
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                key.clone(),
                zone.clone(),
                room,
                &data,
            ));
            for name in [&admin, &witness] {
                world.set_player_position(name, PlayerPosition::new(zone.clone(), room.into()));
            }
        }
        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let storage = ScriptStorage::default();

        let forbidden = storage
            .execute("죽여", &mut body, "대상", None, None, None)
            .unwrap();
        assert_eq!(
            forbidden.0,
            vec!["☞ 강호에는 공격할 수 있는것과 없는것이 있지!"]
        );
        assert!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, room)[0]
                .alive
        );

        data.attributes.remove("공격금지");
        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .insert_mob_data(key.clone(), data);
        let mut admin_person = rhai::Map::new();
        admin_person.insert("id".into(), Dynamic::from("admin-token"));
        admin_person.insert("name".into(), Dynamic::from(admin.clone()));
        let mut witness_person = rhai::Map::new();
        witness_person.insert("id".into(), Dynamic::from("witness-token"));
        witness_person.insert("name".into(), Dynamic::from(witness.clone()));
        let mut context = rhai::Map::new();
        context.insert("self".into(), Dynamic::from(admin_person.clone()));
        context.insert(
            "room_players".into(),
            Dynamic::from(vec![
                Dynamic::from(admin_person),
                Dynamic::from(witness_person),
            ]),
        );
        set_precomputed_party_context(context);
        let killed = storage
            .execute("죽여", &mut body, "대상", None, None, None)
            .unwrap();
        assert!(killed.0.is_empty());
        assert!(killed.1.is_none());
        let wire = "\r\n\x1b[1;37m죽임대상이 조용히 사라집니다.\x1b[0;37m\r\n";
        let (_, deliveries) = take_party_requests(&mut body);
        assert_eq!(
            deliveries,
            vec![
                crate::script::party::PartyDelivery {
                    connection_id: "admin-token".to_string(),
                    raw_text: wire.to_string(),
                },
                crate::script::party::PartyDelivery {
                    connection_id: "witness-token".to_string(),
                    raw_text: wire.to_string(),
                },
            ]
        );
        assert!(
            !get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, room)[0]
                .alive
        );

        let mut world = get_world_state().write().unwrap();
        world.mob_cache.remove_instance(&zone, room, &key);
        world.mob_cache.remove_mob_definition(&key);
        world.remove_player_position(&admin);
        world.remove_player_position(&witness);
    }

    #[test]
    fn admin_kill_numbered_alias_kills_only_second_integrated_mob() {
        use crate::world::{MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

        let suffix = std::process::id();
        let admin = format!("죽여순번관리자-{suffix}");
        let zone = format!("죽여순번존-{suffix}");
        let room = "1";
        let key = format!("{zone}:순번대상");
        let mut data = RawMobData::new();
        data.name = "죽여순번대상".into();
        data.reaction_names = vec!["죽여공통별칭".into()];
        data.hp = 100;
        data.max_hp = 100;
        let first = MobInstance::new(key.clone(), zone.clone(), room, &data);
        let first_id = first.instance_id;
        let second = MobInstance::new(key.clone(), zone.clone(), room, &data);
        let second_id = second.instance_id;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(first);
            world.mob_cache.add_mob_instance(second);
            // record_room_object prepends: second_id is first in Python
            // Room.objs, first_id is therefore the requested second match.
            world.record_test_room_object(&zone, room, RoomObjectRef::Mob(first_id));
            world.record_test_room_object(&zone, room, RoomObjectRef::Mob(second_id));
            world.set_player_position(&admin, PlayerPosition::new(zone.clone(), room.to_string()));
        }
        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let killed = ScriptStorage::default()
            .execute("죽여", &mut body, "2죽여공통", None, None, None)
            .unwrap();
        assert!(killed.0.is_empty());
        let world = get_world_state().read().unwrap();
        let mobs = world.mob_cache.get_all_mobs_in_room(&zone, room);
        assert!(
            mobs.iter()
                .find(|mob| mob.instance_id == second_id)
                .unwrap()
                .alive
        );
        let killed_mob = mobs.iter().find(|mob| mob.instance_id == first_id).unwrap();
        assert!(!killed_mob.alive);
        assert_eq!(killed_mob.act, 2);
        drop(world);
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&admin);
        world.mob_cache.remove_mob(&key);
    }

    #[test]
    fn regen_command_emits_each_python_respawn_description_once_and_confirms_only_all() {
        let suffix = std::process::id();
        let player = format!("리젠검사{suffix}");
        let observer = format!("리젠목격{suffix}");
        let zone = format!("리젠존{suffix}");
        let room = "1";
        let dead_key = format!("{zone}:죽은몹");
        let live_key = format!("{zone}:산몹");
        let mut dead_data = crate::world::RawMobData::new();
        dead_data.name = "죽은몹".into();
        dead_data.desc3 = "죽은몹이 다시 나타납니다.".into();
        dead_data.max_hp = 321;
        let mut dead =
            crate::world::MobInstance::new(dead_key.clone(), zone.clone(), room, &dead_data);
        dead.kill();
        let mut live_data = crate::world::RawMobData::new();
        live_data.name = "산몹".into();
        let live = crate::world::MobInstance::new(live_key.clone(), zone.clone(), room, &live_data);
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(dead_key.clone(), dead_data);
            world.mob_cache.insert_mob_data(live_key.clone(), live_data);
            world.mob_cache.add_mob_instance(dead);
            world.mob_cache.add_mob_instance(live);
            world.set_player_position(
                &player,
                crate::world::PlayerPosition::new(zone.clone(), room.into()),
            );
            world.set_player_position(
                &observer,
                crate::world::PlayerPosition::new(zone.clone(), room.into()),
            );
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("관리자등급", 1000_i64);
        let mut person = rhai::Map::new();
        person.insert("name".into(), Dynamic::from(observer.clone()));
        person.insert("show_prompt".into(), Dynamic::from(true));
        person.insert("hp".into(), Dynamic::from(22_i64));
        person.insert("max_hp".into(), Dynamic::from(33_i64));
        person.insert("mp".into(), Dynamic::from(5_i64));
        person.insert("max_mp".into(), Dynamic::from(8_i64));
        let mut context = rhai::Map::new();
        context.insert(
            "room_players".into(),
            Dynamic::from(vec![Dynamic::from(person)]),
        );
        set_precomputed_party_context(context);

        let all = storage
            .execute("리젠", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(
            all.0,
            vec!["\r\n죽은몹이 다시 나타납니다.", "\r\n☞ 리젠되었습니다."],
            "Python doRegen/writeRoom description must not be duplicated for the actor"
        );
        let sends = match all.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected regen delivery: {other:?}"),
        };
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].0, observer);
        assert_eq!(
            sends[0].1,
            format!(
                "{}\r\n죽은몹이 다시 나타납니다.\r\n\r\n\x1b[0;37;40m[ 22/33, 5/8 ] ",
                crate::script::RAW_USER_MESSAGE_PREFIX
            )
        );
        {
            let world = get_world_state().read().unwrap();
            let mobs = world.mob_cache.get_all_mobs_in_room(&zone, room);
            let revived = mobs.iter().find(|mob| mob.mob_key == dead_key).unwrap();
            assert!(revived.alive);
            assert_eq!(revived.act, 0);
            assert_eq!(revived.hp, 321);
        }
        let no_corpses = storage
            .execute("리젠", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(no_corpses.0, vec!["☞ 리젠될 몹이 없어요!!"]);
        let live_target = storage
            .execute("리젠", &mut body, "산몹", None, None, None)
            .unwrap();
        assert_eq!(live_target.0, vec!["☞ 시체만 가능합니다. *^_^*"]);

        let numeric_live_target = storage
            .execute("리젠", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(numeric_live_target.0, vec!["☞ 시체만 가능합니다. *^_^*"]);

        {
            let mut world = get_world_state().write().unwrap();
            let revived = world
                .mob_cache
                .get_all_mobs_in_room_mut(&zone, room)
                .unwrap()
                .iter_mut()
                .find(|mob| mob.mob_key == dead_key)
                .unwrap();
            revived.kill();
        }
        let numbered_corpse = storage
            .execute("리젠", &mut body, "1시체", None, None, None)
            .unwrap();
        assert_eq!(
            numbered_corpse.0,
            vec!["\r\n죽은몹이 다시 나타납니다."],
            "개별 리젠은 Python처럼 완료 확인문을 출력하지 않는다"
        );
        let sends = match numbered_corpse.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected selected regen delivery: {other:?}"),
        };
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].0, observer);
        assert_eq!(
            sends[0].1,
            format!(
                "{}\r\n죽은몹이 다시 나타납니다.\r\n\r\n\x1b[0;37;40m[ 22/33, 5/8 ] ",
                crate::script::RAW_USER_MESSAGE_PREFIX
            )
        );

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.remove_player_position(&observer);
        world.mob_cache.remove_mob(&dead_key);
        world.mob_cache.remove_mob(&live_key);
        drop(world);
        set_precomputed_party_context(rhai::Map::new());
    }

    #[test]
    fn regen_target_lookup_matches_python_numeric_corpse_and_alias_rules() {
        let zone = "리젠선택규칙존";
        let room = "1";
        let live_key = format!("{zone}:산몹");
        let dead_one_key = format!("{zone}:죽은몹1");
        let dead_two_key = format!("{zone}:죽은몹2");
        let hidden_number_key = format!("{zone}:유형7");
        let mut metadata = HashMap::new();

        let mut live_data = crate::world::RawMobData::new();
        live_data.name = "산몹".into();
        live_data.reaction_names = vec!["산몹별칭".into()];
        let live = crate::world::MobInstance::new(live_key.clone(), zone.into(), room, &live_data);
        metadata.insert(live_key, live_data);

        let mut dead_one_data = crate::world::RawMobData::new();
        dead_one_data.name = "죽은몹1".into();
        let mut dead_one =
            crate::world::MobInstance::new(dead_one_key.clone(), zone.into(), room, &dead_one_data);
        dead_one.act = 2;
        metadata.insert(dead_one_key, dead_one_data);

        let mut dead_two_data = crate::world::RawMobData::new();
        dead_two_data.name = "죽은몹2".into();
        let mut dead_two =
            crate::world::MobInstance::new(dead_two_key.clone(), zone.into(), room, &dead_two_data);
        dead_two.act = 2;
        metadata.insert(dead_two_key, dead_two_data);

        let mut type_seven_data = crate::world::RawMobData::new();
        type_seven_data.name = "유형7".into();
        type_seven_data.mob_type = 7;
        let type_seven = crate::world::MobInstance::new(
            hidden_number_key.clone(),
            zone.into(),
            room,
            &type_seven_data,
        );
        metadata.insert(hidden_number_key, type_seven_data);

        let mobs = vec![type_seven, live, dead_one, dead_two];
        assert_eq!(python_room_mob_index(&mobs, &metadata, "1"), Some(1));
        assert_eq!(python_room_mob_index(&mobs, &metadata, "."), Some(1));
        assert_eq!(python_room_mob_index(&mobs, &metadata, "1 무시됨"), Some(1));
        assert_eq!(python_room_mob_index(&mobs, &metadata, "산몹별"), Some(1));
        assert_eq!(python_room_mob_index(&mobs, &metadata, "시체"), Some(2));
        assert_eq!(python_room_mob_index(&mobs, &metadata, "2시체"), Some(3));
    }

    #[test]
    fn item_generation_keeps_python_zero_and_large_count_semantics() {
        let mut body = Body::new();
        let zero = create_items(&mut body, "사강시", 0);
        assert_eq!(zero["status"].clone().into_string().unwrap(), "ok");
        assert!(body.object.objs.is_empty());

        let many = create_items(&mut body, "사강시", 101);
        assert_eq!(many["status"].clone().into_string().unwrap(), "ok");
        assert_eq!(body.object.objs.len(), 101);
        assert!(body.object.objs.iter().all(|item| {
            item.lock()
                .is_ok_and(|item| item.getString("종류") == "호위" && item.getInt("체력") == 1400)
        }));

        body.object.objs.clear();
        body.set("관리자등급", 2000_i64);
        let invalid = ScriptStorage::default()
            .execute("생성", &mut body, "사강시 잘못", None, None, None)
            .unwrap();
        assert!(invalid.0.is_empty());
        assert!(body.object.objs.is_empty());

        let key = format!("생성단일회귀-{}", std::process::id());
        let path = format!("data/item/{key}.json");
        std::fs::write(
            &path,
            r#"{"아이템정보":{"이름":"천상패","아이템속성":["단일아이템"]}}"#,
        )
        .unwrap();
        assert!(crate::oneitem::oneitem_have(&key, "기존소유자"));
        let duplicate = ScriptStorage::default()
            .execute("생성", &mut body, &key, None, None, None)
            .unwrap();
        assert_eq!(
            duplicate.0,
            vec!["[단일아이템] \x1b[0;36m천상패\x1b[37m가 이미 생성되어 있습니다."]
        );
        assert!(body.object.objs.is_empty());
        let _ = crate::oneitem::oneitem_destroy(&key);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mob_generation_clones_template_into_every_python_location_not_admin_room() {
        let suffix = std::process::id();
        let zone = format!("몹생성존{suffix}");
        let key = format!("{zone}:생성대상");
        let room_dir = std::path::Path::new("data/map").join(&zone);
        std::fs::create_dir_all(&room_dir).unwrap();
        for room in ["1", "2"] {
            std::fs::write(
                room_dir.join(format!("{room}.json")),
                format!(r#"{{"맵정보":{{"이름":"시험방{room}","존이름":"{zone}","설명":[],"출구":[]}}}}"#),
            )
            .unwrap();
        }
        let mut data = crate::world::RawMobData::new();
        data.name = "두방몹".into();
        data.zone = zone.clone();
        data.locations = vec!["1".into(), "2".into()];
        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .insert_mob_data(key.clone(), data);

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        let admin = format!("몹생성관리자{suffix}");
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        get_world_state().write().unwrap().set_player_position(
            &admin,
            crate::world::PlayerPosition::new(zone.clone(), "1".into()),
        );
        let usage = storage
            .execute("몹생성", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["사용법: [몹 이름] 생성"]);
        let created = storage
            .execute("몹생성", &mut body, &key, None, None, None)
            .unwrap();
        assert_eq!(
            created.0,
            vec!["\x1b[1;32m* [두방몹] 생성 되었습니다.\x1b[0;37m"]
        );
        {
            let mut world = get_world_state().write().unwrap();
            assert_eq!(world.mob_cache.get_all_mobs_in_room(&zone, "1").len(), 1);
            assert_eq!(world.mob_cache.get_all_mobs_in_room(&zone, "2").len(), 1);
            let template = world.mob_cache.get_mob(&key).unwrap().clone();
            world
                .mob_cache
                .add_mob_instance(crate::world::MobInstance::new(
                    key.clone(),
                    zone.clone(),
                    "1",
                    &template,
                ));
        }
        let numbered = storage
            .execute("몹제거", &mut body, "2", None, None, None)
            .unwrap();
        assert_eq!(numbered.0, vec!["몹이 제거되었습니다."]);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, "1")
                .len(),
            1,
            "Python removes only the selected duplicate object"
        );
        let removed = storage
            .execute("몹제거", &mut body, "두방몹", None, None, None)
            .unwrap();
        assert_eq!(removed.0, vec!["몹이 제거되었습니다."]);
        {
            let world = get_world_state().read().unwrap();
            assert!(world.mob_cache.get_all_mobs_in_room(&zone, "1").is_empty());
            assert_eq!(world.mob_cache.get_all_mobs_in_room(&zone, "2").len(), 1);
        }
        {
            let mut world = get_world_state().write().unwrap();
            let template = world.mob_cache.get_mob(&key).unwrap().clone();
            let mut corpse =
                crate::world::MobInstance::new(key.clone(), zone.clone(), "1", &template);
            corpse.kill();
            world.mob_cache.add_mob_instance(corpse);
        }
        let corpse_removed = storage
            .execute("몹제거", &mut body, "시체", None, None, None)
            .unwrap();
        assert_eq!(corpse_removed.0, vec!["몹이 제거되었습니다."]);
        assert!(get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, "1")
            .is_empty());
        let failed = storage
            .execute("몹생성", &mut body, "없는존:없는몹", None, None, None)
            .unwrap();
        assert_eq!(failed.0, vec!["* 생성 실패!!!"]);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&admin);
        world.mob_cache.remove_instance(&zone, "1", &key);
        world.mob_cache.remove_instance(&zone, "2", &key);
        world.mob_cache.remove_mob(&key);
        drop(world);
        let _ = std::fs::remove_dir_all(room_dir);
    }

    #[test]
    fn self_status_rhai_uses_full_python_table_instead_of_summary() {
        let zone = format!("본인상태존{}", std::process::id());
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "상태본인");
        body.set("관리자등급", 1000_i64);
        body.set("레벨", 10_i64);
        body.set("체력", 321_i64);
        body.set("최고체력", 450_i64);
        body.set("내공", 123_i64);
        body.set("최고내공", 234_i64);
        get_world_state().write().unwrap().set_player_position(
            "상태본인",
            crate::world::PlayerPosition::new(zone, "1".into()),
        );
        let (output, _) = storage
            .execute("상태보기", &mut body, "상태본인", None, None, None)
            .unwrap();
        assert!(output.iter().any(|line| line.contains("[命  中]")));
        assert!(output.iter().any(|line| line.contains("[은  전]")));
        assert!(output
            .iter()
            .any(|line| line.contains("표국보험은 효력이 없습니다.")));
        assert!(output.iter().any(|line| {
            line == "★ \x1b[1m상태본인\x1b[0;37m의 표국보험은 효력이 없습니다."
        }));
        assert!(!output.iter().any(|line| line.starts_with("◆")));
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position("상태본인");
    }

    #[test]
    fn room_player_status_uses_the_same_full_renderer() {
        let suffix = std::process::id();
        let zone = format!("타인상태존{suffix}");
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "관리자");
        body.set("관리자등급", 1000_i64);
        body.set("체력", 1_i64);
        body.set("최고체력", 100_i64);
        let snapshot = serde_json::json!([{
            "name":"대상자", "level":20, "age":30, "hp":400, "max_hp":600,
            "mp":200, "max_mp":300, "attack":12, "strength":44, "armor":8,
            "arm":33, "dex":55, "weight":70, "current_exp":10, "total_exp":999,
            "hit":7, "miss":6, "critical":5, "luck":4, "silver":1234,
            "성격":"정파", "성별":"남", "소속":"무문", "직위":"문주", "배우자":"",
            "feature":2, "insurance_premium":0, "hp_script":"은 건강합니다.",
            "mp_script":"의 내공은 충만합니다.", "guards":[], "anger":0
        }]);
        body.temp_mut().insert(
            "_online_room_admin".to_string(),
            crate::object::Value::String(snapshot.to_string()),
        );
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                "관리자",
                crate::world::PlayerPosition::new(zone.clone(), "1".into()),
            );
            world.set_player_position(
                "대상자",
                crate::world::PlayerPosition::new(zone, "1".into()),
            );
        }
        let (output, _) = storage
            .execute("상태보기", &mut body, "대상자", None, None, None)
            .unwrap();
        assert!(output
            .iter()
            .any(|line| line.contains("대상자의 현재 상태")));
        assert!(output.iter().any(|line| line.contains("[命  中]")));
        assert!(output.iter().any(|line| line.contains("2개의 여유 특성치")));
        assert!(output.iter().any(|line| {
            line == "★ \x1b[1m대상자\x1b[0;37m는 2개의 여유 특성치를 보유하고 있습니다."
        }));
        assert!(
            output
                .iter()
                .any(|line| line.contains("대상자에게 저승사자가 손짓")),
            "Python status line uses the administrator's 1/100 HP, not target 400/600"
        );
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position("관리자");
        world.remove_player_position("대상자");
    }

    #[test]
    fn mob_status_uses_runtime_instance_and_prints_python_index_line() {
        let suffix = std::process::id();
        let player = format!("몹상태관리자{suffix}");
        let zone = format!("몹상태존{suffix}");
        let room = "1";
        let key = format!("{zone}:표본");
        let mut data = crate::world::RawMobData::new();
        data.name = "상태표본".to_string();
        data.level = 12;
        data.max_hp = 900;
        data.strength = 31;
        data.attributes
            .insert("성별".to_string(), serde_json::json!("남"));
        let mut instance = crate::world::MobInstance::new(key.clone(), zone.clone(), room, &data);
        instance.hp = 456;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(instance);
            world.set_player_position(
                &player,
                crate::world::PlayerPosition::new(zone.clone(), room.to_string()),
            );
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player.clone());
        body.set("관리자등급", 1000_i64);
        let (output, _) = storage
            .execute("상태보기", &mut body, "상태표본", None, None, None)
            .unwrap();
        assert_eq!(output[0], format!("Index : {key}"));
        assert!(output.iter().any(|line| line.contains("456/900")));
        assert!(output
            .iter()
            .any(|line| line.contains("상태표본의 현재 상태")));
        assert!(output.iter().any(|line| {
            line == "★ \x1b[33m상태표본\x1b[37m의 표국보험은 효력이 없습니다."
        }));
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_instance(&zone, room, &key);
        world.mob_cache.remove_mob(&key);
    }

    #[test]
    fn summon_clear_removes_both_sides_of_combat_relationship() {
        let suffix = std::process::id();
        let name = format!("소환전투해제{suffix}");
        let zone = format!("소환전투존{suffix}");
        let key = format!("{zone}:몹");
        let mut data = crate::world::RawMobData::new();
        data.name = "소환대상몹".to_string();
        let mut mob = crate::world::MobInstance::new(key.clone(), zone.clone(), "1", &data);
        mob.act = 1;
        mob.targets.push(name.clone());
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(mob);
        }
        let mut body = Body::new();
        body.set("이름", name.clone());
        body.act = crate::player::ActState::Fight;
        body.temp_mut().insert(
            "_combat_target_ids".to_string(),
            crate::object::Value::String(key.clone()),
        );
        clear_summon_combat(&mut body);
        assert_eq!(body.act, crate::player::ActState::Stand);
        assert!(!body.temp().contains_key("_combat_target_ids"));
        let mut world = get_world_state().write().unwrap();
        let mob = world.mob_cache.get_all_mobs_in_room(&zone, "1")[0];
        assert!(mob.targets.is_empty());
        assert_eq!(mob.act, 0);
        world.mob_cache.remove_instance(&zone, "1", &key);
        world.mob_cache.remove_mob(&key);
    }

    #[test]
    fn mob_recover_accepts_python_corpse_query_and_uses_template_hp() {
        use crate::world::{MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let player = format!("몹회복관리자-{suffix}");
        let zone = format!("몹회복존-{suffix}");
        let key = format!("{zone}:회복대상");
        let mut data = RawMobData::new();
        data.name = "회복시험몹".into();
        data.zone = zone.clone();
        data.hp = 123;
        data.max_hp = 500;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data.clone());
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
            let mut instance = MobInstance::new(key.clone(), zone.clone(), "1", &data);
            instance.hp = 0;
            instance.act = 2;
            instance.alive = false;
            let id = instance.instance_id;
            world.mob_cache.add_mob_instance(instance);
            world.record_test_room_object(&zone, "1", crate::world::RoomObjectRef::Mob(id));
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("관리자등급", 1000_i64);
        let result = ScriptStorage::default()
            .execute("몹회복", &mut body, "시체", None, None, None)
            .unwrap();
        assert_eq!(result.0, vec!["* 회복되었습니다."]);
        let world = get_world_state().read().unwrap();
        let mob = world.mob_cache.get_all_mobs_in_room(&zone, "1")[0];
        assert_eq!(mob.hp, 123);
        assert_eq!(mob.act, 2, "Python only assigns hp and leaves corpse state");
        drop(world);
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&key);
    }

    #[test]
    fn mob_recover_uses_python_numbered_prefix_and_dot_selection() {
        use crate::world::{MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

        let suffix = std::process::id();
        let player = format!("몹회복순번관리자-{suffix}");
        let zone = format!("몹회복순번존-{suffix}");
        let key = format!("{zone}:회복순번몹");
        let mut data = RawMobData::new();
        data.name = "회복순번몹".into();
        data.reaction_names = vec!["회복별칭".into()];
        data.zone = zone.clone();
        data.hp = 222;
        let mut first = MobInstance::new(key.clone(), zone.clone(), "1", &data);
        first.hp = 10;
        let first_id = first.instance_id;
        let mut second = MobInstance::new(key.clone(), zone.clone(), "1", &data);
        second.hp = 20;
        let second_id = second.instance_id;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(first);
            world.mob_cache.add_mob_instance(second);
            world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(first_id));
            world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(second_id));
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("관리자등급", 1000_i64);

        let numbered = storage
            .execute("몹회복", &mut body, "2회복", None, None, None)
            .unwrap();
        assert_eq!(numbered.0, vec!["* 회복되었습니다."]);
        {
            let world = get_world_state().read().unwrap();
            let mobs = world.mob_cache.get_all_mobs_in_room(&zone, "1");
            assert_eq!(mobs[0].hp, 20);
            assert_eq!(mobs[1].hp, 222);
        }
        let dot = storage
            .execute("몹회복", &mut body, ".", None, None, None)
            .unwrap();
        assert_eq!(dot.0, vec!["* 회복되었습니다."]);
        {
            let world = get_world_state().read().unwrap();
            let mobs = world.mob_cache.get_all_mobs_in_room(&zone, "1");
            assert_eq!(mobs[0].hp, 222);
            assert_eq!(mobs[1].hp, 222);
        }

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&key);
    }
}
