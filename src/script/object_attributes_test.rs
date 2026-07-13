use super::*;
#[test]
fn attributes_command_reads_room_json_then_room_object_and_inventory_fallback() {
    use crate::object::Object;
    use crate::world::{get_world_state, PlayerPosition, RoomObjectRef};

    let suffix = std::process::id();
    let player = format!("속성회귀-{suffix}");
    let zone = format!("속성회귀존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::write(
        room_dir.join("1.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "맵정보": {"이름": "속성 시험방", "설명": ["첫줄", "둘째줄"], "출구": []}
        }))
        .unwrap(),
    )
    .unwrap();
    let mut floor = Object::new();
    floor.set("이름", "바닥옥패");
    floor.set("반응이름", "옥패 속성충돌");
    floor.set("시험수치", 17_i64);
    let floor = Arc::new(Mutex::new(floor));
    let mob_key = format!("{zone}:속성시험몹");
    let mut mob_data = RawMobData::new();
    mob_data.name = "속성시험몹".into();
    mob_data.reaction_names = vec!["속성충돌".into()];
    mob_data.zone = zone.clone();
    mob_data.max_hp = 88;
    mob_data
        .attributes
        .insert("시험표식".into(), serde_json::json!("몹값"));
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
        world.get_room_objs_mut(&zone, "1").push(floor.clone());
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), mob_data.clone());
        let mob = MobInstance::new(mob_key.clone(), zone.clone(), "1", &mob_data);
        let mob_id = mob.instance_id;
        world.mob_cache.add_mob_instance(mob);
        world.record_floor_item(&zone, "1", &floor);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(mob_id));
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("반응이름", "속성자신별칭");
    body.set("본인표식", "본인값");
    body.set("관리자등급", 1000_i64);
    let mut inventory = Object::new();
    inventory.set("이름", "소지옥패");
    inventory.set("반응이름", "소지패");
    inventory.set("문자값", "보관값");
    body.object.append(Arc::new(Mutex::new(inventory)));
    let mut hidden_inventory = Object::new();
    hidden_inventory.set("이름", "숨긴패");
    hidden_inventory.set("반응이름", "숨긴패");
    hidden_inventory.set("아이템속성", "출력안함");
    hidden_inventory.set("문자값", "보이면안됨");
    body.object.append(Arc::new(Mutex::new(hidden_inventory)));
    let storage = ScriptStorage::default();

    let room = storage
        .execute("속성", &mut body, "", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(room.contains("#설명\r\n첫줄\r\n둘째줄\r\n\r\n"));
    assert!(room.contains("#이름\r\n속성 시험방\r\n\r\n"));

    let floor = storage
        .execute("속성", &mut body, "옥패", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(floor.contains("#시험수치\r\n17\r\n\r\n"));
    let floor_with_tail = storage
        .execute("속성", &mut body, "옥패 뒤는무시", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(floor_with_tail.contains("#시험수치\r\n17\r\n\r\n"));
    let numeric_mob = storage
        .execute("속성", &mut body, "1", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(numeric_mob.contains("#이름\r\n속성시험몹\r\n\r\n"));
    assert!(numeric_mob.contains("#시험표식\r\n몹값\r\n\r\n"));
    let integrated_mob_first = storage
        .execute("속성", &mut body, "속성충돌", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(integrated_mob_first.contains("#시험표식\r\n몹값\r\n\r\n"));
    assert!(!integrated_mob_first.contains("#시험수치\r\n17\r\n\r\n"));
    let numbered_floor_second = storage
        .execute("속성", &mut body, "2속성충돌", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(numbered_floor_second.contains("#시험수치\r\n17\r\n\r\n"));
    assert!(!numbered_floor_second.contains("#시험표식\r\n몹값\r\n\r\n"));
    let inventory = storage
        .execute("속성", &mut body, "소지패", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(inventory.contains("#문자값\r\n보관값\r\n\r\n"));
    let self_by_alias = storage
        .execute("속성", &mut body, "속성자신", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(self_by_alias.contains("#본인표식\r\n본인값\r\n\r\n"));

    // Python에서 `방`과 `나`는 이 명령의 특별 별칭이 아니다.
    let literal_room = storage
        .execute("속성", &mut body, "방", None, None, None)
        .unwrap();
    assert_eq!(literal_room.0, vec!["☞ 그런 대상이 없어요!"]);
    let literal_self = storage
        .execute("속성", &mut body, "나", None, None, None)
        .unwrap();
    assert_eq!(literal_self.0, vec!["☞ 그런 대상이 없어요!"]);

    // Object.findObjName()은 출력안함 소지품을 건너뛴다.
    let hidden = storage
        .execute("속성", &mut body, "숨긴패", None, None, None)
        .unwrap();
    assert_eq!(hidden.0, vec!["☞ 그런 대상이 없어요!"]);

    // 방 검색 실패 후 소지품에는 원문 전체가 전달된다.
    let inventory_with_tail = storage
        .execute("속성", &mut body, "소지패 뒤", None, None, None)
        .unwrap();
    assert_eq!(inventory_with_tail.0, vec!["☞ 그런 대상이 없어요!"]);

    let mut installed = Object::new();
    installed.set("이름", "속성설치함");
    installed.set("반응이름", "설치속성별칭");
    installed.set("설치표식", "설치값");
    box_commands::register_installed_box(&zone, "1", Arc::new(Mutex::new(installed)));
    let installed_attrs = storage
        .execute("속성", &mut body, "설치속성", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(installed_attrs.contains("#설치표식\r\n설치값\r\n\r\n"));

    let mut summoned = Body::new();
    summoned.set("이름", "속성소환대상");
    summoned.set("반응이름", "소환속성별칭");
    summoned.set("소환표식", "소환값");
    get_world_state()
        .write()
        .unwrap()
        .add_summoned_user(summoned, PlayerPosition::new(zone.clone(), "1".to_string()));
    let summoned_attrs = storage
        .execute("속성", &mut body, "소환속성", None, None, None)
        .unwrap()
        .0
        .join("");
    assert!(summoned_attrs.contains("#소환표식\r\n소환값\r\n\r\n"));
    assert!(get_world_state()
        .write()
        .unwrap()
        .remove_summoned_user("속성소환대상"));

    let missing = storage
        .execute("속성", &mut body, "없는대상", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 대상이 없어요!"]);

    let _ = std::fs::remove_dir_all(room_dir);
    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&player);
    world.mob_cache.remove_mob(&mob_key);
}
#[test]
fn save_object_command_handles_room_item_mob_and_room_as_valid_python_targets() {
    use crate::object::Object;
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player_name = format!("저장회귀-{suffix}");
    let zone = format!("저장회귀존-{suffix}");
    let item_key = format!("저장회귀아이템-{suffix}");
    let mob_file = format!("저장회귀몹-{suffix}");
    let mob_key = format!("{zone}:{mob_file}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    let room_path = room_dir.join("1.json");
    let mob_dir = std::path::Path::new("data/mob").join(&zone);
    let mob_path = mob_dir.join(format!("{mob_file}.json"));
    let item_path = std::path::Path::new("data/item").join(format!("{item_key}.json"));
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::create_dir_all(&mob_dir).unwrap();
    std::fs::write(
        &room_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "맵정보": {"이름": "저장 시험방", "존이름": zone, "설명": [], "출구": []}
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        &mob_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "몹정보": {"이름": "저장시험몹", "체력": 100, "최고체력": 100}
        }))
        .unwrap(),
    )
    .unwrap();

    let mut floor_item = Object::new();
    floor_item.set("이름", "저장시험석");
    floor_item.set("반응이름", "저장시험석\r\n시험석\r\n저장공통");
    floor_item.set("인덱스", item_key.as_str());
    floor_item.set("종류", "일반");
    floor_item.set("설명1", "변경된 설명");
    let floor_item = Arc::new(Mutex::new(floor_item));

    let mut mob_data = RawMobData::new();
    mob_data.name = "저장시험몹".into();
    mob_data.reaction_names = vec!["저장공통".into()];
    mob_data.zone = zone.clone();
    mob_data.max_hp = 100;
    mob_data.attributes.insert(
        "이름".into(),
        serde_json::Value::String("저장시험몹".into()),
    );
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
        let mut instance = MobInstance::new(mob_key.clone(), zone.clone(), "1", &mob_data);
        instance.hp = 73;
        instance
            .runtime_attrs
            .insert("시험값".into(), Value::Int(9));
        let mob_id = instance.instance_id;
        world.mob_cache.add_mob_instance(instance);
        world.record_floor_item(&zone, "1", &floor_item);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(mob_id));
    }

    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();

    let room_saved = storage
        .execute("오브젝트저장", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        room_saved.0,
        vec![format!("* data/map/{zone}/1.json 저장되었습니다.")]
    );
    let item_saved = storage
        .execute("오브젝트저장", &mut body, "시험석", None, None, None)
        .unwrap();
    assert_eq!(
        item_saved.0,
        vec![format!("* data/item/{item_key}.json 저장되었습니다.")]
    );
    let item_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&item_path).unwrap()).unwrap();
    assert_eq!(item_json["아이템정보"]["설명1"], "변경된 설명");

    let mob_saved = storage
        .execute("오브젝트저장", &mut body, "저장시험몹", None, None, None)
        .unwrap();
    assert_eq!(
        mob_saved.0,
        vec![format!("* data/mob/{zone}/{mob_file}.json 저장되었습니다.")]
    );
    let mob_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mob_path).unwrap()).unwrap();
    assert_eq!(mob_json["몹정보"]["체력"], 73);
    assert_eq!(mob_json["몹정보"]["시험값"], 9);

    let shared_mob_first = storage
        .execute("오브젝트저장", &mut body, "저장공통", None, None, None)
        .unwrap();
    assert_eq!(
        shared_mob_first.0,
        vec![format!("* data/mob/{zone}/{mob_file}.json 저장되었습니다.")]
    );
    let numbered_item_second = storage
        .execute("오브젝트저장", &mut body, "2저장공통", None, None, None)
        .unwrap();
    assert_eq!(
        numbered_item_second.0,
        vec![format!("* data/item/{item_key}.json 저장되었습니다.")]
    );

    let missing = storage
        .execute("오브젝트저장", &mut body, "없는대상", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 대상이 없어요!"]);

    let _ = std::fs::remove_file(item_path);
    let _ = std::fs::remove_dir_all(room_dir);
    let _ = std::fs::remove_dir_all(mob_dir);
    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&player_name);
    world.mob_cache.remove_mob(&mob_key);
}
#[test]
fn test_build_ob_exposes_all_object_attributes() {
    let mut body = Body::new();
    body.set("이름", "속성검사");
    body.set("사용자정의문자", "값");
    body.set("사용자정의정수", 77i64);
    body.object.set("사용자정의실수", 1.5f64);

    let map = build_ob_from_body(&body);
    assert_eq!(map["사용자정의문자"].clone().into_string().unwrap(), "값");
    assert_eq!(map["사용자정의정수"].as_int().unwrap(), 77);
    assert_eq!(map["사용자정의실수"].as_float().unwrap(), 1.5);
}

