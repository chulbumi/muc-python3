use super::*;

#[test]
fn event_command_finds_same_room_player_and_iterates_string_characters() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("이벤트관리자-{suffix}");
    let target = format!("이벤트대상-{suffix}");
    let zone = format!("이벤트시험존-{suffix}");
    let room = "1";
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), room.into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), room.into()));
    }
    let mut player = rhai::Map::new();
    player.insert("이름".into(), Dynamic::from(target.clone()));
    player.insert("반응이름".into(), Dynamic::from("별칭"));
    player.insert("이벤트설정리스트".into(), Dynamic::from("가\n나"));
    let mut rooms = HashMap::new();
    rooms.insert(format!("{zone}:{room}"), vec![Dynamic::from(player)]);
    set_precomputed_room_view_players(rooms);

    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1000_i64);
    let storage = ScriptStorage::default();
    let result = storage
        .execute("이벤트", &mut body, "별칭", None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["가", "\n", "나"]);

    let whitespace = storage
        .execute("이벤트", &mut body, " ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법: [대상] 이벤트"]);

    let mut object = Object::new();
    object.set("이름", "사건석");
    object.set("이벤트설정리스트", "갑을");
    let object = Arc::new(Mutex::new(object));
    get_world_state()
        .write()
        .unwrap()
        .get_room_objs_mut(&zone, room)
        .push(object.clone());
    let object_result = storage
        .execute("이벤트", &mut body, " 사건석 ", None, None, None)
        .unwrap();
    assert_eq!(object_result.0, vec!["갑", "을"]);

    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
    world.get_room_objs_mut(&zone, room).clear();
}

#[test]
fn event_admin_commands_match_python_permission_usage_and_missing_text() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 999_i64);
    for command in ["이벤트", "이벤트설정", "이벤트삭제"] {
        let denied = storage
            .execute(command, &mut body, "대상 사건", None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    }
    body.set("관리자등급", 1000_i64);
    for (command, input, expected) in [
        ("이벤트", "", "☞ 사용법: [대상] 이벤트"),
        ("이벤트설정", "대상", "☞ 사용법: [대상] [이벤트] 이벤트설정"),
        ("이벤트삭제", "대상", "☞ 사용법: [대상] [이벤트] 이벤트삭제"),
    ] {
        let usage = storage
            .execute(command, &mut body, input, None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec![expected]);
    }
    for command in ["이벤트", "이벤트설정", "이벤트삭제"] {
        let input = if command == "이벤트" {
            "없는대상"
        } else {
            "없는대상 사건"
        };
        let missing = storage
            .execute(command, &mut body, input, None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["☞ 그런 대상이 없어요!"]);
    }
}

#[test]
fn event_view_uses_room_object_order_and_never_searches_inventory() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("이벤트조회자-{suffix}");
    let target = format!("이벤트조회대상-{suffix}");
    let zone = format!("이벤트조회순서존-{suffix}");
    let room = "1";
    let mut player = rhai::Map::new();
    player.insert("이름".into(), Dynamic::from(target.clone()));
    player.insert("반응이름".into(), Dynamic::from("공통"));
    player.insert("이벤트설정리스트".into(), Dynamic::from("사람"));
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:{room}"),
        vec![Dynamic::from(player)],
    )]));

    let mut floor = Object::new();
    floor.set("이름", "순서아이템");
    floor.set("반응이름", "공통");
    floor.set("이벤트설정리스트", "물건");
    let floor = Arc::new(Mutex::new(floor));
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), room.into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), room.into()));
        world.get_room_objs_mut(&zone, room).push(floor.clone());
        // Room.insert places the latest object at the front. The floor
        // item must therefore beat the matching player for `공통`.
        world.record_floor_item(&zone, room, &floor);
    }
    let mut carried = Object::new();
    carried.set("이름", "가방사건물");
    carried.set("이벤트설정리스트", "가방");
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1000_i64);
    body.object.objs.push(Arc::new(Mutex::new(carried)));
    let storage = ScriptStorage::default();

    let ordered = storage
        .execute("이벤트", &mut body, "공통", None, None, None)
        .unwrap();
    assert_eq!(ordered.0, vec!["물", "건"]);
    let numbered_player = storage
        .execute("이벤트", &mut body, "2공통", None, None, None)
        .unwrap();
    assert_eq!(numbered_player.0, vec!["사", "람"]);
    let inventory = storage
        .execute("이벤트", &mut body, "가방사건물", None, None, None)
        .unwrap();
    assert_eq!(inventory.0, vec!["☞ 그런 대상이 없어요!"]);

    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
    world.get_room_objs_mut(&zone, room).clear();
}

