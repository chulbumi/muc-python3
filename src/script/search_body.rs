//! State/data efun for Python `cmds/뒤져.py`.
//!
//! Rhai owns all visible text. This module resolves a room mob and transfers
//! its ordered child objects while preserving Python's capacity checks.

use crate::player::Body;
use crate::world::get_world_state;
use rhai::{Array, Dynamic, Engine, Map};
use std::collections::HashMap;

const MAX_PLAYER_ITEMS: usize = 300;

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

    // Room.findObjName ignores dead mobs unless the query is `시체`.
    let mut candidate = None;
    for (index, mob) in mobs.iter().enumerate() {
        let Some(data) = metadata.get(&mob.mob_key) else {
            continue;
        };
        if query == "시체" {
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

    let mut taken = Array::new();
    while let Some(item) = mob.inventory.first().cloned() {
        let Ok(item_guard) = item.lock() else { break };
        let weight = item_guard.getInt("무게");
        let item_name = item_guard.getName();
        let item_ansi = item_guard.getString("안시");
        let one_item = item_guard.checkAttr("아이템속성", "단일아이템");
        let item_index = item_guard.getString("인덱스");
        drop(item_guard);
        if body.get_item_count() >= MAX_PLAYER_ITEMS
            || body.get_item_weight().saturating_add(weight) > body.get_str() * 10
        {
            break;
        }
        mob.inventory.remove(0);
        body.object.objs.push(item);
        if one_item {
            crate::oneitem::oneitem_have(&item_index, &body.get_name());
        }
        let mut item_data = Map::new();
        item_data.insert("이름".into(), Dynamic::from(item_name));
        item_data.insert("안시".into(), Dynamic::from(item_ansi));
        taken.push(Dynamic::from(item_data));
    }
    // Python updates timeofregen even when capacity prevents the first item.
    mob.time_of_regen = chrono::Utc::now().timestamp();
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
    use crate::object::Object;
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
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("힘", 100_i64);
        let storage = ScriptStorage::default();
        let result = storage
            .execute("뒤져", &mut body, "시체", None, None, None)
            .unwrap();
        assert_eq!(
            &result.0[..2],
            &[
                "당신이 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[0;36m구슬\x1b[37m을 뒤져서 가집니다.",
                "당신이 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[35m부적\x1b[0;37m을 뒤져서 가집니다."
            ]
        );
        assert_eq!(
            result.0[2],
            format!(
                "\x1b[1m{player_name}\x1b[0;37m가 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[0;36m구슬\x1b[37m을 뒤져서 가집니다.\r\n\x1b[1m{player_name}\x1b[0;37m가 \x1b[33m시험몹\x1b[37m의 시체속에서 \x1b[35m부적\x1b[0;37m을 뒤져서 가집니다."
            )
        );
        assert_eq!(body.object.objs.len(), 2);
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "구슬");
        assert_eq!(body.object.objs[1].lock().unwrap().getName(), "부적");
        {
            let world = get_world_state().read().unwrap();
            let mobs = world.mob_cache.get_all_mobs_in_room(&zone, "1");
            assert!(mobs[0].inventory.is_empty());
            assert!(mobs[0].time_of_regen > 0);
        }

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_mob(&mob_key);
    }
}
