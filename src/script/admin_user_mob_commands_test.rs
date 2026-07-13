use super::*;
#[test]
fn user_mob_commands_create_and_remove_socketless_player_objects() {
    use crate::world::{get_world_state, PlayerPosition, RoomObjectRef};

    let suffix = std::process::id();
    let admin_name = format!("사용자몹관리자-{suffix}");
    let summoned_name = format!("사용자몹대상-{suffix}");
    let loaded_name = format!("사용자몹실제이름-{suffix}");
    let zone = format!("사용자몹시험존-{suffix}");
    let user_path = format!("data/user/{summoned_name}.json");
    let mut saved = Body::new();
    saved.set("이름", loaded_name.as_str());
    saved.set("설명1", "소환된 사용자가 서 있습니다.");
    assert!(save_body_to_json(&mut saved, &user_path));
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    let storage = ScriptStorage::default();

    admin.set("관리자등급", 999_i64);
    for command in ["사용자몹소환", "사용자몹제거", "사용자몹제거1"] {
        let denied = storage
            .execute(command, &mut admin, &summoned_name, None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    }
    admin.set("관리자등급", 1000_i64);

    let usage = storage
        .execute("사용자몹소환", &mut admin, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [대상] 사용자몹소환"]);
    let whitespace_summon = storage
        .execute("사용자몹소환", &mut admin, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_summon.0, vec!["☞ 사용법: [대상] 사용자몹소환"]);
    let whitespace_remove = storage
        .execute("사용자몹제거1", &mut admin, "   ", None, None, None)
        .unwrap();
    assert_eq!(
        whitespace_remove.0,
        vec!["사용법: [사용자 이름] 사용자몹제거"]
    );
    let missing_name = format!("없는사용자몹-{suffix}");
    let _ = std::fs::remove_file(format!("data/user/{missing_name}.json"));
    let missing = storage
        .execute("사용자몹소환", &mut admin, &missing_name, None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["존재하지않는 사용자입니다."]);
    let summoned = storage
        .execute("사용자몹소환", &mut admin, &summoned_name, None, None, None)
        .unwrap();
    assert_eq!(
        summoned.0,
        vec![format!(
            "\x1b[1m{loaded_name}\x1b[0;37m{} 소환하였습니다.",
            han_eul(&loaded_name)
        )]
    );
    {
        let world = get_world_state().read().unwrap();
        assert_eq!(
            world
                .summoned_users()
                .iter()
                .filter(|user| user.body.get_name() == loaded_name)
                .count(),
            1
        );
        assert!(matches!(
            world.get_room_object_order(&zone, "1").first(),
            Some(RoomObjectRef::SummonedUser(_))
        ));
    }
    let absent = storage
        .execute("사용자몹제거1", &mut admin, "없는사용자", None, None, None)
        .unwrap();
    assert!(
        absent.0.is_empty(),
        "Python silently returns when no name matches"
    );
    let mut collision = Object::new();
    collision.set("이름", "사용자몹충돌물건");
    collision.set("반응이름", loaded_name.as_str());
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let item_first = storage
        .execute("사용자몹제거", &mut admin, &loaded_name, None, None, None)
        .unwrap();
    assert_eq!(item_first.0, vec!["그런 몹이 없어요!"]);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .summoned_users()
            .iter()
            .filter(|user| user.body.get_name() == loaded_name)
            .count(),
        1
    );
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, "1", &collision);
        world
            .get_room_objs_mut(&zone, "1")
            .retain(|item| !Arc::ptr_eq(item, &collision));
    }
    let removed_in_room = storage
        .execute("사용자몹제거", &mut admin, &loaded_name, None, None, None)
        .unwrap();
    assert_eq!(removed_in_room.0, vec!["사용자몹이 제거되었습니다."]);
    assert!(!get_world_state()
        .read()
        .unwrap()
        .summoned_users()
        .iter()
        .any(|user| user.body.get_name() == loaded_name));

    storage
        .execute("사용자몹소환", &mut admin, &summoned_name, None, None, None)
        .unwrap();
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "2".into()));
    let wrong_room = storage
        .execute("사용자몹제거", &mut admin, &loaded_name, None, None, None)
        .unwrap();
    assert_eq!(wrong_room.0, vec!["그런 몹이 없어요!"]);
    let removed = storage
        .execute("사용자몹제거1", &mut admin, &loaded_name, None, None, None)
        .unwrap();
    assert_eq!(removed.0, vec!["사용자몹이 제거되었습니다."]);
    assert!(!get_world_state()
        .read()
        .unwrap()
        .summoned_users()
        .iter()
        .any(|user| user.body.get_name() == loaded_name));

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&admin_name);
    let _ = std::fs::remove_file(user_path);
}

