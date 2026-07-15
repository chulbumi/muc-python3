//! State/data efuns for Python `cmds/분노.py`.
//! Visible combat scripts and ANSI are rendered by Rhai.

use super::cast::with_room_player_body_mut;
use super::combat_commands::combat_target_instance_ids;
use super::current_body_position;
use crate::player::{ActState, Body};
use crate::world::get_world_state;
use rand::Rng;
use rhai::{Array, Dynamic, Engine, Map};

fn guards(body: &Body) -> Vec<std::sync::Arc<std::sync::Mutex<crate::object::Object>>> {
    body.object
        .objs
        .iter()
        .filter(|item| {
            item.lock()
                .is_ok_and(|item| item.getString("종류") == "호위")
        })
        .cloned()
        .collect()
}

fn guard_count(body: &Body) -> i64 {
    guards(body).len() as i64
}

fn primary_target(body: &Body) -> Map {
    let mut result = Map::new();
    result.insert("found".into(), Dynamic::from(false));
    let Some((zone, room)) = current_body_position(body) else {
        return result;
    };
    let Ok(world) = get_world_state().read() else {
        return result;
    };
    let instance_ids = combat_target_instance_ids(body);
    let mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
    // Python iterates ob.target in combat-registration order and skips only
    // targets that are no longer in the current room. Room insertion order
    // (and especially its reverse) must not choose a different opponent.
    for instance_id in instance_ids {
        let Some((index, mob)) = mobs
            .iter()
            .enumerate()
            .find(|(_, mob)| mob.instance_id == instance_id && mob.alive)
        else {
            continue;
        };
        result.insert("found".into(), Dynamic::from(true));
        result.insert("id".into(), Dynamic::from(mob.mob_key.clone()));
        result.insert("instance_id".into(), Dynamic::from(mob.instance_id as i64));
        result.insert("name".into(), Dynamic::from(mob.name.clone()));
        result.insert("room_index".into(), Dynamic::from(index as i64));
        result.insert("current".into(), Dynamic::from(true));
        break;
    }
    result
}

fn run_anger(body: &mut Body, target_id: &str, instance_id: i64) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from("missing"));
    if body.act != ActState::Fight || body.get_int("분노") < 100 {
        return result;
    }
    let guards = guards(body);
    if guards.is_empty() {
        body.set("분노", body.get_int("분노") - 100);
        result.insert("status".into(), Dynamic::from("no_guards"));
        return result;
    }
    let Some((zone, room)) = current_body_position(body) else {
        return result;
    };
    let Ok(mut world) = get_world_state().write() else {
        return result;
    };
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return result;
    };
    let Some(mob) = mobs
        .iter_mut()
        .find(|mob| instance_id > 0 && mob.instance_id == instance_id as u64)
    else {
        return result;
    };
    if mob.mob_key != target_id
        || !mob.alive
        || !combat_target_instance_ids(body).contains(&mob.instance_id)
    {
        result.insert("status".into(), Dynamic::from("not_current"));
        return result;
    }
    body.set("분노", body.get_int("분노") - 100);
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("target".into(), Dynamic::from(mob.name.clone()));

    let mut events = Array::new();
    let mut rng = rand::thread_rng();
    for guard in guards {
        let Ok(mut guard) = guard.lock() else {
            continue;
        };
        let mut event = Map::new();
        event.insert("guard".into(), Dynamic::from(guard.getName()));
        event.insert("guard_ansi".into(), Dynamic::from(guard.getString("안시")));
        event.insert(
            "use_script".into(),
            Dynamic::from(guard.getString("사용스크립")),
        );
        event.insert(
            "attack_script".into(),
            Dynamic::from(guard.getString("공격스크립")),
        );
        event.insert(
            "fail_script".into(),
            Dynamic::from(guard.getString("실패스크립")),
        );
        let chance =
            100 + guard.getInt("명중력") - (mob.level - body.get_int("레벨") + 90).div_euclid(3);
        let mut stop_after_event = false;
        if guard.getInt("체력") < 1 || rng.gen_range(0..=99) > chance {
            event.insert("hit".into(), Dynamic::from(false));
            event.insert("damage".into(), Dynamic::from(0_i64));
        } else {
            let add_variance = rng.gen_range(0..=1) == 0;
            let variance = rng.gen_range(0..=9_i64);
            let base = body.get_str() as i64 * guard.getInt("공격력") / 100;
            let mut damage = if add_variance {
                base + variance
            } else {
                base - variance
            };
            damage = damage.max(1);
            let hp = (guard.getInt("체력") - damage * guard.getInt("체력감소") / 100).max(0);
            guard.set("체력", hp);
            if mob.hp <= 1 {
                damage = 0;
            }
            mob.hp -= damage;
            let below_zero = mob.hp < 0;
            if below_zero {
                mob.hp = 1;
                stop_after_event = true;
            }
            event.insert("hit".into(), Dynamic::from(true));
            event.insert("damage".into(), Dynamic::from(damage));
        }
        events.push(Dynamic::from(event));
        if stop_after_event {
            break;
        }
    }
    result.insert("events".into(), Dynamic::from(events));
    result
}