#[test]
fn event_set_delete_preserve_full_remainder_and_mutate_live_world_cache() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let admin = format!("이벤트수정관리자-{suffix}");
    let zone = format!("이벤트수정존-{suffix}");
    let room = "1";
    let mob_key = format!("{zone}:시험몹");
    let mut data = RawMobData::new();
    data.name = "이벤트시험몹".into();
    data.reaction_names = vec!["시험몹".into()];
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), data.clone());
        world.mob_cache.add_mob_instance(MobInstance::new(
            mob_key.clone(),
            zone.clone(),
            room,
            &data,
        ));
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), room.into()));
    }
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1000_i64);
    let storage = ScriptStorage::default();
    let event = "$대화 $대 물건 ";

    let set = storage
        .execute(
            "이벤트설정",
            &mut body,
            &format!("  시험몹   {event}"),
            None,
            None,
            None,
        )
        .unwrap();
    let normalized_event = event.trim();
    assert_eq!(
        set.0,
        vec![format!("☞ [{normalized_event}] 이벤트가 설정되었습니다.")]
    );
    assert!(get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .check_mob_event(&mob_key, normalized_event));

    let duplicate = storage
        .execute(
            "이벤트설정",
            &mut body,
            &format!("시험몹 {event}"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(duplicate.0, vec!["☞ 이미 설정되어 있습니다."]);

    let deleted = storage
        .execute(
            "이벤트삭제",
            &mut body,
            &format!("시험몹 {event}"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        deleted.0,
        vec![format!("☞ [{normalized_event}] 이벤트가 삭제되었습니다.")]
    );
    assert!(!get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .check_mob_event(&mob_key, normalized_event));

    let mut event_item = Object::new();
    event_item.set("이름", "이벤트설정석");
    event_item.set("반응이름", "설정석별칭");
    let event_item = Arc::new(Mutex::new(event_item));
    {
        let mut world = get_world_state().write().unwrap();
        world
            .get_room_objs_mut(&zone, room)
            .push(event_item.clone());
        world.record_floor_item(&zone, room, &event_item);
    }
    let item_set = storage
        .execute(
            "이벤트설정",
            &mut body,
            "설정석 아이템이벤트",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        item_set.0,
        vec!["☞ [아이템이벤트] 이벤트가 설정되었습니다."]
    );
    assert_eq!(
        event_item.lock().unwrap().getString("이벤트설정리스트"),
        "아이템이벤트"
    );
    let item_deleted = storage
        .execute(
            "이벤트삭제",
            &mut body,
            "설정석 아이템이벤트",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        item_deleted.0,
        vec!["☞ [아이템이벤트] 이벤트가 삭제되었습니다."]
    );
    assert_eq!(event_item.lock().unwrap().getString("이벤트설정리스트"), "");

    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, room).clear();
    world.mob_cache.remove_instance(&zone, room, &mob_key);
    world.mob_cache.remove_mob_definition(&mob_key);
    world.remove_player_position(&admin);
}

#[test]
fn event_delete_supports_self_and_same_room_player_with_python_attr_semantics() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("이벤트삭제자-{suffix}");
    let target = format!("이벤트삭제대상-{suffix}");
    let zone = format!("이벤트삭제존-{suffix}");
    let room = "1";
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), room.to_string()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), room.to_string()));
    }
    let mut admin_snapshot = rhai::Map::new();
    admin_snapshot.insert("이름".into(), Dynamic::from(admin.clone()));
    admin_snapshot.insert("반응이름".into(), Dynamic::from("관리자별칭"));
    admin_snapshot.insert("이벤트설정리스트".into(), Dynamic::from("자기이벤트\n남김"));
    let mut target_snapshot = rhai::Map::new();
    target_snapshot.insert("이름".into(), Dynamic::from(target.clone()));
    target_snapshot.insert("반응이름".into(), Dynamic::from("삭제별칭"));
    target_snapshot.insert(
        "이벤트설정리스트".into(),
        Dynamic::from("원격이벤트\n부분문자열"),
    );
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:{room}"),
        vec![
            Dynamic::from(admin_snapshot),
            Dynamic::from(target_snapshot),
        ],
    )]));

    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1000_i64);
    body.set("이벤트설정리스트", "자기이벤트\n남김");
    let storage = ScriptStorage::default();

    let own = storage
        .execute(
            "이벤트삭제",
            &mut body,
            &format!("{admin} 자기이벤트"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(own.0, vec!["☞ [자기이벤트] 이벤트가 삭제되었습니다."]);
    assert_eq!(body.get_string("이벤트설정리스트"), "남김");

    let remote = storage
        .execute(
            "이벤트삭제",
            &mut body,
            "삭제별칭 원격이벤트",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(remote.0, vec!["☞ [원격이벤트] 이벤트가 삭제되었습니다."]);
    assert_eq!(
        take_admin_set_player_value_request(&mut body),
        Some((
            target.clone(),
            "이벤트설정리스트".into(),
            serde_json::Value::String("부분문자열".into()),
        ))
    );

    // Python checkAttr accepts a substring, then delAttr fails to remove
    // a non-exact element but the command still reports success.
    let partial = storage
        .execute("이벤트삭제", &mut body, "삭제별칭 부분", None, None, None)
        .unwrap();
    assert_eq!(partial.0, vec!["☞ [부분] 이벤트가 삭제되었습니다."]);
    assert_eq!(
        take_admin_set_player_value_request(&mut body).unwrap().2,
        serde_json::Value::String("원격이벤트\n부분문자열".into())
    );

    let unset = storage
        .execute("이벤트삭제", &mut body, "삭제별칭 없음", None, None, None)
        .unwrap();
    assert_eq!(unset.0, vec!["☞ [없음] 이벤트는 설정되어있지 않습니다."]);
    let missing = storage
        .execute("이벤트삭제", &mut body, "없는대상 사건", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 대상이 없어요!"]);

    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
}

#[test]
fn event_set_supports_self_and_same_room_player_with_python_duplicate_check() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("이벤트설정자-{suffix}");
    let target = format!("이벤트설정대상-{suffix}");
    let zone = format!("이벤트설정플레이어존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let snapshot = |name: &str, reactions: &str, events: &str| {
        let mut player = rhai::Map::new();
        player.insert("이름".into(), Dynamic::from(name.to_string()));
        player.insert("반응이름".into(), Dynamic::from(reactions.to_string()));
        player.insert("이벤트설정리스트".into(), Dynamic::from(events.to_string()));
        Dynamic::from(player)
    };
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![
            snapshot(&admin, "", "기존"),
            snapshot(&target, "설정별칭", "긴이벤트이름"),
        ],
    )]));
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1000_i64);
    body.set("이벤트설정리스트", "기존");
    let storage = ScriptStorage::default();

    let own = storage
        .execute(
            "이벤트설정",
            &mut body,
            &format!("{admin} 새이벤트"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(own.0, vec!["☞ [새이벤트] 이벤트가 설정되었습니다."]);
    assert_eq!(body.get_string("이벤트설정리스트"), "기존\n새이벤트");

    // checkAttr is substring membership in the Python object base.
    let duplicate = storage
        .execute("이벤트설정", &mut body, "설정별칭 이벤트", None, None, None)
        .unwrap();
    assert_eq!(duplicate.0, vec!["☞ 이미 설정되어 있습니다."]);
    assert!(take_admin_set_player_value_request(&mut body).is_none());

    let remote = storage
        .execute("이벤트설정", &mut body, "설정별칭 추가", None, None, None)
        .unwrap();
    assert_eq!(remote.0, vec!["☞ [추가] 이벤트가 설정되었습니다."]);
    assert_eq!(
        take_admin_set_player_value_request(&mut body),
        Some((
            target.clone(),
            "이벤트설정리스트".into(),
            serde_json::Value::String("긴이벤트이름\n추가".into()),
        ))
    );

    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
}