#[test]
fn summon_user_mob_restores_saved_body_and_allows_python_duplicate_players() {
    use crate::world::{get_world_state, PlayerPosition, RoomObjectRef};

    let suffix = std::process::id();
    let admin_name = format!("사용자몹중복관리자-{suffix}");
    let file_name = format!("사용자몹중복파일-{suffix}");
    let loaded_name = format!("사용자몹중복실제-{suffix}");
    let zone = format!("사용자몹중복존-{suffix}");
    let user_path = format!("data/user/{file_name}.json");

    let mut saved = Body::new();
    saved.set("이름", loaded_name.as_str());
    saved.set("체력", 321_i64);
    saved.set("힘", 77_i64);
    saved.set("반응이름", "중복별칭\r\n둘째별칭");
    assert!(save_body_to_json(&mut saved, &user_path));

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 1000_i64);
    let storage = ScriptStorage::default();

    for expected_count in 1..=2 {
        let result = storage
            .execute("사용자몹소환", &mut admin, &file_name, None, None, None)
            .unwrap();
        assert_eq!(
            result.0,
            vec![format!(
                "\x1b[1m{loaded_name}\x1b[0;37m{} 소환하였습니다.",
                han_eul(&loaded_name)
            )]
        );
        let world = get_world_state().read().unwrap();
        assert_eq!(
            world
                .summoned_users()
                .iter()
                .filter(|user| user.body.object.getString("이름") == loaded_name)
                .count(),
            expected_count
        );
        let latest = world
            .summoned_users()
            .iter()
            .rev()
            .find(|user| user.body.object.getString("이름") == loaded_name)
            .unwrap();
        assert_eq!(latest.body.object.getInt("체력"), 321);
        assert_eq!(latest.body.object.getInt("힘"), 77);
        assert_eq!(
            latest.body.object.getString("반응이름"),
            "중복별칭\r\n둘째별칭"
        );
        assert_eq!(latest.position.zone, zone);
        assert_eq!(latest.position.room, "1");
    }
    {
        let world = get_world_state().read().unwrap();
        let order = world.get_room_object_order(&zone, "1");
        assert!(matches!(
            order.first(),
            Some(RoomObjectRef::SummonedUser(_))
        ));
        assert!(matches!(order.get(1), Some(RoomObjectRef::SummonedUser(_))));
    }

    // Python 사용자몹제거1 scans channel.players in insertion order, so duplicate
    // names are removed one at a time and a missing name is silent.
    for expected_count in [1, 0] {
        let result = storage
            .execute("사용자몹제거1", &mut admin, &loaded_name, None, None, None)
            .unwrap();
        assert_eq!(result.0, vec!["사용자몹이 제거되었습니다."]);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .summoned_users()
                .iter()
                .filter(|user| user.body.object.getString("이름") == loaded_name)
                .count(),
            expected_count
        );
    }
    let missing = storage
        .execute("사용자몹제거1", &mut admin, &loaded_name, None, None, None)
        .unwrap();
    assert!(missing.0.is_empty());

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&admin_name);
    let _ = std::fs::remove_file(user_path);
}

#[test]
fn admin_summon_finds_socketless_user_and_queues_target_side_rhai_move() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin_name = format!("소환체이동관리자-{suffix}");
    let target_name = format!("소환체이동대상-{suffix}");
    let zone = format!("소환체이동존-{suffix}");
    let mut target = Body::new();
    target.set("이름", target_name.as_str());
    let id = {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.add_summoned_user(target, PlayerPosition::new(zone.clone(), "2".into()))
    };
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 2000_i64);
    let result = ScriptStorage::default()
        .execute("소환", &mut admin, &target_name, None, None, None)
        .unwrap();
    assert!(result.0.is_empty());
    assert_eq!(
        take_summon_player_request(&mut admin),
        vec![(target_name.clone(), zone.clone(), "1".to_string())]
    );
    let mut world = get_world_state().write().unwrap();
    let extracted = world.take_summoned_user_by_name(&target_name).unwrap();
    assert_eq!(extracted.id, id);
    assert_eq!(extracted.position.room, "2");
    world.restore_summoned_user(extracted, PlayerPosition::new(zone, "1".into()));
    assert_eq!(
        world
            .summoned_users()
            .iter()
            .find(|user| user.id == id)
            .unwrap()
            .position
            .room,
        "1"
    );
    world.remove_summoned_user_by_id(id);
    world.remove_player_position(&admin_name);
}

#[test]
fn admin_front_moves_to_socketless_users_room_like_python_channel_player() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin_name = format!("앞소환관리자-{suffix}");
    let target_name = format!("앞소환대상-{suffix}");
    let zone = format!("앞소환존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for room in ["1", "2"] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보":{"이름":format!("앞소환방{room}"),"존이름":zone,"설명":[],"출구":[]}}).to_string(),
        )
        .unwrap();
    }
    let mut target = Body::new();
    target.set("이름", target_name.as_str());
    let id = {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.room_cache.get_room(&zone, "2").unwrap();
        world.set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.add_summoned_user(target, PlayerPosition::new(zone.clone(), "2".into()))
    };
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 1000_i64);
    let output = ScriptStorage::default()
        .execute("앞", &mut admin, &target_name, None, None, None)
        .unwrap();
    assert!(!output.0.iter().any(|line| line.contains("활동중인")));
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&admin_name)
            .unwrap()
            .room,
        "2"
    );
    let mut world = get_world_state().write().unwrap();
    world.remove_summoned_user_by_id(id);
    world.remove_player_position(&admin_name);
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}