fn run_anger_player(body: &mut Body, target_name: &str) -> Map {
    let mut result = Map::new();
    result.insert("status".into(), Dynamic::from("missing"));
    if body.act != ActState::Fight || body.get_int("분노") < 100 {
        return result;
    }
    let guards = guards(body);
    if guards.is_empty() {
        body.set("분노", body.get_int("분노") - 100);
        result.insert("status".into(), Dynamic::from("no_guards"));
        return result;
    }
    if super::combat_commands::pvp_target(body).as_deref() != Some(target_name) {
        result.insert("status".into(), Dynamic::from("not_current"));
        return result;
    }
    let attacker_level = body.get_int("레벨");
    let attacker_strength = body.get_str() as i64;
    let applied = with_room_player_body_mut(target_name, |target| {
        let target_level = target.get_int("레벨");
        let target_display = target.get_name();
        let mut target_hp = target.get_hp();
        let mut events = Array::new();
        let mut rng = rand::thread_rng();
        for guard in guards {
            let Ok(mut guard) = guard.lock() else {
                continue;
            };
            let mut event = Map::new();
            event.insert("guard".into(), Dynamic::from(guard.getName()));
            event.insert("guard_ansi".into(), Dynamic::from(guard.getString("안시")));
            event.insert(
                "use_script".into(),
                Dynamic::from(guard.getString("사용스크립")),
            );
            event.insert(
                "attack_script".into(),
                Dynamic::from(guard.getString("공격스크립")),
            );
            event.insert(
                "fail_script".into(),
                Dynamic::from(guard.getString("실패스크립")),
            );
            let chance =
                100 + guard.getInt("명중력") - (target_level - attacker_level + 90).div_euclid(3);
            let mut stop_after_event = false;
            if guard.getInt("체력") < 1 || rng.gen_range(0..=99) > chance {
                event.insert("hit".into(), Dynamic::from(false));
                event.insert("damage".into(), Dynamic::from(0_i64));
            } else {
                let base = attacker_strength * guard.getInt("공격력") / 100;
                let add_variance = rng.gen_range(0..=1) == 0;
                let variance = rng.gen_range(0..=9_i64);
                let mut damage = if add_variance {
                    base + variance
                } else {
                    base - variance
                };
                damage = damage.max(1);
                let guard_hp =
                    (guard.getInt("체력") - damage * guard.getInt("체력감소") / 100).max(0);
                guard.set("체력", guard_hp);
                if target_hp <= 1 {
                    damage = 0;
                }
                target_hp -= damage;
                if target_hp < 0 {
                    target_hp = 1;
                    stop_after_event = true;
                }
                event.insert("hit".into(), Dynamic::from(true));
                event.insert("damage".into(), Dynamic::from(damage));
            }
            events.push(Dynamic::from(event));
            if stop_after_event {
                break;
            }
        }
        target.set("체력", target_hp);
        (target_display, events)
    });
    let Some((target_display, events)) = applied else {
        return result;
    };
    body.set("분노", body.get_int("분노") - 100);
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("target".into(), Dynamic::from(target_display));
    result.insert("events".into(), Dynamic::from(events));
    result
}

