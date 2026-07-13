use super::*;
#[test]
fn force_command_queues_real_target_reentry_and_has_no_admin_success_text() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let admin = format!("명령관리자-{suffix}");
    let target = format!("명령대상-{suffix}");
    let decoy = format!("명령앞대상-{suffix}");
    let zone = format!("명령시험존-{suffix}");
    let mob_key = format!("{zone}:명령몹");
    let mut mob_data = RawMobData::new();
    mob_data.name = "명령몹".into();
    mob_data.zone = zone.clone();
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), mob_data.clone());
        world.mob_cache.add_mob_instance(MobInstance::new(
            mob_key.clone(),
            zone.clone(),
            "1",
            &mob_data,
        ));
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&decoy, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 2000_i64);
    let mut target_view = Body::new();
    target_view.set("이름", target.as_str());
    target_view.set("반응이름", "강제대상별칭");
    let mut decoy_view = Body::new();
    decoy_view.set("이름", decoy.as_str());
    decoy_view.set("반응이름", "강제대상별칭");
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![
            build_room_view_player_snapshot(&decoy_view),
            build_room_view_player_snapshot(&target_view),
        ],
    )]));
    let mut collision = Object::new();
    collision.set("이름", "강제명령충돌패");
    collision.set("반응이름", "강제대상별칭");
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let item_first = ScriptStorage::default()
        .execute("명령", &mut body, "강제대상별칭 점수", None, None, None)
        .unwrap();
    assert_eq!(item_first.0, vec!["☞ 그런 대상이 없어요. *^_^*"]);
    assert!(take_force_command_request(&mut body).is_empty());
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, "1", &collision);
        world
            .get_room_objs_mut(&zone, "1")
            .retain(|item| !Arc::ptr_eq(item, &collision));
    }
    let result = ScriptStorage::default()
        .execute("명령", &mut body, "2강제대상별칭 점수", None, None, None)
        .unwrap();
    assert!(result.0.is_empty());
    assert_eq!(
        take_force_command_request(&mut body),
        vec![(target.clone(), "점수".to_string())]
    );
    let mut box_collision = Object::new();
    box_collision.set("이름", "강제명령동적상자");
    box_collision.set("반응이름", "강제대상별칭");
    let box_collision = Arc::new(Mutex::new(box_collision));
    {
        let mut world = get_world_state().write().unwrap();
        world
            .get_room_objs_mut(&zone, "1")
            .push(box_collision.clone());
        world.record_box(&zone, "1", &box_collision);
    }
    let box_first = ScriptStorage::default()
        .execute("명령", &mut body, "강제대상별칭 점수", None, None, None)
        .unwrap();
    assert_eq!(box_first.0, vec!["☞ 그런 대상이 없어요. *^_^*"]);
    assert!(take_force_command_request(&mut body).is_empty());
    let mob_target = ScriptStorage::default()
        .execute("명령", &mut body, "명령몹 점수", None, None, None)
        .unwrap();
    assert_eq!(
        mob_target.0,
        vec!["☞ 그런 대상이 없어요. *^_^*"],
        "Python rejects non-player room objects with the generic target message"
    );
    assert!(take_force_command_request(&mut body).is_empty());
    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
    world.remove_player_position(&decoy);
    world.mob_cache.remove_mob(&mob_key);
}
#[test]
fn target_side_failed_summon_still_clears_combat_and_pvp_like_python() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("소환실패대상-{suffix}");
    let zone = format!("소환실패출발-{suffix}");
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone, "1".into()));
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.act = crate::player::ActState::Fight;
    body.temp_mut()
        .insert("_combat_target_ids".into(), Value::String("몹:대상".into()));
    body.temp_mut()
        .insert("_pvp_target".into(), Value::String("사용자토큰".into()));

    let result = ScriptStorage::default()
        .execute(
            "__summon_move",
            &mut body,
            &format!("없는소환존-{suffix} 999"),
            None,
            None,
            None,
        )
        .unwrap();
    assert!(result.0.is_empty());
    assert_eq!(body.act, crate::player::ActState::Stand);
    assert!(!body.temp().contains_key("_combat_target_ids"));
    assert!(!body.temp().contains_key("_pvp_target"));

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
}
#[test]
fn summon_command_queues_target_side_move_without_invented_admin_success_text() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("소환관리자-{suffix}");
    let target = format!("소환대상-{suffix}");
    let zone = format!("소환요청존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "2".into()));
    }
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 2000_i64);
    let result = ScriptStorage::default()
        .execute("소환", &mut body, &target, None, None, None)
        .unwrap();
    assert!(result.0.is_empty());
    assert_eq!(
        take_summon_player_request(&mut body),
        vec![(target.clone(), zone.clone(), "1".to_string())]
    );
    let world = get_world_state().read().unwrap();
    assert_eq!(
        world.get_player_position(&target).unwrap().room,
        "2",
        "admin-side script must not move the target before target-side Rhai runs"
    );
    drop(world);
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
}
#[test]
fn admin_front_variants_keep_summon_and_silent_direct_move_distinct() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("앞관리자-{suffix}");
    let target = format!("앞대상-{suffix}");
    let zone = format!("앞시험존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for (room, name) in [("1", "출발방"), ("2", "도착방")] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보":{"이름":name,"존이름":zone,"설명":[],"출구":[]}})
                .to_string(),
        )
        .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.room_cache.get_room(&zone, "2").unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "2".into()));
    }
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();
    let direct = storage
        .execute("앞앞", &mut body, &target, None, None, None)
        .unwrap();
    assert!(
        direct.0.is_empty(),
        "Python 앞앞 directly inserts with no room output"
    );
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&admin)
            .unwrap()
            .room,
        "2"
    );

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
    let summoned = storage
        .execute("앞", &mut body, &target, None, None, None)
        .unwrap();
    assert!(summoned.0[0].contains("알수 없는 기운에 휘말려 사라집니다."));
    assert!(summoned.0.iter().any(|line| line.contains("도착방")));

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
    for command in ["앞", "앞앞"] {
        let spaced = storage
            .execute(
                command,
                &mut body,
                &format!("  {target}  "),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_player_position(&admin)
                .unwrap()
                .room,
            "2",
            "Python parse_command strips the parameter for {command}: {:?}",
            spaced.0
        );
        get_world_state()
            .write()
            .unwrap()
            .set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
    }
    for command in ["앞", "앞앞"] {
        let whitespace = storage
            .execute(command, &mut body, " ", None, None, None)
            .unwrap();
        assert_eq!(
            whitespace.0,
            vec!["☞ 운영자 명령: [대상] 앞"],
            "Python parser turns whitespace-only input into an empty argument for {command}"
        );
    }

    let broken_target = format!("앞손상대상-{suffix}");
    get_world_state().write().unwrap().set_player_position(
        &broken_target,
        PlayerPosition::new(format!("없는앞존-{suffix}"), "999".into()),
    );
    body.act = crate::player::ActState::Fight;
    body.temp_mut()
        .insert("_combat_target_ids".into(), Value::String("몹:대상".into()));
    body.temp_mut()
        .insert("_pvp_target".into(), Value::String("상대토큰".into()));
    let failed = storage
        .execute("앞", &mut body, &broken_target, None, None, None)
        .unwrap();
    assert_eq!(failed.0, vec!["☞ 공간이동에 실패하였습니다."]);
    assert_eq!(body.act, crate::player::ActState::Stand);
    assert!(!body.temp().contains_key("_combat_target_ids"));
    assert!(!body.temp().contains_key("_pvp_target"));
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&admin)
            .unwrap()
            .room,
        "1"
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
    world.remove_player_position(&broken_target);
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn change_player_command_resolves_room_reaction_prefix_and_preserves_raw_input() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let actor = format!("체인지관리자-{suffix}");
    let target = format!("체인지대상-{suffix}");
    let zone = format!("체인지시험존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&actor, &target] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".to_string()));
        }
    }
    let mut target_snapshot = rhai::Map::new();
    target_snapshot.insert("이름".into(), Dynamic::from(target.clone()));
    target_snapshot.insert("반응이름".into(), Dynamic::from("교환별칭\r\n다른별칭"));
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![Dynamic::from(target_snapshot)],
    )]));

    let mut body = Body::new();
    body.set("이름", actor.as_str());
    body.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();
    assert!(storage
        .execute("체인지", &mut body, "교환별", None, None, None)
        .unwrap()
        .0
        .is_empty());
    assert_eq!(
        take_change_player_request(&mut body).as_deref(),
        Some(target.as_str())
    );
    assert!(storage
        .execute("체인지", &mut body, " ", None, None, None)
        .unwrap()
        .0
        .contains(&"☞ 사용법: [대상] 체인지".to_string()));
    assert!(storage
        .execute("체인지", &mut body, "교환별칭", None, None, None)
        .unwrap()
        .0
        .is_empty());
    assert_eq!(
        take_change_player_request(&mut body).as_deref(),
        Some(target.as_str())
    );

    let mut collision = Object::new();
    collision.set("이름", "체인지충돌물건");
    collision.set("반응이름", "교환별칭");
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let item_first = storage
        .execute("체인지", &mut body, "교환별칭", None, None, None)
        .unwrap();
    assert_eq!(item_first.0, vec!["여기에 없나봐요^^"]);
    assert!(take_change_player_request(&mut body).is_none());

    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        "1",
        crate::world::RoomObjectRef::Player(target.clone()),
    );
    let player_first = storage
        .execute("체인지", &mut body, "교환별칭", None, None, None)
        .unwrap();
    assert!(player_first.0.is_empty());
    assert_eq!(
        take_change_player_request(&mut body).as_deref(),
        Some(target.as_str())
    );

    set_precomputed_room_view_players(HashMap::new());
    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&actor);
    world.remove_player_position(&target);
}

