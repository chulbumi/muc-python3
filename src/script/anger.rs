//! State/data efuns for Python `cmds/분노.py`.
//! Visible combat scripts and ANSI are rendered by Rhai.

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
    for (index, mob) in mobs.iter().enumerate().rev() {
        if !mob.alive || !instance_ids.contains(&mob.instance_id) {
            continue;
        }
        result.insert("found".into(), Dynamic::from(true));
        result.insert("id".into(), Dynamic::from(mob.mob_key.clone()));
        result.insert("name".into(), Dynamic::from(mob.name.clone()));
        result.insert("room_index".into(), Dynamic::from(index as i64));
        result.insert("current".into(), Dynamic::from(true));
        break;
    }
    result
}

fn run_anger(body: &mut Body, target_id: &str, room_index: i64) -> Map {
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
    let Ok(index) = usize::try_from(room_index) else {
        return result;
    };
    let Some(mob) = mobs.get_mut(index) else {
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
            if mob.hp < 0 {
                mob.hp = 1;
            }
            event.insert("hit".into(), Dynamic::from(true));
            event.insert("damage".into(), Dynamic::from(damage));
        }
        events.push(Dynamic::from(event));
        if mob.hp <= 1 {
            break;
        }
    }
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
        move |_ob: &mut Map, id: &str, index: i64| run_anger(unsafe { &mut *body_ptr }, id, index),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptStorage;

    #[test]
    fn rhai_anger_rejects_noncombat_with_python_text() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        let (output, special) = storage
            .execute("분노", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(
            output,
            vec!["☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네"]
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
}