pub(super) fn register_anger_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    let ptr = body_ptr;
    engine.register_fn("anger_guard_count", move |_ob: &mut Map| {
        guard_count(unsafe { &*ptr })
    });
    let ptr = body_ptr;
    engine.register_fn("anger_primary_target", move |_ob: &mut Map| {
        primary_target(unsafe { &*ptr })
    });
    engine.register_fn(
        "run_guard_anger",
        move |_ob: &mut Map, id: &str, instance_id: i64| {
            run_anger(unsafe { &mut *body_ptr }, id, instance_id)
        },
    );
    let ptr = body_ptr;
    engine.register_fn(
        "run_guard_anger_player",
        move |_ob: &mut Map, name: &str| run_anger_player(unsafe { &mut *ptr }, name),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Value;
    use crate::script::ScriptStorage;
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};
    use std::sync::{Arc, Mutex};

    #[test]
    fn rhai_anger_rejects_noncombat_with_python_text() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        let (output, special) = storage
            .execute("분노", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(
            output,
            vec![
                "☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네"
            ]
        );
        assert!(special.is_none());
    }

    #[test]
    fn no_guard_branch_consumes_exactly_one_hundred_anger() {
        let mut body = Body::new();
        body.act = ActState::Fight;
        body.set("분노", 135_i64);
        let result = run_anger(&mut body, "", 0);
        assert_eq!(result["status"].clone().into_string().unwrap(), "no_guards");
        assert_eq!(body.get_int("분노"), 35);
    }

    #[test]
    fn implicit_target_missing_after_combat_entry_still_consumes_anger() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.act = ActState::Fight;
        body.set("분노", 135_i64);
        let mut guard = crate::object::Object::new();
        guard.set("종류", "호위");
        body.object.objs.push(Arc::new(Mutex::new(guard)));
        let dummy_target = Arc::new(Mutex::new(crate::object::Object::new()));
        body.targets.push(Arc::downgrade(&dummy_target));

        let (output, special) = storage
            .execute("분노", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 공격할 그런 대상이 없습니다."]);
        assert_eq!(body.get_int("분노"), 35);
        assert!(
            special.is_none(),
            "Python never sends its accumulated room msg"
        );
    }

    #[test]
    fn implicit_anger_target_uses_python_combat_registration_order() {
        let suffix = std::process::id();
        let player = format!("분노순서검사-{suffix}");
        let zone = format!("분노순서존-{suffix}");
        let room = "1";
        let first_key = format!("{zone}:첫대상");
        let second_key = format!("{zone}:둘째대상");
        let mut first_data = RawMobData::new();
        first_data.name = "첫대상".into();
        let mut second_data = RawMobData::new();
        second_data.name = "둘째대상".into();
        let first = MobInstance::new(first_key.clone(), zone.clone(), room, &first_data);
        let second = MobInstance::new(second_key.clone(), zone.clone(), room, &second_data);
        let first_id = first.instance_id;
        let second_id = second.instance_id;
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(first_key.clone(), first_data);
            world
                .mob_cache
                .insert_mob_data(second_key.clone(), second_data);
            // add_mob_instance prepends, so room order becomes second, first.
            // Combat order below is deliberately second, first: Python picks
            // second while the old reverse room scan picked first.
            world.mob_cache.add_mob_instance(first);
            world.mob_cache.add_mob_instance(second);
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.to_string()));
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.temp_mut().insert(
            "_combat_target_instance_ids".into(),
            Value::String(format!("{second_id}\n{first_id}")),
        );

        let selected = primary_target(&body);
        assert!(selected["found"].clone().as_bool().unwrap());
        assert_eq!(selected["id"].clone().into_string().unwrap(), second_key);
        assert_eq!(selected["room_index"].clone().as_int().unwrap(), 0);

        let mut world = get_world_state().write().unwrap();
        world.mob_cache.remove_instance(&zone, room, &first_key);
        world.mob_cache.remove_instance(&zone, room, &second_key);
        world.mob_cache.remove_mob_definition(&first_key);
        world.mob_cache.remove_mob_definition(&second_key);
        world.remove_player_position(&player);
    }

    #[test]
    fn explicit_anger_target_stops_at_python_first_matching_floor_item() {
        let suffix = std::process::id();
        let player = format!("분노충돌검사-{suffix}");
        let zone = format!("분노충돌존-{suffix}");
        let room = "1";
        let key = format!("{zone}:분노대상몹");
        let mut data = RawMobData::new();
        data.name = "분노대상몹".into();
        data.reaction_names = vec!["분노충돌".into()];
        data.zone = zone.clone();
        let mob = MobInstance::new(key.clone(), zone.clone(), room, &data);
        let mob_id = mob.instance_id;
        let mut item = crate::object::Object::new();
        item.set("이름", "분노충돌패");
        item.set("반응이름", "분노충돌");
        let item = Arc::new(Mutex::new(item));
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(mob);
            world.record_test_room_object(&zone, room, RoomObjectRef::Mob(mob_id));
            world.get_room_objs_mut(&zone, room).push(item.clone());
            // Room.insert prepends: the item is the first Python match.
            world.record_floor_item(&zone, room, &item);
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
        }
        let mut guard = crate::object::Object::new();
        guard.set("이름", "분노호위");
        guard.set("종류", "호위");
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("분노", 135_i64);
        body.act = ActState::Fight;
        body.object.objs.push(Arc::new(Mutex::new(guard)));
        body.temp_mut().insert(
            "_combat_target_instance_ids".into(),
            Value::String(mob_id.to_string()),
        );
        body.temp_mut()
            .insert("_combat_target_ids".into(), Value::String(key.clone()));
        let chosen = primary_target(&body);
        assert!(chosen["found"].clone().as_bool().unwrap(), "{chosen:?}");
        assert_eq!(chosen["id"].clone().into_string().unwrap(), key);

        let (output, special) = ScriptStorage::default()
            .execute("분노", &mut body, "분노충돌", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 공격할 그런 대상이 없습니다."]);
        assert_eq!(body.get_int("분노"), 135);
        assert!(special.is_none());

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&key);
        world.get_room_objs_mut(&zone, room).clear();
    }

    #[test]
    fn explicit_pvp_anger_damages_current_player_target_and_consumes_anger() {
        let suffix = std::process::id();
        let attacker_name = format!("분노비무공격-{suffix}");
        let target_name = format!("분노비무대상-{suffix}");
        let zone = format!("분노비무존-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &attacker_name,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
            world.set_player_position(&target_name, PlayerPosition::new(zone.clone(), "1".into()));
        }
        let mut target = Body::new();
        target.set("이름", target_name.as_str());
        target.set("체력", 500_i64);
        target.set("최고체력", 500_i64);
        target.set("레벨", 1_i64);
        target.act = ActState::Fight;
        crate::script::set_cast_room_players(vec![crate::script::CastRoomPlayerRef::new(
            &mut target,
        )]);

        let mut guard = crate::object::Object::new();
        guard.set("이름", "비무호위");
        guard.set("종류", "호위");
        guard.set("체력", 100_i64);
        guard.set("명중력", 10_000_i64);
        guard.set("공격력", 100_i64);
        guard.set("체력감소", 0_i64);
        guard.set("사용스크립", "[무] 출격");
        guard.set("공격스크립", "[무] [방] 공격");
        guard.set("실패스크립", "[무] 실패");
        let mut attacker = Body::new();
        attacker.set("이름", attacker_name.as_str());
        attacker.set("레벨", 1_i64);
        attacker.set("힘", 100_i64);
        attacker.set("분노", 135_i64);
        attacker.act = ActState::Fight;
        attacker.object.objs.push(Arc::new(Mutex::new(guard)));
        attacker.temp_mut().insert(
            super::super::combat_commands::PVP_TARGET.into(),
            Value::String(target_name.clone()),
        );

        let (output, special) = ScriptStorage::default()
            .execute("분노", &mut attacker, &target_name, None, None, None)
            .unwrap();
        assert!(output.len() >= 2);
        assert_eq!(attacker.get_int("분노"), 35);
        assert!(target.get_hp() < 500);
        assert!(
            special.is_none(),
            "Python builds room text but never sends it"
        );

        crate::script::clear_cast_room_players();
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&attacker_name);
        world.remove_player_position(&target_name);
    }

    #[test]
    fn implicit_anger_hits_only_the_registered_instance_of_duplicate_mobs() {
        let suffix = std::process::id();
        let player = format!("분노동종개체검사-{suffix}");
        let zone = format!("분노동종개체존-{suffix}");
        let room = "1";
        let key = format!("{zone}:동종몹");
        let mut data = RawMobData::new();
        data.name = "동종분노대상".into();
        data.level = 1;
        data.max_hp = 500;
        let selected = MobInstance::new(key.clone(), zone.clone(), room, &data);
        let selected_id = selected.instance_id;
        let untouched = MobInstance::new(key.clone(), zone.clone(), room, &data);
        let untouched_id = untouched.instance_id;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data);
            world.mob_cache.add_mob_instance(selected);
            world.mob_cache.add_mob_instance(untouched);
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
        }

        let mut guard = crate::object::Object::new();
        guard.set("이름", "동종검증호위");
        guard.set("종류", "호위");
        guard.set("체력", 100_i64);
        guard.set("명중력", 10_000_i64);
        guard.set("공격력", 100_i64);
        guard.set("체력감소", 0_i64);
        guard.set("사용스크립", "[무] 출격");
        guard.set("공격스크립", "[무] [방] 공격");
        guard.set("실패스크립", "[무] 실패");
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("레벨", 1_i64);
        body.set("힘", 100_i64);
        body.set("분노", 135_i64);
        body.act = ActState::Fight;
        body.object.objs.push(Arc::new(Mutex::new(guard)));
        body.temp_mut().insert(
            "_combat_target_instance_ids".into(),
            Value::String(selected_id.to_string()),
        );
        body.temp_mut()
            .insert("_combat_target_ids".into(), Value::String(key.clone()));

        let (output, special) = ScriptStorage::default()
            .execute("분노", &mut body, "", None, None, None)
            .unwrap();
        assert!(output.len() >= 2, "{output:?}");
        assert_eq!(body.get_int("분노"), 35);
        assert!(special.is_none());
        let world = get_world_state().read().unwrap();
        let mobs = world.mob_cache.get_all_mobs_in_room(&zone, room);
        assert!(
            mobs.iter()
                .find(|mob| mob.instance_id == selected_id)
                .unwrap()
                .hp
                < 500
        );
        assert_eq!(
            mobs.iter()
                .find(|mob| mob.instance_id == untouched_id)
                .unwrap()
                .hp,
            500
        );
        drop(world);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&key);
    }
}