#[test]
fn force_command_matches_python_guards_transparency_self_and_command_remainder() {
    use crate::world::{get_world_state, PlayerPosition, RoomObjectRef};

    let suffix = std::process::id();
    let admin = format!("강제관리자-{suffix}");
    let hidden = format!("강제투명-{suffix}");
    let zone = format!("강제명령존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&hidden, PlayerPosition::new(zone.clone(), "1".into()));
        world.record_test_room_object(&zone, "1", RoomObjectRef::Player(admin.clone()));
        world.record_test_room_object(&zone, "1", RoomObjectRef::Player(hidden.clone()));
    }

    let mut admin_view = Body::new();
    admin_view.set("이름", admin.as_str());
    admin_view.set("반응이름", "관리별칭");
    let mut hidden_view = Body::new();
    hidden_view.set("이름", hidden.as_str());
    hidden_view.set("반응이름", "숨은별칭");
    hidden_view.set("투명상태", 1_i64);
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![
            build_room_view_player_snapshot(&admin_view),
            build_room_view_player_snapshot(&hidden_view),
        ],
    )]));

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("반응이름", "관리별칭");
    body.set("관리자등급", 1999_i64);

    let denied = storage
        .execute("명령", &mut body, "관리별칭 점수", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert!(take_force_command_request(&mut body).is_empty());

    body.set("관리자등급", 2000_i64);
    for input in ["", "관리별칭", "  관리별칭   "] {
        let usage = storage
            .execute("명령", &mut body, input, None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 운영자 명령: [대상] [내용] 명령"]);
    }

    let invisible = storage
        .execute("명령", &mut body, "숨은별칭 점수", None, None, None)
        .unwrap();
    assert_eq!(invisible.0, vec!["☞ 그런 대상이 없어요. *^_^*"]);
    assert!(take_force_command_request(&mut body).is_empty());

    let forced_self = storage
        .execute(
            "명령",
            &mut body,
            "  관리별칭     말   내부  공백   ",
            None,
            None,
            None,
        )
        .unwrap();
    assert!(forced_self.0.is_empty());
    assert_eq!(
        take_force_command_request(&mut body),
        vec![(admin.clone(), "말   내부  공백".to_string())]
    );

    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&hidden);
}