#[test]
fn attributes_view_prints_other_online_players_full_sorted_raw_attr_map() {
    use crate::world::{get_world_state, PlayerPosition, RoomObjectRef};

    let suffix = std::process::id();
    let admin_name = format!("속성조회관리자-{suffix}");
    let target_name = format!("속성조회대상-{suffix}");
    let zone = format!("속성조회존-{suffix}");
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 1000_i64);
    admin.temp_mut().insert(
        "_online_room_admin".into(),
        Value::String(
            serde_json::json!([{
                "name": target_name,
                "raw_attrs": {
                    "하": ["첫째", "둘째"],
                    "가": 17,
                    "나": "문자값"
                }
            }])
            .to_string(),
        ),
    );
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.record_test_room_object(&zone, "1", RoomObjectRef::Player(target_name.clone()));
    }
    let output = ScriptStorage::default()
        .execute("속성", &mut admin, &target_name, None, None, None)
        .unwrap();
    assert_eq!(
        output.0,
        vec!["#가\r\n17\r\n\r\n#나\r\n문자값\r\n\r\n#하\r\n첫째\r\n둘째\r\n\r\n"]
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin_name);
    world.remove_player_position(&target_name);
}

#[test]
fn attribute_append_and_remove_mutate_socketless_player_and_keep_python_messages() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin_name = format!("속성변경관리자-{suffix}");
    let target_name = format!("속성변경소환체-{suffix}");
    let zone = format!("속성변경존-{suffix}");
    let mut target = Body::new();
    target.set("이름", target_name.as_str());
    target.set("반응이름", "속성소환별칭");
    let id = {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.add_summoned_user(target, PlayerPosition::new(zone, "1".into()))
    };
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();

    assert_eq!(
        storage
            .execute(
                "속성추가",
                &mut admin,
                "속성소환 표식 첫째",
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["☞ 속성이 추가 되었습니다."]
    );
    assert_eq!(
        storage
            .execute(
                "속성추가",
                &mut admin,
                "속성소환 표식 둘째 뒤는무시",
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["☞ 속성이 추가 되었습니다."]
    );
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .summoned_users()
            .iter()
            .find(|user| user.id == id)
            .unwrap()
            .body
            .get_string("표식"),
        "첫째\r\n둘째"
    );
    assert_eq!(
        storage
            .execute(
                "속성제거",
                &mut admin,
                "속성소환 표식 첫째",
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["☞ 속성이 제거 되었습니다."]
    );
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .summoned_users()
            .iter()
            .find(|user| user.id == id)
            .unwrap()
            .body
            .get_string("표식"),
        "둘째"
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_summoned_user_by_id(id);
    world.remove_player_position(&admin_name);
}
