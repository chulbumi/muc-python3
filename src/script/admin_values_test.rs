use super::*;
#[test]
fn rank_command_uses_live_players_stable_ties_and_python_columns() {
    let online = [
        ("Alpha", 10_i64, 0_i64),
        ("Bravo", 20_i64, 0_i64),
        ("Charlie", 20_i64, 0_i64),
        ("Operator", 999_i64, 1000_i64),
        ("Zero", 0_i64, 0_i64),
    ]
    .into_iter()
    .map(|(name, strength, admin)| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name));
        map.insert("힘".into(), Dynamic::from(strength));
        map.insert("관리자등급".into(), Dynamic::from(admin));
        Dynamic::from(map)
    })
    .collect();
    set_precomputed_all_online(online);
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "순위조회자");
    body.set("은전", 50_000_i64);
    let invalid = storage
        .execute("순위", &mut body, "관리자등급", None, None, None)
        .unwrap();
    assert_eq!(invalid.0, vec!["☞ 사용법: [특성치] 순위"]);
    assert_eq!(body.get_int("은전"), 50_000);
    let poor = storage
        .execute("순위", &mut body, "힘", None, None, None)
        .unwrap();
    assert_eq!(poor.0, vec!["☞ 은전이 부족해요."]);
    assert_eq!(body.get_int("은전"), 50_000);
    body.set("은전", 200_000_i64);
    let normal = storage
        .execute("순위", &mut body, "힘", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("은전"), 100_000);
    assert_eq!(
        normal.0,
        vec!["[01] Bravo      [02] Charlie    [03] Alpha      "]
    );

    body.set("관리자등급", 1000_i64);
    let admin = storage
        .execute("순위", &mut body, "힘", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("은전"), 0);
    assert_eq!(
        admin.0,
        vec!["     Bravo 20                Charlie 20                  Alpha 10             \r\n"]
    );
    clear_precomputed_all_online();
    let _ = std::fs::remove_file("data/user/순위조회자.json");
}
#[test]
fn admin_status_uses_full_query_and_integrated_room_object_order() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin_name = format!("상태관리자-{suffix}");
    let target_name = format!("상태 대상자-{suffix}");
    let zone = format!("상태통합순서존-{suffix}");
    let room = "1";
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 1000_i64);
    let target_json = serde_json::json!({
        "name": target_name,
        "raw_attrs": {"반응이름": "공통 상태별칭 혼합접두별칭"},
        "level": 10, "age": 20, "hp": 30, "max_hp": 40,
        "mp": 5, "max_mp": 9, "attack": 1, "strength": 2,
        "armor": 3, "arm": 4, "dex": 5, "weight": 0,
        "current_exp": 6, "total_exp": 7, "hit": 8, "miss": 9,
        "critical": 10, "luck": 11, "silver": 12,
        "성격": "", "성별": "", "소속": "", "직위": "", "배우자": "",
        "feature": 0, "insurance_premium": 0, "hp_script": "",
        "mp_script": "", "nickname": "", "anger": 0,
        "targets": [], "guards": []
    });
    admin.temp_mut().insert(
        "_online_room_admin".into(),
        Value::String(serde_json::to_string(&vec![target_json]).unwrap()),
    );
    let mut item = Object::new();
    item.set("이름", "상태충돌물건");
    item.set("반응이름", "공통 상태별칭 혼합");
    let item = Arc::new(Mutex::new(item));
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin_name, PlayerPosition::new(zone.clone(), room.into()));
        world.set_player_position(&target_name, PlayerPosition::new(zone.clone(), room.into()));
        world.get_room_objs_mut(&zone, room).push(item.clone());
        world.record_floor_item(&zone, room, &item);
    }
    let storage = ScriptStorage::default();

    // The most recently inserted item is selected before the matching
    // player, and 상태보기 rejects items exactly like Python.
    let item_first = storage
        .execute("상태보기", &mut admin, "공통 상태별칭", None, None, None)
        .unwrap();
    assert_eq!(item_first.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    let second_match = storage
        .execute("상태보기", &mut admin, "2공통", None, None, None)
        .unwrap();
    assert!(second_match
        .0
        .iter()
        .any(|line| line.contains(&target_name)));

    // Python keeps exact-match (`c`) and prefix-match (`d`) numbering
    // separate.  The item is exact `혼합`, while the player only has the
    // prefix `혼합접두별칭`; therefore neither is the second match.
    let separate_exact_and_prefix_counts = storage
        .execute("상태보기", &mut admin, "2혼합", None, None, None)
        .unwrap();
    assert_eq!(
        separate_exact_and_prefix_counts.0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );

    item.lock().unwrap().set("투명상태", 1_i64);
    let transparent_item_skipped = storage
        .execute("상태보기", &mut admin, "공통 뒤의말", None, None, None)
        .unwrap();
    assert!(transparent_item_skipped
        .0
        .iter()
        .any(|line| line.contains(&target_name)));

    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, room).clear();
        world.remove_floor_item_record(&zone, room, &item);
    }
    let player = storage
        .execute("상태보기", &mut admin, "공통 상태별칭", None, None, None)
        .unwrap();
    assert!(player.0.iter().any(|line| line.contains(&target_name)));
    assert!(player.0.iter().any(|line| line.contains("30/40")));

    let first_word_only = storage
        .execute("상태보기", &mut admin, "공통", None, None, None)
        .unwrap();
    assert!(first_word_only
        .0
        .iter()
        .any(|line| line.contains(&target_name)));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin_name);
    world.remove_player_position(&target_name);
}

