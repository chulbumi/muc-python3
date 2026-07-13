//! State/data efun for Python `cmds/뒤져.py`.
//!
//! Rhai owns all visible text. This module resolves a room mob and transfers
//! its ordered child objects while preserving Python's capacity checks.

use crate::player::Body;
use crate::world::get_world_state;
use rhai::{Array, Dynamic, Engine, Map};
use std::collections::HashMap;

fn matches_name(name: &str, aliases: &[String], query: &str) -> bool {
    name == query
        || aliases
            .iter()
            .any(|alias| alias == query || alias.starts_with(query))
}

fn search_mob(body: &mut Body, query: &str) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from("missing"));
    let Ok(mut world) = get_world_state().write() else {
        return result;
    };
    let Some(position) = world.get_player_position(&body.get_name()).cloned() else {
        return result;
    };
    if super::admin_combat::python_named_room_selection_is_nonmob(
        &world,
        &position.zone,
        &position.room,
        query,
    ) {
        return result;
    }
    let metadata = world
        .mob_cache
        .get_all_mobs_in_room(&position.zone, &position.room)
        .into_iter()
        .filter_map(|mob| {
            world
                .mob_cache
                .get_mob(&mob.mob_key)
                .cloned()
                .map(|data| (mob.mob_key.clone(), data))
        })
        .collect::<HashMap<_, _>>();
    let Some(mobs) = world
        .mob_cache
        .get_all_mobs_in_room_mut(&position.zone, &position.room)
    else {
        return result;
    };

    let query = if query.trim() == "." { "1" } else { query };
    let numeric_order = query.parse::<usize>().ok().filter(|order| *order > 0);
    // Room.findObjName's pure numeric mode selects the Nth living, visible
    // mob. Named lookup ignores dead mobs unless the query is `시체`.
    let mut candidate = None;
    let mut numeric_seen = 0usize;
    for (index, mob) in mobs.iter().enumerate() {
        let Some(data) = metadata.get(&mob.mob_key) else {
            continue;
        };
        if let Some(order) = numeric_order {
            if mob.act != 2 && mob.act != 3 && data.mob_type != 7 {
                numeric_seen += 1;
                if numeric_seen == order {
                    candidate = Some(index);
                    break;
                }
            }
        } else if query == "시체" {
            if mob.act == 2 {
                candidate = Some(index);
                break;
            }
        } else if mob.act != 2
            && mob.act != 3
            && matches_name(&data.name, &data.reaction_names, query)
        {
            candidate = Some(index);
            break;
        }
    }
    let Some(index) = candidate else {
        return result;
    };
    let mob = &mut mobs[index];
    let Some(data) = metadata.get(&mob.mob_key) else {
        return result;
    };
    let dead = mob.act == 2;
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("name".into(), Dynamic::from(data.name.clone()));
    result.insert("dead".into(), Dynamic::from(dead));
    result.insert(
        "searchable".into(),
        Dynamic::from(dead || data.mob_type == 6),
    );

    if !dead && data.mob_type != 6 {
        return result;
    }

    let had_items = !mob.inventory.is_empty();
    let mut taken = Array::new();
    while let Some(item) = mob.inventory.first().cloned() {
        let Ok(item_guard) = item.lock() else { break };
        let weight = item_guard.getInt("무게");
        let item_name = item_guard.getName();
        let item_ansi = item_guard.getString("안시");
        let one_item = item_guard.checkAttr("아이템속성", "단일아이템");
        let item_index = item_guard.getString("인덱스");
        drop(item_guard);
        let max_items = super::get_murim_config_int("사용자아이템갯수").max(0) as usize;
        if body.get_item_count() >= max_items
            || body.get_item_weight().saturating_add(weight) > body.get_str() * 10
        {
            break;
        }
        mob.inventory.remove(0);
        // Python Object.insert() prepends each transferred object. Multiple
        // corpse items therefore appear in reverse transfer order ahead of
        // the player's pre-existing inventory.
        body.object.objs.insert(0, item);
        if one_item {
            crate::oneitem::oneitem_have(&item_index, &body.get_name());
        }
        let mut item_data = Map::new();
        item_data.insert("이름".into(), Dynamic::from(item_name));
        item_data.insert("안시".into(), Dynamic::from(item_ansi));
        taken.push(Dynamic::from(item_data));
    }
    // Python returns before this assignment for an already-empty corpse, but
    // does refresh it when an item existed and capacity rejected the first
    // transfer.
    if had_items {
        mob.time_of_regen = chrono::Utc::now().timestamp();
    }
    result.insert("items".into(), Dynamic::from(taken));
    result
}

