//! State efuns for Python combat-administration commands.
//! Rhai retains all user-visible output.

use super::current_body_position;
use crate::player::Body;
use crate::world::get_world_state;
use rhai::{Array, Dynamic, Engine, Map};
use std::collections::HashMap;

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
    } else if query == "시체" {
        if let Some(index) = mobs.iter().position(|mob| mob.act == 2) {
            selected.push(index);
        } else {
            result.insert("status".into(), Dynamic::from("unknown"));
            return result;
        }
    } else {
        // Room.findObjName hides ACT_DEATH/ACT_REGEN unless queried as 시체.
        let live_match = mobs.iter().any(|mob| {
            mob.act != 2
                && mob.act != 3
                && metadata.get(&mob.mob_key).is_some_and(|data| {
                    data.name == query || data.reaction_names.iter().any(|name| name == query)
                })
        });
        result.insert(
            "status".into(),
            Dynamic::from(if live_match { "not_corpse" } else { "unknown" }),
        );
        return result;
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
    if mob.mob_key != target_id || !mob.alive {
        return result;
    }
    let name = mob.name.clone();
    mob.kill();
    mob.targets.clear();
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
    let ordered = world.get_room_object_order(&zone, &room);
    let metadata = world
        .mob_cache
        .ordered_mob_templates()
        .map(|(key, data)| (key.to_string(), data.clone()))
        .collect::<HashMap<_, _>>();
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
        return false;
    };
    let first = query.split_whitespace().next().unwrap_or("");
    if first.is_empty() {
        return false;
    }
    let numeric = first.parse::<usize>().ok().filter(|number| *number != 0);
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
        let Some(data) = metadata.get(&mob.mob_key) else { continue };
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
                count == 1
            } else {
                false
            }
        } else {
            if mob.act == 2 || mob.act == 3 {
                false
            } else if data.name == first || data.reaction_names.iter().any(|name| name == first) {
                count += 1;
                count == 1
            } else if data
                .reaction_names
                .iter()
                .any(|alias| alias.starts_with(first))
            {
                prefix_count += 1;
                prefix_count == 1
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
    if one_item && !crate::oneitem::oneitem_get(&index).is_empty() {
        result.insert("status".into(), Dynamic::from("one_exists"));
        result.insert("name".into(), Dynamic::from(display_name));
        result.insert("particle_source".into(), Dynamic::from(particle_source));
        return result;
    }
    if one_item {
        crate::oneitem::oneitem_have(&index, &body.get_name());
    }
    for _ in 0..count.max(0) {
        body.object
            .objs
            .push(std::sync::Arc::new(std::sync::Mutex::new(
                template_guard.deepclone(),
            )));
    }
    result.insert("status".into(), Dynamic::from("ok"));
    result.insert("name".into(), Dynamic::from(display_name));
    result
}

fn body_status(body: &Body) -> Map {
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

fn clear_summon_combat(body: &mut Body) {
    let name = body.get_name();
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
    engine.register_fn("recover_named_room_mob", move |_ob: &mut Map, query: &str| {
        recover_named_mob(unsafe { &*ptr }, query)
    });
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
    }

    #[test]
    fn self_status_rhai_uses_full_python_table_instead_of_summary() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "상태본인");
        body.set("관리자등급", 1000_i64);
        body.set("레벨", 10_i64);
        body.set("체력", 321_i64);
        body.set("최고체력", 450_i64);
        body.set("내공", 123_i64);
        body.set("최고내공", 234_i64);
        let (output, _) = storage
            .execute("상태보기", &mut body, "상태본인", None, None, None)
            .unwrap();
        assert!(output.iter().any(|line| line.contains("[命  中]")));
        assert!(output.iter().any(|line| line.contains("[은  전]")));
        assert!(output
            .iter()
            .any(|line| line.contains("표국보험은 효력이 없습니다.")));
        assert!(!output.iter().any(|line| line.starts_with("◆")));
    }

    #[test]
    fn room_player_status_uses_the_same_full_renderer() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "관리자");
        body.set("관리자등급", 1000_i64);
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
        let (output, _) = storage
            .execute("상태보기", &mut body, "대상자", None, None, None)
            .unwrap();
        assert!(output
            .iter()
            .any(|line| line.contains("대상자의 현재 상태")));
        assert!(output.iter().any(|line| line.contains("[命  中]")));
        assert!(output.iter().any(|line| line.contains("2개의 여유 특성치")));
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
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
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
}
