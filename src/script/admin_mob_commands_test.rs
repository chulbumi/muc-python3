use super::*;
#[test]
fn admin_rank_and_mob_commands_normalize_whitespace_like_python_dispatch() {
    let rank_file_before = std::fs::read("data/config/rank.json").ok();
    let suffix = std::process::id();
    let rank_type = format!("공백순위-{suffix}");
    let rank_name = format!("공백순위인-{suffix}");
    rank_write(&rank_type, &rank_name, 10, 1);
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 1999_i64);
    let denied = storage
        .execute("순위초기화", &mut body, &rank_type, None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert_eq!(rank_read(&rank_type, &rank_name), 1);
    body.set("관리자등급", 2000_i64);
    let missing = storage
        .execute(
            "순위초기화",
            &mut body,
            &format!("없는순위-{suffix}"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 순위는 없습니다."]);
    assert_eq!(rank_read(&rank_type, &rank_name), 1);

    let rank = storage
        .execute("순위초기화", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(rank.0, vec!["* 전체가 초기화 되었습니다."]);
    assert_eq!(rank_read(&rank_type, &rank_name), 0);
    let spawn = storage
        .execute("몹생성", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(spawn.0, vec!["사용법: [몹 이름] 생성"]);
    let remove = storage
        .execute("몹제거", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(remove.0, vec!["사용법: [몹 이름] 몹제거"]);
    rank_clear(&rank_type);
    if let Some(contents) = rank_file_before {
        std::fs::write("data/config/rank.json", contents).unwrap();
    } else {
        let _ = std::fs::remove_file("data/config/rank.json");
    }
}

#[test]
fn mob_editor_uses_python_argument_permission_and_input_state() {
    use crate::command::handler::{CommandResult, PendingInput};

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 0_i64);
    for input in ["", "   ", "존만"] {
        let usage = storage
            .execute("몹제작", &mut body, input, None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [존이름] [몹이름] 몹제작"]);
    }
    let denied = storage
        .execute("몹제작", &mut body, "시험존 시험몹", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^"]);

    body.set("관리자등급", 1000_i64);
    let editor = storage
        .execute(
            "몹제작",
            &mut body,
            "  시험존\t시험몹   뒤의값은무시 ",
            None,
            None,
            None,
        )
        .unwrap();
    assert!(
        editor.0.is_empty(),
        "Python starts with write(), not sendLine()"
    );
    assert!(matches!(
        editor.1,
        Some(CommandResult::RequestInput {
            ref prompt,
            state: PendingInput::FileEdit { ref relative_path, ref lines }
        }) if prompt == "작성을 마치시려면 '.' 를 입력하세요.\r\n:"
            && relative_path == "mob/시험존/시험몹.json"
            && lines.is_empty()
    ));
}

#[test]
fn mob_find_uses_python_zone_then_template_order_and_search_modes() {
    use crate::world::{get_world_state, RawMobData};

    let suffix = std::process::id();
    let zone_a = format!("몹찾기앞존-{suffix}");
    let zone_b = format!("몹찾기뒤존-{suffix}");
    let keys = [
        format!("{zone_a}:첫몹"),
        format!("{zone_b}:중간몹"),
        format!("{zone_a}:나중몹"),
    ];
    let make = |name: &str, location: serde_json::Value| {
        let mut data = RawMobData::new();
        data.name = name.into();
        data.mob_type = 6;
        data.attributes.insert("위치".into(), location);
        data
    };
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(
            keys[0].clone(),
            make("순서검색첫몹", serde_json::json!(["11", "20-23"])),
        );
        world.mob_cache.insert_mob_data(
            keys[1].clone(),
            make("순서검색중간몹", serde_json::json!("이름방")),
        );
        world.mob_cache.insert_mob_data(
            keys[2].clone(),
            make("순서검색나중몹", serde_json::json!([])),
        );
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 999_i64);
    assert_eq!(
        storage
            .execute("몹찾기", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );
    body.set("관리자등급", 1000_i64);
    assert_eq!(
        storage
            .execute("몹찾기", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 운영자 명령: [몹이름]|[몹종류] 몹찾기\r\n ex) 산적 몹찾기 or 6 몹찾기"]
    );
    let by_type = storage
        .execute("몹찾기", &mut body, "6종", None, None, None)
        .unwrap();
    let selected = by_type
        .0
        .into_iter()
        .filter(|line| line.contains("순서검색"))
        .collect::<Vec<_>>();
    assert_eq!(
        selected,
        vec![
            "\x1b[33m순서검색첫몹\x1b[37m(첫몹) : ['11', '20-23']",
            "\x1b[33m순서검색나중몹\x1b[37m(나중몹) : []",
            "\x1b[33m순서검색중간몹\x1b[37m(중간몹) : '이름방'",
        ]
    );
    assert_eq!(
        storage
            .execute("몹찾기", &mut body, "검색중간", None, None, None)
            .unwrap()
            .0,
        vec!["\x1b[33m순서검색중간몹\x1b[37m(중간몹) : '이름방'"]
    );
    assert_eq!(
        storage
            .execute("몹찾기", &mut body, "절대없는검색어", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 찾으시는 몹이 없습니다."]
    );
    let mut world = get_world_state().write().unwrap();
    for key in keys {
        world.mob_cache.remove_mob(&key);
    }
}

#[test]
fn mob_recover_checks_python_permission_and_usage_before_room_lookup() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 999_i64);
    assert_eq!(
        storage
            .execute("몹회복", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );
    body.set("관리자등급", 1000_i64);
    assert_eq!(
        storage
            .execute("몹회복", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 운영자 명령: [대상] 몹회복"]
    );
    assert_eq!(
        storage
            .execute("몹회복", &mut body, "없는몹", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );
}

#[test]
fn mob_delete_removes_python_template_but_keeps_existing_room_clone() {
    use crate::world::{get_world_state, MobInstance, RawMobData};

    let suffix = std::process::id();
    let zone = format!("몹삭제존-{suffix}");
    let key = format!("{zone}:삭제시험몹");
    let directory = std::path::Path::new("data/mob").join(&zone);
    let path = directory.join("삭제시험몹.json");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        &path,
        serde_json::json!({"몹정보": {
            "이름": "삭제시험몹", "존이름": zone, "위치": []
        }})
        .to_string(),
    )
    .unwrap();
    let mut data = RawMobData::new();
    data.name = "삭제시험몹".into();
    data.zone = zone.clone();
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        world
            .mob_cache
            .add_mob_instance(MobInstance::new(key.clone(), zone.clone(), "1", &data));
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 1999_i64);
    let denied = storage
        .execute("몹삭제", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 2000_i64);
    for input in ["", " \t "] {
        let usage = storage
            .execute("몹삭제", &mut body, input, None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["사용법: [몹 인덱스] 몹삭제"]);
    }
    for malformed in ["콜론없음", ":", ":몹", "존:"] {
        let missing = storage
            .execute("몹삭제", &mut body, malformed, None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["존재하지않는 몹입니다."], "{malformed}");
    }

    let deleted = storage
        .execute("몹삭제", &mut body, &format!("  {key}  "), None, None, None)
        .unwrap();
    assert_eq!(deleted.0, vec!["몹이 삭제되었습니다."]);
    {
        let world = get_world_state().read().unwrap();
        assert!(world.mob_cache.get_mob(&key).is_none());
        assert_eq!(
            world.mob_cache.get_all_mobs_in_room(&zone, "1").len(),
            1,
            "Python room clone survives deletion from Mob.Mobs"
        );
    }
    let missing = storage
        .execute("몹삭제", &mut body, &key, None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["존재하지않는 몹입니다."]);

    let recreated = storage
        .execute("몹생성", &mut body, &key, None, None, None)
        .unwrap();
    assert_eq!(
        recreated.0,
        vec!["\x1b[1;32m* [삭제시험몹] 생성 되었습니다.\x1b[0;37m"]
    );
    let deleted_after_reload = storage
        .execute("몹삭제", &mut body, &key, None, None, None)
        .unwrap();
    assert_eq!(deleted_after_reload.0, vec!["몹이 삭제되었습니다."]);
    assert!(path.exists(), "Python never deletes the source mob JSON");

    get_world_state()
        .write()
        .unwrap()
        .mob_cache
        .remove_mob(&key);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn mob_create_reloads_json_and_preserves_string_and_named_room_locations() {
    use crate::world::get_world_state;

    let suffix = std::process::id();
    let zone = format!("몹위치존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    let mob_dir = std::path::Path::new("data/mob").join(&zone);
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::create_dir_all(&mob_dir).unwrap();
    for room in ["1", "2", "이름방"] {
        std::fs::write(
            room_dir.join(format!("{room}.json")),
            serde_json::json!({"맵정보": {
                "이름": room, "존이름": zone, "설명": [], "출구": [], "몹": []
            }})
            .to_string(),
        )
        .unwrap();
    }
    std::fs::write(
        mob_dir.join("문자위치.json"),
        serde_json::json!({"몹정보": {
            "이름": "문자위치몹", "존이름": zone, "위치": "12",
            "체력": 50, "내공": 7
        }})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        mob_dir.join("이름위치.json"),
        serde_json::json!({"몹정보": {
            "이름": "이름위치몹", "존이름": zone,
            "위치": ["이름방", "없는방"], "체력": 80
        }})
        .to_string(),
    )
    .unwrap();

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 2000_i64);
    let scalar_key = format!("{zone}:문자위치");
    let scalar = storage
        .execute("몹생성", &mut body, &scalar_key, None, None, None)
        .unwrap();
    assert_eq!(
        scalar.0,
        vec!["\x1b[1;32m* [문자위치몹] 생성 되었습니다.\x1b[0;37m"]
    );
    let named_key = format!("{zone}:이름위치");
    let named = storage
        .execute("몹생성", &mut body, &named_key, None, None, None)
        .unwrap();
    assert_eq!(
        named.0,
        vec!["\x1b[1;32m* [이름위치몹] 생성 되었습니다.\x1b[0;37m"]
    );

    {
        let world = get_world_state().read().unwrap();
        assert_eq!(
            world.mob_cache.get_mob(&scalar_key).unwrap().locations,
            ["1", "2"]
        );
        assert_eq!(world.mob_cache.get_all_mobs_in_room(&zone, "1").len(), 1);
        assert_eq!(world.mob_cache.get_all_mobs_in_room(&zone, "2").len(), 1);
        assert_eq!(
            world.mob_cache.get_mob(&named_key).unwrap().locations,
            ["이름방", "없는방"]
        );
        assert_eq!(
            world.mob_cache.get_all_mobs_in_room(&zone, "이름방").len(),
            1
        );
        assert!(world
            .mob_cache
            .get_all_mobs_in_room(&zone, "없는방")
            .is_empty());
    }

    let mut world = get_world_state().write().unwrap();
    world.mob_cache.remove_mob(&scalar_key);
    world.mob_cache.remove_mob(&named_key);
    drop(world);
    let _ = std::fs::remove_dir_all(room_dir);
    let _ = std::fs::remove_dir_all(mob_dir);
}

#[test]
fn mob_remove_matches_python_selection_and_clears_room_object_order() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player = format!("몹제거관리자-{suffix}");
    let zone = format!("몹제거존-{suffix}");
    let room = "1";
    let key = format!("{zone}:시험몹");
    let mut data = RawMobData::new();
    data.name = "시험몹".into();
    data.reaction_names = vec!["시험대상".into()];
    let first = MobInstance::new(key.clone(), zone.clone(), room, &data);
    let first_id = first.instance_id;
    let second = MobInstance::new(key.clone(), zone.clone(), room, &data);
    let second_id = second.instance_id;
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
        world.mob_cache.insert_mob_data(key.clone(), data);
        world.mob_cache.add_mob_instance(first);
        world.mob_cache.add_mob_instance(second);
        world.record_test_room_object(&zone, room, RoomObjectRef::Mob(first_id));
        world.record_test_room_object(&zone, room, RoomObjectRef::Mob(second_id));
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 1999_i64);
    assert_eq!(
        storage
            .execute("몹제거", &mut body, "시험몹", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );
    body.set("관리자등급", 2000_i64);
    assert_eq!(
        storage
            .execute("몹제거", &mut body, " \t ", None, None, None)
            .unwrap()
            .0,
        vec!["사용법: [몹 이름] 몹제거"]
    );
    assert_eq!(
        storage
            .execute("몹제거", &mut body, "2시험대상 뒤의말", None, None, None)
            .unwrap()
            .0,
        vec!["몹이 제거되었습니다."]
    );
    {
        let world = get_world_state().read().unwrap();
        assert_eq!(world.mob_cache.get_all_mobs_in_room(&zone, room).len(), 1);
        assert!(!world
            .get_room_object_order(&zone, room)
            .contains(&RoomObjectRef::Mob(first_id)));
    }
    assert_eq!(
        storage
            .execute("몹제거", &mut body, ".", None, None, None)
            .unwrap()
            .0,
        vec!["몹이 제거되었습니다."]
    );
    assert!(get_world_state().read().unwrap().get_room_object_order(&zone, room).iter().all(
        |object| !matches!(object, RoomObjectRef::Mob(id) if *id == first_id || *id == second_id)
    ));
    assert_eq!(
        storage
            .execute("몹제거", &mut body, "시험몹", None, None, None)
            .unwrap()
            .0,
        vec!["그런 몹이 없어요!"]
    );
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
}