pub(super) fn register_search_body_efun(engine: &mut Engine, body_ptr: *mut Body) {
    engine.register_fn(
        "search_room_mob",
        move |_ob: &mut Map, query: &str| -> Map { search_mob(unsafe { &mut *body_ptr }, query) },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::handler::CommandResult;
    use crate::object::Object;
    use crate::script::party::set_precomputed_party_context;
    use crate::script::ScriptStorage;
    use crate::world::{MobInstance, PlayerPosition, RawMobData};
    use std::sync::{Arc, Mutex};

    #[test]
    fn rhai_search_usage_is_the_python_message() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "뒤져사용법검사");
        let (output, special) = storage
            .execute("뒤져", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 사용법: [대상] 뒤져"]);
        assert!(special.is_none());
    }

    #[test]
    fn corpse_search_moves_items_and_preserves_python_ansi_particles_and_order() {
        let suffix = std::process::id();
        let player_name = format!("뒤져회귀-{suffix}");
        let observer_name = format!("뒤져목격-{suffix}");
        let zone = format!("뒤져회귀존-{suffix}");
        let mob_key = format!("{zone}:시험몹");
        let mut data = RawMobData::new();
        data.name = "시험몹".into();
        data.zone = zone.clone();
        let mut mob = MobInstance::new(mob_key.clone(), zone.clone(), "1", &data);
        mob.act = 2;
        let mut first = Object::new();
        first.set("이름", "구슬");
        first.set("반응이름", "구슬");
        first.set("무게", 1_i64);
        let mut second = Object::new();
        second.set("이름", "부적");
        second.set("반응이름", "부적");
        second.set("무게", 1_i64);
        second.set("안시", "\x1b[35m");
        mob.inventory.push(Arc::new(Mutex::new(first)));
        mob.inventory.push(Arc::new(Mutex::new(second)));
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(mob_key.clone(), data);
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(&player_name, PlayerPosition::new(zone.clone(), "1".into()));
            world.set_player_position(
                &observer_name,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("힘", 100_i64);
        let mut existing = Object::new();
        existing.set("이름", "기존품");
        body.object.objs.push(Arc::new(Mutex::new(existing)));
        let mut person = rhai::Map::new();
        person.insert("name".into(), rhai::Dynamic::from(observer_name.clone()));
        person.insert("show_prompt".into(), rhai::Dynamic::from(true));
        person.insert("hp".into(), rhai::Dynamic::from(21_i64));
        person.insert("max_hp".into(), rhai::Dynamic::from(31_i64));
        person.insert("mp".into(), rhai::Dynamic::from(4_i64));
        person.insert("max_mp".into(), rhai::Dynamic::from(9_i64));
        let mut context = rhai::Map::new();
        context.insert(
            "room_players".into(),
            rhai::Dynamic::from(vec![rhai::Dynamic::from(person)]),
        );
        set_precomputed_party_context(context);
        let storage = ScriptStorage::default();
        let result = storage
            .execute("뒤져", &mut body, "시체", None, None, None)
            .unwrap();
        assert_eq!(
            result.0,
            &[
                "당신이 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[0;36m구슬\x1b[37m을 뒤져서 가집니다.",
                "당신이 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[35m부적\x1b[0;37m을 뒤져서 가집니다."
            ]
        );
        let sends = match result.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected search delivery: {other:?}"),
        };
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].0, observer_name);
        assert_eq!(sends[0].1, format!(
            "{}\r\n\x1b[1m{player_name}\x1b[0;37m가 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[0;36m구슬\x1b[37m을 뒤져서 가집니다.\r\n\x1b[1m{player_name}\x1b[0;37m가 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[35m부적\x1b[0;37m을 뒤져서 가집니다.\r\n\r\n\x1b[0;37;40m[ 21/31, 4/9 ] ",
            crate::script::RAW_USER_MESSAGE_PREFIX,
        ));
        assert_eq!(body.object.objs.len(), 3);
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "부적");
        assert_eq!(body.object.objs[1].lock().unwrap().getName(), "구슬");
        assert_eq!(body.object.objs[2].lock().unwrap().getName(), "기존품");
        {
            let world = get_world_state().read().unwrap();
            let mobs = world.mob_cache.get_all_mobs_in_room(&zone, "1");
            assert!(mobs[0].inventory.is_empty());
            assert!(mobs[0].time_of_regen > 0);
        }

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.remove_player_position(&observer_name);
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        set_precomputed_party_context(rhai::Map::new());
    }

    #[test]
    fn empty_corpse_does_not_refresh_regen_but_rejected_loot_does() {
        let suffix = std::process::id();
        let player_name = format!("뒤져빈시체-{suffix}");
        let zone = format!("뒤져빈시체존-{suffix}");
        let empty_key = format!("{zone}:빈시체");
        let blocked_key = format!("{zone}:무거운시체");

        let mut empty_data = RawMobData::new();
        empty_data.name = "빈시체몹".into();
        empty_data.zone = zone.clone();
        let mut empty = MobInstance::new(empty_key.clone(), zone.clone(), "1", &empty_data);
        empty.act = 2;
        empty.time_of_regen = 123;

        let mut blocked_data = RawMobData::new();
        blocked_data.name = "무거운시체몹".into();
        blocked_data.zone = zone.clone();
        let mut blocked = MobInstance::new(blocked_key.clone(), zone.clone(), "2", &blocked_data);
        blocked.act = 2;
        let mut stone = Object::new();
        stone.set("이름", "무거운돌");
        stone.set("무게", 1_i64);
        blocked.inventory.push(Arc::new(Mutex::new(stone)));

        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(empty_key.clone(), empty_data);
            world
                .mob_cache
                .insert_mob_data(blocked_key.clone(), blocked_data);
            world.mob_cache.add_mob_instance(empty);
            world.mob_cache.add_mob_instance(blocked);
            world.set_player_position(&player_name, PlayerPosition::new(zone.clone(), "1".into()));
        }

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("힘", 0_i64);

        let empty_output = storage
            .execute("뒤져", &mut body, "시체", None, None, None)
            .unwrap();
        assert_eq!(
            empty_output.0[0],
            "당신이 \x1b[33m빈시체몹\x1b[37m의 시체를 뒤집니다. '뒤적~ 뒤적~'"
        );
        {
            let world = get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room(&zone, "1")
                .into_iter()
                .find(|mob| mob.mob_key == empty_key)
                .unwrap();
            assert_eq!(mob.time_of_regen, 123);
        }

        get_world_state()
            .write()
            .unwrap()
            .set_player_position(&player_name, PlayerPosition::new(zone.clone(), "2".into()));
        let rejected = storage
            .execute("뒤져", &mut body, "시체", None, None, None)
            .unwrap();
        assert_eq!(
            rejected.0[0],
            "당신이 \x1b[33m무거운시체몹\x1b[37m의 시체를 뒤집니다. '뒤적~ 뒤적~'"
        );
        {
            let world = get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room(&zone, "2")
                .into_iter()
                .find(|mob| mob.mob_key == blocked_key)
                .unwrap();
            assert!(mob.time_of_regen > 0);
            assert_eq!(mob.inventory.len(), 1);
        }

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_mob(&empty_key);
        world.mob_cache.remove_mob(&blocked_key);
    }

    #[test]
    fn numeric_search_skips_hidden_mobs_and_dot_means_first_like_python_room_lookup() {
        let suffix = std::process::id();
        let player_name = format!("뒤져숫자-{suffix}");
        let zone = format!("뒤져숫자존-{suffix}");
        let visible_key = format!("{zone}:보이는몹");
        let hidden_key = format!("{zone}:숨은몹");
        let mut visible_data = RawMobData::new();
        visible_data.name = "보이는몹".into();
        visible_data.zone = zone.clone();
        let mut hidden_data = RawMobData::new();
        hidden_data.name = "숨은몹".into();
        hidden_data.zone = zone.clone();
        hidden_data.mob_type = 7;
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(visible_key.clone(), visible_data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                visible_key.clone(),
                zone.clone(),
                "1",
                &visible_data,
            ));
            world
                .mob_cache
                .insert_mob_data(hidden_key.clone(), hidden_data.clone());
            // Mob insertion prepends, putting type-7 ahead of the visible mob.
            world.mob_cache.add_mob_instance(MobInstance::new(
                hidden_key.clone(),
                zone.clone(),
                "1",
                &hidden_data,
            ));
            world.set_player_position(&player_name, PlayerPosition::new(zone.clone(), "1".into()));
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        let storage = ScriptStorage::default();
        for query in ["1", "."] {
            let output = storage
                .execute("뒤져", &mut body, query, None, None, None)
                .unwrap()
                .0;
            assert_eq!(
                output[0],
                "당신이 \x1b[33m보이는몹\x1b[37m의 몸을 더듬습니다. \"뭐 없나~~ -.-\""
            );
        }

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_mob(&visible_key);
        world.mob_cache.remove_mob(&hidden_key);
    }

    #[test]
    fn named_search_stops_at_floor_item_before_matching_mob() {
        use crate::world::RoomObjectRef;

        let suffix = std::process::id();
        let player = format!("뒤져충돌-{suffix}");
        let zone = format!("뒤져충돌존-{suffix}");
        let key = format!("{zone}:뒤져몹");
        let mut data = RawMobData::new();
        data.name = "뒤져몹".into();
        data.reaction_names = vec!["공통뒤져별칭".into()];
        data.zone = zone.clone();
        data.mob_type = 6;
        let mob = MobInstance::new(key.clone(), zone.clone(), "1", &data);
        let mob_id = mob.instance_id;
        let mut item = Object::new();
        item.set("이름", "뒤져충돌패");
        item.set("반응이름", "공통뒤져별칭");
        let item = Arc::new(Mutex::new(item));
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(mob);
            world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(mob_id));
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
            world.get_room_objs_mut(&zone, "1").push(item.clone());
            // Room.insert prepends, so the item is selected before the mob.
            world.record_floor_item(&zone, "1", &item);
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        let output = ScriptStorage::default()
            .execute("뒤져", &mut body, "공통뒤져별칭", None, None, None)
            .unwrap();
        assert_eq!(output.0, vec!["☞ 뒤질대상이 없어요. ^^"]);

        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, "1", &item);
        world.get_room_objs_mut(&zone, "1").clear();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&key);
    }
}