#[test]
fn comma_value_command_executes_python_assignment_branches() {
    use crate::object::Object;
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player_name = format!("값값회귀-{suffix}");
    let zone = format!("값값회귀존-{suffix}");
    let mut floor_item = Object::new();
    floor_item.set("이름", "시험석");
    floor_item.set("반응이름", "시험석\r\n돌\r\n충돌\r\n혼합");
    floor_item.set("무게", 10_i64);
    floor_item.set("설명", "기존");
    let floor_item = Arc::new(Mutex::new(floor_item));

    let mob_key = format!("{zone}:시험몹");
    let mut mob_data = RawMobData::new();
    mob_data.name = "시험몹".to_string();
    mob_data.reaction_names = vec!["충돌".to_string(), "혼합접두별칭".to_string()];
    mob_data.zone = zone.clone();
    mob_data.max_hp = 100;
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            &player_name,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
        world.get_room_objs_mut(&zone, "1").push(floor_item.clone());
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), mob_data.clone());
        let mob = MobInstance::new(mob_key, zone.clone(), "1", &mob_data);
        let mob_id = mob.instance_id;
        world.mob_cache.add_mob_instance(mob);
        world.record_floor_item(&zone, "1", &floor_item);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(mob_id));
    }

    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();

    let usage = storage
        .execute("값값", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0.len(), 2);
    assert!(usage.0[0].parse::<i64>().is_ok());
    assert_eq!(usage.0[1], "☞ 사용법: [대상],[키],[값] 값설정");

    let too_long = storage
        .execute(
            "값값",
            &mut body,
            &format!("시험석,설명,{}", "긴".repeat(21)),
            None,
            None,
            None,
        )
        .unwrap();
    assert!(too_long.0[0].parse::<i64>().is_ok());
    assert_eq!(too_long.0[1], "☞ 너무 길어요!");
    assert_eq!(floor_item.lock().unwrap().getString("설명"), "기존");

    let numeric = storage
        .execute("값값", &mut body, "시험석,무게,25", None, None, None)
        .unwrap();
    assert_eq!(numeric.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(floor_item.lock().unwrap().getInt("무게"), 25);

    let string_with_comma = storage
        .execute("값값", &mut body, "시험석,설명,쉼표,포함", None, None, None)
        .unwrap();
    assert_eq!(string_with_comma.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(floor_item.lock().unwrap().getString("설명"), "쉼표,포함");
    assert_eq!(
        reaction_names(&floor_item.lock().unwrap().getString("반응이름")),
        vec![
            "시험석".to_string(),
            "돌".to_string(),
            "충돌".to_string(),
            "혼합".to_string(),
        ]
    );

    let reaction_prefix = storage
        .execute("값값", &mut body, "시험,설명,접두별칭", None, None, None)
        .unwrap();
    assert_eq!(reaction_prefix.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(floor_item.lock().unwrap().getString("설명"), "접두별칭");

    let integrated_mob_first = storage
        .execute("값값", &mut body, "충돌,체력,66", None, None, None)
        .unwrap();
    assert_eq!(integrated_mob_first.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, "1")[0]
            .hp,
        66
    );
    assert!(!floor_item.lock().unwrap().attr.contains_key("체력"));

    let numbered_exact = storage
        .execute("값값", &mut body, "2충돌,체력,44", None, None, None)
        .unwrap();
    assert_eq!(numbered_exact.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(floor_item.lock().unwrap().getInt("체력"), 44);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, "1")[0]
            .hp,
        66
    );

    let separate_exact_prefix = storage
        .execute("값값", &mut body, "2혼합,체력,99", None, None, None)
        .unwrap();
    assert_eq!(separate_exact_prefix.0[1], "☞ 그런 대상이 없어요!");
    assert_eq!(floor_item.lock().unwrap().getInt("체력"), 44);

    let numeric_room_mob = storage
        .execute("값값", &mut body, "1,체력,55", None, None, None)
        .unwrap();
    assert_eq!(numeric_room_mob.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, "1")[0]
            .hp,
        55
    );

    let mut summoned = Body::new();
    summoned.set("이름", "소환값대상");
    summoned.set("반응이름", "소환값별칭");
    summoned.set("레벨", 3_i64);
    let summoned_id = get_world_state()
        .write()
        .unwrap()
        .add_summoned_user(summoned, PlayerPosition::new(zone.clone(), "1".to_string()));
    let summoned_value = storage
        .execute("값값", &mut body, "소환값,레벨,9", None, None, None)
        .unwrap();
    assert_eq!(summoned_value.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .summoned_users()
            .iter()
            .find(|user| user.id == summoned_id)
            .unwrap()
            .body
            .get_int("레벨"),
        9
    );
    assert!(get_world_state()
        .write()
        .unwrap()
        .remove_summoned_user("소환값대상"));

    let mut installed = Object::new();
    installed.set("이름", "설치값보관함");
    installed.set("반응이름", "설치값별칭");
    installed.set("은전", 10_i64);
    let installed = Arc::new(Mutex::new(installed));
    box_commands::register_installed_box(&zone, "1", installed.clone());
    let installed_value = storage
        .execute("값값", &mut body, "설치값,은전,77", None, None, None)
        .unwrap();
    assert_eq!(installed_value.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(installed.lock().unwrap().getInt("은전"), 77);

    let invalid = storage
        .execute("값값", &mut body, "시험석,무게,아님", None, None, None)
        .unwrap();
    assert_eq!(invalid.0[1], "☞ 잘못된 값입니다.");
    assert_eq!(floor_item.lock().unwrap().getInt("무게"), 25);

    let mob = storage
        .execute("값값", &mut body, "시험몹,체력,77", None, None, None)
        .unwrap();
    assert_eq!(mob.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, "1")[0]
            .hp,
        77
    );

    let missing = storage
        .execute("값값", &mut body, "없는것,키,값", None, None, None)
        .unwrap();
    assert_eq!(missing.0[1], "☞ 그런 대상이 없어요!");

    body.temp_mut().insert(
        "_online_room_admin".into(),
        Value::String(
            serde_json::json!([{
                "name": "다른무림인",
                "raw_attrs": {"레벨": 10, "설명": "기존설명"}
            }])
            .to_string(),
        ),
    );
    let other_player = storage
        .execute("값값", &mut body, "다른무림인,레벨,33", None, None, None)
        .unwrap();
    assert_eq!(other_player.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(
        take_admin_set_player_value_request(&mut body),
        Some((
            "다른무림인".to_string(),
            "레벨".to_string(),
            serde_json::json!(33)
        ))
    );
    let mut self_collision = Object::new();
    self_collision.set("이름", player_name.as_str());
    let self_collision = Arc::new(Mutex::new(self_collision));
    {
        let mut world = get_world_state().write().unwrap();
        world
            .get_room_objs_mut(&zone, "1")
            .push(self_collision.clone());
        world.record_floor_item(&zone, "1", &self_collision);
    }
    let self_item_first = storage
        .execute(
            "값설정",
            &mut body,
            &format!("{player_name} 자기충돌값 77"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(self_item_first.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(self_collision.lock().unwrap().getInt("자기충돌값"), 77);
    assert!(!body.object.attr.contains_key("자기충돌값"));
    let numbered_item = storage
        .execute(
            "값설정",
            &mut body,
            "2충돌 설명 값설정둘째",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(numbered_item.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(floor_item.lock().unwrap().getString("설명"), "값설정둘째");
    assert!(!get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room(&zone, "1")[0]
        .runtime_attrs
        .contains_key("설명"));
    let invalid_other_player = storage
        .execute(
            "값값",
            &mut body,
            "다른무림인,레벨,숫자아님",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(invalid_other_player.0[1], "☞ 잘못된 값입니다.");
    assert_eq!(take_admin_set_player_value_request(&mut body), None);

    get_world_state().write().unwrap().set_player_position(
        "다른무림인",
        PlayerPosition::new(zone.clone(), "1".to_string()),
    );
    let mut other_snapshot = rhai::Map::new();
    other_snapshot.insert("이름".into(), Dynamic::from("다른무림인"));
    other_snapshot.insert("반응이름".into(), Dynamic::from(""));
    set_precomputed_room_view_players(std::collections::HashMap::from([(
        format!("{zone}:1"),
        vec![Dynamic::from(other_snapshot)],
    )]));
    let delete_other_player = storage
        .execute("값삭제", &mut body, "다른무림인 레벨", None, None, None)
        .unwrap();
    assert_eq!(delete_other_player.0, vec!["☞ 값이 삭제되었습니다."]);
    assert_eq!(
        take_admin_set_player_value_request(&mut body),
        Some((
            "다른무림인".to_string(),
            "레벨".to_string(),
            serde_json::Value::Null
        ))
    );
    let delete_missing_other_key = storage
        .execute("값삭제", &mut body, "다른무림인 없는키", None, None, None)
        .unwrap();
    assert_eq!(delete_missing_other_key.0, vec!["☞ 해당 키가 없습니다."]);
    assert_eq!(take_admin_set_player_value_request(&mut body), None);

    // 새 키는 Python이 int()만 시도하므로 소수 표기는 문자열이다.
    let decimal_new = storage
        .execute("값값", &mut body, "시험석,새값,1.5", None, None, None)
        .unwrap();
    assert_eq!(decimal_new.0[1], "☞ 값이 설정되었습니다.");
    assert_eq!(
        floor_item.lock().unwrap().attr.get("새값"),
        Some(&Value::String("1.5".to_string()))
    );

    assert_eq!(
        python_coerce_attribute(None, "값 "),
        Ok(Value::String("값 ".to_string()))
    );

    let space_other_player = storage
        .execute(
            "값설정",
            &mut body,
            "다른무림인 설명 새 설명",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(space_other_player.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(
        take_admin_set_player_value_request(&mut body),
        Some((
            "다른무림인".to_string(),
            "설명".to_string(),
            serde_json::json!("새 설명")
        ))
    );

    let spaced = storage
        .execute(
            "값설정",
            &mut body,
            "시험석 설명 공백이 들어간 설명",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(spaced.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(
        floor_item.lock().unwrap().getString("설명"),
        "공백이 들어간 설명"
    );

    let long_tail = format!("시험석 설명 짧음 {}", "뒤".repeat(60));
    let accepted_long_tail = storage
        .execute("값설정", &mut body, &long_tail, None, None, None)
        .unwrap();
    assert_eq!(accepted_long_tail.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(
        floor_item.lock().unwrap().getString("설명"),
        format!("짧음 {}", "뒤".repeat(60))
    );
    let too_long_third = format!("시험석 설명 {} 뒤", "긴".repeat(51));
    let rejected_third = storage
        .execute("값설정", &mut body, &too_long_third, None, None, None)
        .unwrap();
    assert_eq!(rejected_third.0, vec!["☞ 너무 길어요!"]);

    let mut inventory_item = Object::new();
    inventory_item.set("이름", "소지시험품");
    inventory_item.set("반응이름", "소지시험품");
    inventory_item.set("무게", 3_i64);
    let inventory_item = Arc::new(Mutex::new(inventory_item));
    body.object.append(inventory_item.clone());
    let inventory = storage
        .execute("값설정", &mut body, "소지시험품 무게 9", None, None, None)
        .unwrap();
    assert_eq!(inventory.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(inventory_item.lock().unwrap().getInt("무게"), 9);

    let room_attr = storage
        .execute(
            "값설정",
            &mut body,
            "방 시험속성 여러 단어 값",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(room_attr.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .room_attrs
            .get(&format!("{zone}:1"))
            .and_then(|attrs| attrs.get("시험속성"))
            .map(String::as_str),
        Some("여러 단어 값")
    );

    let invalid_space = storage
        .execute(
            "값설정",
            &mut body,
            "소지시험품 무게 숫자아님",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(invalid_space.0, vec!["☞ 잘못된 값입니다."]);
    assert_eq!(inventory_item.lock().unwrap().getInt("무게"), 9);

    let delete_usage = storage
        .execute("값삭제", &mut body, "시험석", None, None, None)
        .unwrap();
    assert_eq!(delete_usage.0, vec!["☞ 사용법: [대상] [키] 값삭제"]);

    floor_item.lock().unwrap().set("삭제표식", "바닥");
    get_world_state()
        .write()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room_mut(&zone, "1")
        .unwrap()[0]
        .runtime_attrs
        .insert("삭제표식".into(), Value::String("몹".into()));
    let delete_numbered_floor = storage
        .execute("값삭제", &mut body, "2충돌 삭제표식", None, None, None)
        .unwrap();
    assert_eq!(delete_numbered_floor.0, vec!["☞ 값이 삭제되었습니다."]);
    assert!(!floor_item.lock().unwrap().attr.contains_key("삭제표식"));
    assert!(get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room(&zone, "1")[0]
        .runtime_attrs
        .contains_key("삭제표식"));
    let delete_integrated_first = storage
        .execute("값삭제", &mut body, "충돌 삭제표식", None, None, None)
        .unwrap();
    assert_eq!(delete_integrated_first.0, vec!["☞ 값이 삭제되었습니다."]);
    assert!(!get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room(&zone, "1")[0]
        .runtime_attrs
        .contains_key("삭제표식"));

    let delete_floor = storage
        .execute("값삭제", &mut body, "시험석 설명", None, None, None)
        .unwrap();
    assert_eq!(delete_floor.0, vec!["☞ 값이 삭제되었습니다."]);
    assert!(!floor_item.lock().unwrap().attr.contains_key("설명"));
    let delete_floor_again = storage
        .execute("값삭제", &mut body, "시험석 설명", None, None, None)
        .unwrap();
    assert_eq!(delete_floor_again.0, vec!["☞ 해당 키가 없습니다."]);

    // Python 값삭제는 env.findObjName만 사용하므로 소지품을 찾지 않는다.
    let inventory_not_environment = storage
        .execute("값삭제", &mut body, "소지시험품 무게", None, None, None)
        .unwrap();
    assert_eq!(inventory_not_environment.0, vec!["☞ 그런 대상이 없어요!"]);
    assert_eq!(inventory_item.lock().unwrap().getInt("무게"), 9);

    get_world_state()
        .write()
        .unwrap()
        .get_room_attrs_mut(&zone, "1")
        .insert("시험 속성".into(), "값".into());
    let delete_room_spaced_key = storage
        .execute("값삭제", &mut body, "방 시험 속성", None, None, None)
        .unwrap();
    assert_eq!(delete_room_spaced_key.0, vec!["☞ 값이 삭제되었습니다."]);
    assert!(!get_world_state()
        .read()
        .unwrap()
        .room_attrs
        .get(&format!("{zone}:1"))
        .is_some_and(|attrs| attrs.contains_key("시험 속성")));

    let delete_missing = storage
        .execute("값삭제", &mut body, "없는대상 없는키", None, None, None)
        .unwrap();
    assert_eq!(delete_missing.0, vec!["☞ 그런 대상이 없어요!"]);

    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&player_name);
    world.remove_player_position("다른무림인");
    drop(world);
    set_precomputed_room_view_players(std::collections::HashMap::new());
}
