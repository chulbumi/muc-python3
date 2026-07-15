use super::*;
#[test]
fn item_editor_and_delete_match_python_runtime_registry_semantics() {
    use crate::command::handler::{CommandResult, PendingInput};

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 0_i64);
    let usage = storage
        .execute("아이템제작", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [파일명] 아이템제작"]);
    let denied = storage
        .execute("아이템제작", &mut body, "시험파일", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^"]);
    body.set("관리자등급", 1000_i64);
    let editor = storage
        .execute(
            "아이템제작",
            &mut body,
            "  시험파일\t뒤의값은무시 ",
            None,
            None,
            None,
        )
        .unwrap();
    assert!(matches!(
        editor.1,
        Some(CommandResult::RequestInput {
            ref prompt,
            state: PendingInput::FileEdit { ref relative_path, ref lines }
        }) if prompt == "작성을 마치시려면 '.' 를 입력하세요.\r\n:"
            && relative_path == "item/시험파일.json" && lines.is_empty()
    ));

    body.set("관리자등급", 1999_i64);
    let delete_denied = storage
        .execute("아이템삭제", &mut body, "없는항목", None, None, None)
        .unwrap();
    assert_eq!(delete_denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    body.set("관리자등급", 2000_i64);
    let delete_usage = storage
        .execute("아이템삭제", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(delete_usage.0, vec!["사용법: [아이템 인덱스] 아이템삭제"]);
    let missing_key = format!("없는아이템삭제-{}", std::process::id());
    let _ = std::fs::remove_file(format!("data/item/{missing_key}.json"));
    let delete_missing = storage
        .execute("아이템삭제", &mut body, &missing_key, None, None, None)
        .unwrap();
    assert_eq!(delete_missing.0, vec!["존재하지않는 아이템입니다."]);

    let key = format!("아이템삭제회귀-{}", std::process::id());
    let path = std::path::Path::new("data/item").join(format!("{key}.json"));
    std::fs::write(
            &path,
            serde_json::json!({"아이템정보":{"이름":"삭제시험품","종류":"기타","반응이름":["삭제시험품"]}})
                .to_string(),
        )
        .unwrap();
    get_world_state()
        .write()
        .unwrap()
        .item_cache
        .load_item(&key)
        .unwrap();
    let existing = object_from_item_json(&key).unwrap().0;
    body.object.objs.push(existing.clone());
    let deleted = storage
        .execute("아이템삭제", &mut body, &key, None, None, None)
        .unwrap();
    assert_eq!(deleted.0, vec!["아이템이 삭제되었습니다."]);
    assert!(path.exists());
    assert_eq!(existing.lock().unwrap().getName(), "삭제시험품");
    let recreated = storage
        .execute("생성", &mut body, &key, None, None, None)
        .unwrap();
    assert_eq!(
        recreated.0,
        vec!["\x1b[1;32m* [삭제시험품] 생성 되었습니다.\x1b[0;37m"]
    );
    assert_eq!(body.get_item_count(), 2);
    assert_eq!(body.object.inv_stack.get(&key), Some(&1));
    let deleted_again = storage
        .execute("아이템삭제", &mut body, &key, None, None, None)
        .unwrap();
    assert_eq!(deleted_again.0, vec!["아이템이 삭제되었습니다."]);
    assert!(path.exists(), "Python never deletes the item JSON file");
    std::fs::remove_file(path).unwrap();
}
#[test]
fn pickup_uses_python_raw_mode_numeric_prefix_actual_name_particle_and_room_notice() {
    use crate::command::handler::CommandResult;
    use crate::script::party::set_precomputed_party_context;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let actor = format!("습득자-{suffix}");
    let observer = format!("습득목격자-{suffix}");
    let zone = format!("습득시험존-{suffix}");
    let make_item = |ansi: &str| {
        let mut item = Object::new();
        item.set("이름", "설삼과");
        item.set("반응이름", "설삼과\r\n열매");
        item.set("안시", ansi);
        item.set("무게", 1_i64);
        Arc::new(Mutex::new(item))
    };
    let colored = make_item("\x1b[1;31m");
    let first = make_item("");
    let second = make_item("");
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&actor, &observer] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".to_string()));
        }
        for item in [&colored, &first, &second] {
            world.get_room_objs_mut(&zone, "1").push(item.clone());
            world.record_floor_item(&zone, "1", item);
        }
    }
    let mut body = Body::new();
    body.set("이름", actor.as_str());
    body.set("힘", 100_i64);
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(observer.clone()));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("hp".into(), Dynamic::from(27_i64));
    person.insert("max_hp".into(), Dynamic::from(37_i64));
    person.insert("mp".into(), Dynamic::from(4_i64));
    person.insert("max_mp".into(), Dynamic::from(5_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);
    let storage = ScriptStorage::default();

    let single = storage
        .execute("가져", &mut body, "열매", None, None, None)
        .unwrap();
    assert_eq!(
        single.0,
        vec!["당신이 \x1b[36m\x1b[1;31m설삼과\x1b[0;37m를\x1b[37m 집어서 품속에 갈무리 합니다."]
    );
    let sends = match single.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected pickup delivery: {other:?}"),
    };
    assert_eq!(sends.len(), 1);
    assert_eq!(sends[0].0, observer);
    assert!(sends[0].1.contains("설삼과\x1b[0;37m를"));
    assert!(sends[0].1.ends_with("\r\n\x1b[0;37;40m[ 27/37, 4/5 ] "));

    let grouped = storage
        .execute("가져", &mut body, "열매 2개", None, None, None)
        .unwrap();
    assert_eq!(
        grouped.0,
        vec!["당신이 \x1b[36m설삼과\x1b[37m 2개를 집어서 품속에 갈무리 합니다."]
    );
    assert_eq!(body.object.objs.len(), 3);
    assert!(get_world_state()
        .read()
        .unwrap()
        .get_room_objs(&zone, "1")
        .is_empty());

    let spaced_all = storage
        .execute("가져", &mut body, " 모두 ", None, None, None)
        .unwrap();
    assert_eq!(spaced_all.0, vec!["☞ 더이상 가질 물건이 없다네"]);

    let mut all_colored = Object::new();
    all_colored.set("이름", "붉은열매");
    all_colored.set("안시", "\x1b[1;31m");
    all_colored.set("무게", 1_i64);
    let all_colored = Arc::new(Mutex::new(all_colored));
    {
        let mut world = get_world_state().write().unwrap();
        world
            .get_room_objs_mut(&zone, "1")
            .push(all_colored.clone());
        world.record_floor_item(&zone, "1", &all_colored);
    }
    let all = storage
        .execute("가져", &mut body, "모두", None, None, None)
        .unwrap();
    assert_eq!(
        all.0,
        vec!["당신이 \x1b[36m\x1b[1;31m붉은열매\x1b[0;37m를\x1b[37m 집어서 품속에 갈무리 합니다."]
    );
    let all_sends = match all.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected all-pickup delivery: {other:?}"),
    };
    assert!(all_sends[0].1.starts_with(&format!(
        "{}\r\n\x1b[1m{actor}\x1b[0;37m{} ",
        RAW_USER_MESSAGE_PREFIX,
        han_iga(&actor)
    )));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&actor);
    world.remove_player_position(&observer);
    set_precomputed_party_context(rhai::Map::new());
    let _ = std::fs::remove_file(format!("data/user/{actor}.json"));
}

#[test]
fn set_option_accepts_python_decimal_underscore_syntax() {
    let mut body = Body::new();
    body.set("관리자등급", 2000_i64);
    let item = Arc::new(Mutex::new(Object::new()));
    item.lock().unwrap().set("이름", "밑줄검");
    body.object.objs.push(item.clone());

    let output = ScriptStorage::default()
        .execute("옵설정", &mut body, "밑줄검 힘 +1_000", None, None, None)
        .unwrap();

    assert_eq!(output.0, vec!["☞ 설정되었습니다."]);
    assert_eq!(item.lock().unwrap().getString("옵션"), "힘 1000");
}

#[test]
fn random_magic_preserves_python_option_roll_insertion_order() {
    let mut item = Object::new();
    item.set("이름", "순서검");
    item.set("종류", "무기");
    item.set("계층", "무기");
    item.set("공격력", 0_i64);
    item.set("방어력", 0_i64);
    // gate, 10/20/50% bonus rolls, then four option-index/value pairs
    let rolls = [0, 10, 20, 0, 0, 999, 2, 999, 3, 999, 4, 999];
    let mut index = 0usize;
    let applied = apply_item_magic_with_roll(&mut item, 10_000, 2, false, &mut |low, high| {
        let value = rolls.get(index).copied().unwrap_or(low);
        index += 1;
        value.clamp(low, high)
    });
    assert!(applied);
    let lines = item.getString("옵션");
    let options = lines
        .lines()
        .map(|line| line.split_whitespace().next().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(options.len(), 4);
    assert_eq!(options, vec!["힘", "맷집", "체력", "내공"]);
}

#[test]
fn compact_inventory_admin_can_view_socketless_summoned_player() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let viewer_name = format!("소소소환관리자-{suffix}");
    let target_name = format!("소소소환대상-{suffix}");
    let zone = format!("소소소환존-{suffix}");
    let mut target = Body::new();
    target.set("이름", target_name.as_str());
    target.set("반응이름", "소소소환별칭");
    target.set("은전", 2468_i64);
    let mut herb = Object::new();
    herb.set("이름", "소환약초");
    target.object.objs.push(Arc::new(Mutex::new(herb)));
    let id = {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&viewer_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.add_summoned_user(target, PlayerPosition::new(zone.clone(), "1".into()))
    };
    let mut viewer = Body::new();
    viewer.set("이름", viewer_name.as_str());
    viewer.set("관리자등급", 1000_i64);
    viewer.set("금전", 31_i64);
    let output = ScriptStorage::default()
        .execute("소소", &mut viewer, "소소소환", None, None, None)
        .unwrap()
        .0;
    assert!(output.contains(&"\x1b[36m소환약초\x1b[37m".to_string()));
    assert!(output
        .iter()
        .any(|line| line.contains("은전 :                 2468 개")));
    assert!(output
        .iter()
        .any(|line| line.contains("금전 :                   31 개")));

    let mut world = get_world_state().write().unwrap();
    world.remove_summoned_user_by_id(id);
    world.remove_player_position(&viewer_name);
}

#[test]
fn admin_create_matches_python_permission_argument_counts_and_success_bytes() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "생성명령회귀관리자");
    body.set("관리자등급", 1999_i64);
    assert_eq!(
        storage
            .execute("생성", &mut body, "사강시", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );
    body.set("관리자등급", 2000_i64);
    assert_eq!(
        storage
            .execute("생성", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["사용법: [아이템 이름] [갯수] 생성"]
    );
    assert_eq!(
        storage
            .execute("생성", &mut body, "존재하지않는생성품", None, None, None)
            .unwrap()
            .0,
        vec!["* 생성 실패!!!"]
    );

    for (argument, added) in [("사강시 0", 0), ("사강시 -2", 0), ("사강시 +2 뒤는무시", 2)]
    {
        let before = body.object.objs.len();
        let result = storage
            .execute("생성", &mut body, argument, None, None, None)
            .unwrap();
        assert_eq!(
            result.0,
            vec!["\x1b[1;32m* [사강시] 생성 되었습니다.\x1b[0;37m"]
        );
        assert_eq!(body.object.objs.len(), before + added);
    }
    assert!(body
        .object
        .objs
        .iter()
        .all(|item| item.lock().is_ok_and(|item| {
            item.getString("종류") == "호위" && item.getInt("체력") == 1400
        })));

    let before = body.object.objs.len();
    let invalid = storage
        .execute("생성", &mut body, "사강시 1.5", None, None, None)
        .unwrap();
    assert!(invalid.0.is_empty(), "Python int() aborts before sendLine");
    assert_eq!(body.object.objs.len(), before);
}

#[test]
fn test_item_commands_create_drop_get_destroy() {
    use crate::player::Body;
    use crate::world::{get_world_state, PlayerPosition};

    let mut body = Body::new();
    body.set("이름", "item_test_player");
    body.set("관리자등급", 2000i64);
    body.set("힘", 1000_i64);

    // 플레이어 위치를 낙양성:1로 설정 (버리기/가져오기에 필요)
    {
        let mut w = get_world_state().write().unwrap();
        w.set_player_position(
            "item_test_player",
            PlayerPosition::new("낙양성".to_string(), "1".to_string()),
        );
    }

    let storage = ScriptStorage::default();
    if !storage.has_script("생성") {
        return; // cmds/생성.rhai가 없으면 스킵
    }

    // data/item/289.json 필요 (cargo test 시 cwd=프로젝트 루트)
    if !std::path::Path::new("data/item/289.json").exists() {
        return; // 데이터 없으면 스킵
    }

    // 1) 생성 289 (data/item/289.json = 철퇴)
    let res = storage.execute("생성", &mut body, "289", None, None, None);
    assert!(res.is_ok(), "생성 실패: {:?}", res.err());
    let (out, _) = res.as_ref().unwrap();
    assert_eq!(
        body.object.inv_stack.get("289"),
        Some(&1),
        "outputs: {out:?}"
    );
    assert!(body.object.objs.is_empty());

    // 2) 버리기 철퇴
    let res = storage.execute("버려", &mut body, "철퇴", None, None, None);
    assert!(res.is_ok(), "버리기 실패: {:?}", res.err());
    assert_eq!(body.get_item_count(), 0, "버린 후 인벤 비어있음");
    {
        let mut w = get_world_state().write().unwrap();
        let ro = w.get_room_objs_mut("낙양성", "1");
        assert_eq!(ro.len(), 1, "방 바닥에 1개");
        assert_eq!(ro[0].lock().unwrap().getName(), "철퇴");
    }

    // 3) 가져오기 철퇴
    let res = storage.execute("가져", &mut body, "철퇴", None, None, None);
    assert!(res.is_ok(), "가져오기 실패: {:?}", res.err());
    assert!(
        res.as_ref()
            .unwrap()
            .0
            .join("\r\n")
            .contains("철퇴\x1b[37m를\x1b[37m 집어서"),
        "가져 조사는 Python han_obj처럼 목적격이어야 함: {:?}",
        res.as_ref().unwrap().0
    );
    assert_eq!(body.object.inv_stack.get("289"), Some(&1));
    assert!(body.object.objs.is_empty());
    {
        let mut w = get_world_state().write().unwrap();
        let ro = w.get_room_objs_mut("낙양성", "1");
        assert_eq!(ro.len(), 0, "가져온 후 방 바닥 비어있음");
    }

    // 4) 소각 철퇴
    let res = storage.execute("소각", &mut body, "철퇴", None, None, None);
    assert!(res.is_ok(), "소각 실패: {:?}", res.err());
    assert_eq!(body.get_item_count(), 0, "소각 후 인벤 비어있음");

    // 5) 생성 → 부셔
    let _ = storage.execute("생성", &mut body, "289", None, None, None);
    assert_eq!(body.object.inv_stack.get("289"), Some(&1));
    let res = storage.execute("부셔", &mut body, "철퇴", None, None, None);
    assert!(res.is_ok(), "부셔 실패: {:?}", res.err());
    assert_eq!(body.get_item_count(), 0, "부신 후 인벤 비어있음");

    // 6) 모두 가져 / 모두 입어
    let _ = storage.execute("생성", &mut body, "289", None, None, None);
    let _ = storage.execute("생성", &mut body, "289", None, None, None);
    let _ = storage.execute("버려", &mut body, "모두", None, None, None);
    assert!(body.object.objs.is_empty());
    let picked = storage
        .execute("가져", &mut body, "모두", None, None, None)
        .unwrap();
    assert_eq!(body.object.inv_stack.get("289"), Some(&2));
    assert!(picked.0.join("\r\n").contains("철퇴\x1b[37m 2개를 집어서"));

    let equipped = storage
        .execute("입어", &mut body, "모두", None, None, None)
        .unwrap();
    assert_eq!(
        body.object
            .objs
            .iter()
            .filter(|item| item.lock().is_ok_and(|item| item.getBool("inUse")))
            .count(),
        1,
        "같은 계층 장비는 Python checkArmed에 따라 하나만 착용"
    );
    assert!(
        !equipped.0.is_empty(),
        "모두 입어는 착용한 각 장비의 Python 사용 문구를 출력해야 함"
    );
    let spaced_all = storage
        .execute("벗어", &mut body, " 모두 ", None, None, None)
        .unwrap();
    let unequip_output = spaced_all.0.join("\r\n");
    assert!(
        unequip_output.contains("당신이 \x1b[36m\x1b[0;36m철퇴\x1b[37m를\x1b[37m 착용해제 합니다.")
    );
    assert!(!unequip_output.contains("착용한 장비 1개를 해제했습니다."));
    assert_eq!(
        body.object
            .objs
            .iter()
            .filter(|item| item.lock().is_ok_and(|item| item.getBool("inUse")))
            .count(),
        0,
        "공백을 제거한 '모두'는 전체 해제로 처리해야 함"
    );
    assert!(body
        .object
        .objs
        .iter()
        .all(|item| item.lock().is_ok_and(|item| !item.getBool("inUse"))));

    let _ = storage.execute("입어", &mut body, "철퇴", None, None, None);
    let remembered = storage
        .execute("세트기억", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(remembered.0.join("\r\n"), "☞ 기억 되었습니다.");
    let remembered_id = body.get_string("세트기억");
    assert!(remembered_id.starts_with("SET-"));
    assert!(uuid::Uuid::parse_str(&remembered_id[4..]).is_ok());
    assert!(body
        .object
        .objs
        .iter()
        .any(|item| item
            .lock()
            .is_ok_and(|item| reaction_names(&item.getString("반응이름"))
                .iter()
                .any(|alias| alias == &remembered_id))));
    let _ = storage.execute("벗어", &mut body, "모두", None, None, None);
    let set_equipped = storage
        .execute("세트착용", &mut body, "", None, None, None)
        .unwrap();
    assert!(!set_equipped.0.is_empty());
    assert!(body
        .object
        .objs
        .iter()
        .any(|item| item.lock().is_ok_and(|item| item.getBool("inUse"))));
}

#[test]
fn remember_empty_equipment_still_replaces_python_set_uuid() {
    let mut body = Body::new();
    body.set("이름", "빈세트기억검사");
    body.set("세트기억", "SET-이전값");
    let output = ScriptStorage::default()
        .execute("세트기억", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(output.0, vec!["☞ 무엇을 기억하시려구요?."]);
    let saved = body.get_string("세트기억");
    assert_ne!(saved, "SET-이전값");
    assert!(saved.starts_with("SET-"));
    assert!(uuid::Uuid::parse_str(&saved[4..]).is_ok());
}

#[test]
fn remember_set_preserves_python_string_and_array_alias_elements_with_spaces() {
    let mut body = Body::new();
    let mut string_alias = Object::new();
    string_alias.set("이름", "문자열별칭장비");
    string_alias.set("종류", "무기");
    string_alias.set("반응이름", "오래된 검");
    string_alias.set("inUse", 1_i64);
    let string_alias = Arc::new(Mutex::new(string_alias));
    let mut array_alias = Object::new();
    array_alias.set("이름", "배열별칭장비");
    array_alias.set("종류", "방어구");
    array_alias.set("반응이름", "낡은 갑옷|무거운 갑옷|SET-이전값");
    array_alias.set("inUse", 1_i64);
    let array_alias = Arc::new(Mutex::new(array_alias));
    body.object.objs = vec![string_alias.clone(), array_alias.clone()];

    let output = ScriptStorage::default()
        .execute("세트기억", &mut body, "인자는 무시", None, None, None)
        .unwrap();
    assert_eq!(output.0, vec!["☞ 기억 되었습니다."]);
    let set_name = body.get_string("세트기억");
    assert_eq!(
        string_alias.lock().unwrap().getString("반응이름"),
        format!("오래된 검\r\n{set_name}")
    );
    assert_eq!(
        array_alias.lock().unwrap().getString("반응이름"),
        format!("낡은 갑옷\r\n무거운 갑옷\r\n{set_name}")
    );
}

#[test]
fn set_wear_equips_the_tagged_same_name_instance_by_python_inventory_order() {
    use crate::script::party::set_precomputed_party_context;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let actor = format!("세트순번검사-{suffix}");
    let observer = format!("세트착용관찰-{suffix}");
    let zone = format!("세트착용존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&actor, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&observer, PlayerPosition::new(zone, "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", actor.as_str());
    body.set("세트기억", "SET-선택");
    let mut ordinary = Object::new();
    ordinary.set("이름", "쌍검");
    ordinary.set("반응이름", "쌍검");
    ordinary.set("종류", "무기");
    ordinary.set("계층", "무기");
    let ordinary = Arc::new(Mutex::new(ordinary));
    let mut tagged = Object::new();
    tagged.set("이름", "쌍검");
    tagged.set("반응이름", "쌍검\r\nSET-선택");
    tagged.set("종류", "무기");
    tagged.set("계층", "무기");
    tagged.set("안시", "\x1b[35m");
    let tagged = Arc::new(Mutex::new(tagged));
    body.object.objs.push(ordinary.clone());
    body.object.objs.push(tagged.clone());
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(observer.clone()));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("hp".into(), Dynamic::from(19_i64));
    person.insert("max_hp".into(), Dynamic::from(29_i64));
    person.insert("mp".into(), Dynamic::from(2_i64));
    person.insert("max_mp".into(), Dynamic::from(3_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);

    let worn = ScriptStorage::default()
        .execute("세트착용", &mut body, "", None, None, None)
        .unwrap();
    assert!(!ordinary.lock().unwrap().getBool("inUse"));
    assert!(tagged.lock().unwrap().getBool("inUse"));
    assert_eq!(
        worn.0,
        vec!["당신이 \x1b[36m\x1b[35m쌍검\x1b[0;37m을\x1b[37m 착용합니다."]
    );
    let sends = match worn.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected set-wear result: {other:?}"),
    };
    assert_eq!(
        sends,
        vec![(
            observer.clone(),
            format!(
                "{}\r\n\x1b[1m{actor}\x1b[0;37m{} \x1b[36m\x1b[35m쌍검\x1b[0;37m을\x1b[37m 착용합니다.\r\n\r\n\x1b[0;37;40m[ 19/29, 2/3 ] ",
                RAW_USER_MESSAGE_PREFIX,
                han_iga(&actor)
            )
        )]
    );
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&actor);
    world.remove_player_position(&observer);
    set_precomputed_party_context(rhai::Map::new());
}

#[test]
fn burn_and_break_use_actual_item_text_notify_room_and_persist_removal() {
    let _oneitem_guard = ONEITEM_COMMAND_TEST_LOCK.lock().unwrap();
    use crate::command::handler::CommandResult;
    use crate::object::Object;
    use crate::script::party::set_precomputed_party_context;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let self_name = format!("파괴회귀-{suffix}");
    let observer = format!("파괴관찰-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            &self_name,
            PlayerPosition::new("파괴회귀존".into(), suffix.to_string()),
        );
        world.set_player_position(
            &observer,
            PlayerPosition::new("파괴회귀존".into(), suffix.to_string()),
        );
    }
    let mut body = Body::new();
    body.set("이름", self_name.as_str());
    let mut fruit = Object::new();
    fruit.set("이름", "설삼과");
    fruit.set("반응이름", "설삼과\r\n과일");
    body.object.append(Arc::new(Mutex::new(fruit)));
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(observer.clone()));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("hp".into(), Dynamic::from(14_i64));
    person.insert("max_hp".into(), Dynamic::from(24_i64));
    person.insert("mp".into(), Dynamic::from(5_i64));
    person.insert("max_mp".into(), Dynamic::from(6_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);
    let storage = ScriptStorage::default();

    let (output, special) = storage
        .execute("소각", &mut body, "과일", None, None, None)
        .unwrap();
    assert_eq!(
        output,
        vec!["당신이 \x1b[36m\x1b[0;36m설삼과\x1b[37m를\x1b[37m 소각해버립니다."]
    );
    assert!(body.object.objs.is_empty());
    assert!(matches!(
        special,
        Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
            if own == "당신이 \x1b[36m\x1b[0;36m설삼과\x1b[37m를\x1b[37m 소각해버립니다."
                && sends == &vec![(
                    observer.clone(),
                    format!("{}\r\n\x1b[1m{self_name}\x1b[0;37m{} \x1b[36m\x1b[0;36m설삼과\x1b[37m를\x1b[37m 소각해버립니다.\r\n\r\n\x1b[0;37;40m[ 14/24, 5/6 ] ", RAW_USER_MESSAGE_PREFIX, han_iga(&self_name))
                )]
    ));

    let mut unbreakable = Object::new();
    unbreakable.set("이름", "금강석");
    unbreakable.set("반응이름", "돌");
    unbreakable.set("아이템속성", "부수지못함");
    body.object.append(Arc::new(Mutex::new(unbreakable)));
    let blocked = storage
        .execute("부셔", &mut body, "돌", None, None, None)
        .unwrap();
    assert_eq!(blocked.0, vec!["☞ 부셔지지 않네요. ^^"]);
    assert_eq!(body.object.objs.len(), 1);

    body.object.objs.clear();
    for _ in 0..2 {
        let mut pottery = Object::new();
        pottery.set("이름", "도자기");
        pottery.set("반응이름", "그릇");
        body.object.append(Arc::new(Mutex::new(pottery)));
    }
    let broken = storage
        .execute("부셔", &mut body, "그릇 2", None, None, None)
        .unwrap();
    assert_eq!(
        broken.0,
        vec!["당신이 \x1b[36m도자기\x1b[37m 2개를 부셔버립니다."]
    );
    assert!(matches!(
        broken.1,
        Some(CommandResult::OutputAndSendToUsers(_, ref sends))
            if sends == &vec![(
                observer.clone(),
                format!("{}\r\n\x1b[1m{self_name}\x1b[0;37m{} \x1b[36m도자기\x1b[37m 2개를 부셔버립니다.\r\n\r\n\x1b[0;37;40m[ 14/24, 5/6 ] ", RAW_USER_MESSAGE_PREFIX, han_iga(&self_name))
            )]
    ));
    assert!(body.object.objs.is_empty());

    let mut colored = Object::new();
    colored.set("이름", "옥");
    colored.set("반응이름", "옥");
    colored.set("안시", "\x1b[35m");
    body.object.append(Arc::new(Mutex::new(colored)));
    let single = storage
        .execute("부셔", &mut body, "옥 1개", None, None, None)
        .unwrap();
    assert_eq!(
        single.0,
        vec!["당신이 \x1b[36m\x1b[35m옥\x1b[0;37m을\x1b[37m 부셔버립니다."]
    );
    assert!(body.object.objs.is_empty());

    let unique_index = format!("파괴단일-{suffix}");
    let mut unique = Object::new();
    unique.set("이름", "단일옥패");
    unique.set("반응이름", "옥패");
    unique.set("인덱스", unique_index.as_str());
    unique.set("아이템속성", "단일아이템");
    body.object.append(Arc::new(Mutex::new(unique)));
    assert!(crate::oneitem::oneitem_have(&unique_index, &self_name));
    let _ = storage
        .execute("소각", &mut body, "옥패", None, None, None)
        .unwrap();
    assert_eq!(crate::oneitem::oneitem_get(&unique_index), "");

    body.object.inv_stack.insert("1037".into(), 1);
    let stacked = storage
        .execute("소각", &mut body, "탕수육", None, None, None)
        .unwrap();
    assert_eq!(
        stacked.0[0],
        "당신이 \x1b[36m\x1b[0;36m탕수육\x1b[37m을\x1b[37m 소각해버립니다."
    );
    assert!(body.object.inv_stack.is_empty());

    body.object.inv_stack.insert("1037".into(), 1);
    let stacked_break = storage
        .execute("부셔", &mut body, "탕수육", None, None, None)
        .unwrap();
    assert_eq!(
        stacked_break.0[0],
        "당신이 \x1b[36m\x1b[0;36m탕수육\x1b[37m을\x1b[37m 부셔버립니다."
    );
    assert!(body.object.inv_stack.is_empty());
    assert!(body.object.objs.is_empty());

    body.object.inv_stack.insert("토령시".into(), 1);
    let protected_stack = storage
        .execute("부셔", &mut body, "토령시", None, None, None)
        .unwrap();
    assert_eq!(protected_stack.0, vec!["☞ 부셔지지 않네요. ^^"]);
    assert_eq!(body.object.inv_stack.get("토령시"), Some(&1));
    assert!(body.object.objs.is_empty());

    let (changed_food, _) = object_from_item_json("1037").unwrap();
    changed_food.lock().unwrap().set("판매가격", 99_i64);
    body.object.objs.push(changed_food);
    body.object.inv_stack.insert("1037".into(), 2);
    let mixed_destroy = storage
        .execute("소각", &mut body, "탕수육 3", None, None, None)
        .unwrap();
    assert_eq!(
        mixed_destroy.0,
        vec!["당신이 \x1b[36m탕수육\x1b[37m 3개를 소각해버립니다."]
    );
    assert!(!body.object.inv_stack.contains_key("1037"));
    assert!(body.object.objs.is_empty());

    let _ = std::fs::remove_file(format!("data/user/{self_name}.json"));
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&self_name);
    world.remove_player_position(&observer);
    set_precomputed_party_context(rhai::Map::new());
}

#[test]
fn decompose_uses_first_python_merchant_and_preserves_item_ansi_and_shard_bug() {
    use crate::command::handler::CommandResult;
    use crate::script::party::set_precomputed_party_context;
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player = format!("분해회귀-{suffix}");
    let observer = format!("분해관찰자-{suffix}");
    let zone = format!("분해회귀존-{suffix}");
    let seller_key = format!("{zone}:판매만");
    let buyer_key = format!("{zone}:매입상인");
    let mut seller_data = RawMobData::new();
    seller_data
        .attributes
        .insert("물건판매".into(), serde_json::json!(["물품"]));
    let seller = MobInstance::new(seller_key.clone(), zone.clone(), "1", &seller_data);
    let seller_id = seller.instance_id;
    let mut buyer_data = RawMobData::new();
    buyer_data
        .attributes
        .insert("물건구입".into(), serde_json::json!("고물상 40"));
    let buyer = MobInstance::new(buyer_key.clone(), zone.clone(), "1", &buyer_data);
    let buyer_id = buyer.instance_id;
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(seller_key.clone(), seller_data);
        world
            .mob_cache
            .insert_mob_data(buyer_key.clone(), buyer_data);
        world.mob_cache.add_mob_instance(seller);
        world.mob_cache.add_mob_instance(buyer);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(buyer_id));
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(seller_id));
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&observer, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    let mut weapon = Object::new();
    weapon.set("이름", "자빛검");
    weapon.set("종류", "무기");
    weapon.set("안시", "\x1b[35m");
    weapon.set_option(&HashMap::from([
        ("힘".into(), 1),
        ("맷집".into(), 1),
        ("민첩성".into(), 1),
        ("운".into(), 1),
    ]));
    body.object.objs.push(Arc::new(Mutex::new(weapon)));
    let mut mastery_weapon = Object::new();
    mastery_weapon.set("이름", "올숙보존검");
    mastery_weapon.set("인덱스", "올숙무기시험");
    mastery_weapon.set("종류", "무기");
    mastery_weapon.set("옵션", "힘 1");
    body.object.objs.push(Arc::new(Mutex::new(mastery_weapon)));
    let unique_index = format!("분해단일-{suffix}");
    let mut malformed = Object::new();
    malformed.set("이름", "깨진옵션검");
    malformed.set("인덱스", unique_index.as_str());
    malformed.set("종류", "무기");
    malformed.set("옵션", "깨진옵션");
    malformed.set("아이템속성", "단일아이템");
    body.object.objs.push(Arc::new(Mutex::new(malformed)));
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(observer.clone()));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("hp".into(), Dynamic::from(61_i64));
    person.insert("max_hp".into(), Dynamic::from(71_i64));
    person.insert("mp".into(), Dynamic::from(8_i64));
    person.insert("max_mp".into(), Dynamic::from(9_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);
    assert!(crate::oneitem::oneitem_have(&unique_index, &player));
    let storage = ScriptStorage::default();
    let blocked = storage
        .execute("분해", &mut body, "모두", None, None, None)
        .unwrap();
    assert_eq!(blocked.0, vec!["☞ 상인이 없어요. ^_^"]);
    assert_eq!(body.object.objs.len(), 3);

    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.remove_instance(&zone, "1", &seller_key);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(buyer_id));
    }
    let decomposed = storage
        .execute("분해", &mut body, "모두", None, None, None)
        .unwrap();
    assert_eq!(
        decomposed.0,
        vec![
            "당신이 \x1b[35m자빛검\x1b[0;37m 1개를 분해합니다.",
            "당신이 \x1b[0;36m깨진옵션검\x1b[37m 1개를 분해합니다.",
        ]
    );
    let sends = match decomposed.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected decompose result: {other:?}"),
    };
    assert_eq!(sends.len(), 2);
    assert_eq!(sends[0].0, observer);
    assert_eq!(
        sends[0].1,
        format!(
            "{}\r\n\x1b[1m{player}\x1b[0;37m{} \x1b[35m자빛검\x1b[0;37m 1개를 분해합니다.\r\n\r\n\x1b[0;37;40m[ 61/71, 8/9 ] ",
            RAW_USER_MESSAGE_PREFIX,
            han_iga(&player)
        )
    );
    assert_eq!(body.object.inv_stack.get("강철조각"), Some(&3));
    assert!(body
        .object
        .objs
        .iter()
        .any(|item| item.lock().is_ok_and(|item| item.getName() == "올숙보존검")));
    assert_eq!(crate::oneitem::oneitem_get(&unique_index), "");

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.remove_player_position(&observer);
    world.mob_cache.remove_mob(&seller_key);
    world.mob_cache.remove_mob(&buyer_key);
    set_precomputed_party_context(rhai::Map::new());
    let _ = std::fs::remove_file(format!("data/user/{player}.json"));
}

#[test]
fn give_commands_preserve_python_lookup_self_and_admin_grant_requests() {
    use crate::command::handler::CommandResult;
    use crate::object::Object;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let giver = format!("전달자-{suffix}");
    let target = format!("수령자-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            &giver,
            PlayerPosition::new("전달회귀존".into(), suffix.to_string()),
        );
        world.set_player_position(
            &target,
            PlayerPosition::new("전달회귀존".into(), suffix.to_string()),
        );
    }
    let mut body = Body::new();
    body.set("이름", giver.as_str());
    body.set("관리자등급", 2000_i64);
    body.set("은전", 10_i64);
    let mut item = Object::new();
    item.set("이름", "청옥패");
    item.set("반응이름", "옥패");
    item.set("아이템속성", "줄수없음");
    body.object.append(Arc::new(Mutex::new(item)));
    let mut hidden = Object::new();
    hidden.set("이름", "숨은패");
    hidden.set("반응이름", "숨패");
    hidden.set("아이템속성", "출력안함");
    body.object.append(Arc::new(Mutex::new(hidden)));
    let mut target_view = Body::new();
    target_view.set("이름", target.as_str());
    target_view.set("반응이름", "전달대상별칭");
    set_precomputed_room_view_players(HashMap::from([(
        format!("전달회귀존:{suffix}"),
        vec![build_room_view_player_snapshot(&target_view)],
    )]));
    let storage = ScriptStorage::default();

    let missing_item_first = storage
        .execute("줘", &mut body, "없는대상 없는물건", None, None, None)
        .unwrap();
    assert_eq!(
        missing_item_first.0,
        vec!["☞ 그런 아이템이 소지품에 없어요."]
    );

    let self_play = storage
        .execute("줘", &mut body, &format!("{giver} 옥패"), None, None, None)
        .unwrap();
    assert_eq!(
        self_play.0,
        vec!["당신이 \x1b[36m청옥패를\x1b[37m 가지고 장난합니다. '@_@'"]
    );
    assert!(self_play.1.is_none());

    let self_money = storage
        .execute(
            "줘",
            &mut body,
            &format!("{giver} 은전 3"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(body.get_int("은전"), 10);
    assert_eq!(
        self_money.0,
        vec![
            format!("당신이 \x1b[1m{giver}\x1b[0;37m에게 은전 3개를 줍니다."),
            format!(
                "\r\n\x1b[1m{giver}\x1b[0;37m{} 당신에게 은전 3개를 줍니다.",
                han_iga(&giver)
            ),
        ]
    );

    let python_substring_alias = storage
        .execute("줘", &mut body, &format!("{giver} 옥"), None, None, None)
        .unwrap();
    assert_eq!(
        python_substring_alias.0,
        vec!["당신이 \x1b[36m청옥패를\x1b[37m 가지고 장난합니다. '@_@'"]
    );

    let hidden_item = storage
        .execute("줘", &mut body, &format!("{giver} 숨패"), None, None, None)
        .unwrap();
    assert_eq!(hidden_item.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);

    let mut collision = Object::new();
    collision.set("이름", "전달충돌패");
    collision.set("반응이름", "전달대상별칭");
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world
            .get_room_objs_mut("전달회귀존", &suffix.to_string())
            .push(collision.clone());
        world.record_floor_item("전달회귀존", &suffix.to_string(), &collision);
    }
    let item_first = storage
        .execute("줘", &mut body, "전달대상별칭 은전 1", None, None, None)
        .unwrap();
    assert_eq!(
        item_first.0,
        vec!["☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^"]
    );
    {
        let room = suffix.to_string();
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record("전달회귀존", &room, &collision);
        world
            .get_room_objs_mut("전달회귀존", &room)
            .retain(|item| !Arc::ptr_eq(item, &collision));
    }

    let normal_money = storage
        .execute("줘", &mut body, "전달대상별칭 은전 7", None, None, None)
        .unwrap();
    assert!(matches!(
        normal_money.1,
        Some(CommandResult::GiveToPlayer {
            give_silver: Some(7),
            deduct_from_giver: true,
            bypass_item_limits: false,
            ..
        })
    ));

    body.set("은전", 2_000_i64);
    let large_money = storage
        .execute("줘", &mut body, "전달대상별칭 은전 1000", None, None, None)
        .unwrap();
    assert!(matches!(
        large_money.1,
        Some(CommandResult::GiveToPlayer {
            give_silver: Some(1_000),
            deduct_from_giver: true,
            ..
        })
    ));
    body.set("은전", 10_i64);

    let admin_money = storage
        .execute("줘줘", &mut body, "전달대상별칭 은전 25", None, None, None)
        .unwrap();
    assert!(matches!(
        admin_money.1,
        Some(CommandResult::GiveToPlayer {
            give_silver: Some(25),
            deduct_from_giver: false,
            bypass_item_limits: false,
            ..
        })
    ));
    assert_eq!(
        body.get_int("은전"),
        10,
        "요청 단계에서 관리자 은전을 차감하지 않음"
    );

    let admin_self_money = storage
        .execute(
            "줘줘",
            &mut body,
            &format!("{giver} 은전 4"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(body.get_int("은전"), 14);
    assert_eq!(
        admin_self_money.0,
        vec![
            format!("당신이 \x1b[1m{giver}\x1b[0;37m에게 은전 4개를 줍니다."),
            format!(
                "\r\n\x1b[1m{giver}\x1b[0;37m{} 당신에게 은전 4개를 줍니다.",
                han_iga(&giver)
            ),
        ]
    );

    let admin_item = storage
        .execute("줘줘", &mut body, "전달대상별칭 옥패 100", None, None, None)
        .unwrap();
    assert!(matches!(
        admin_item.1,
        Some(CommandResult::GiveToPlayer {
            give_item: Some((ref name, 1, 100)),
            bypass_item_limits: true,
            ..
        }) if name == "청옥패"
    ));

    let (modified_throwing, _) = object_from_item_json("비황석").unwrap();
    modified_throwing.lock().unwrap().set("급습위력", 81_i64);
    body.object.objs.push(modified_throwing);
    body.object.inv_stack.insert("비황석".into(), 1);
    let numbered_counted_item = storage
        .execute("줘", &mut body, "전달대상별칭 2비황석", None, None, None)
        .unwrap();
    assert!(matches!(
        numbered_counted_item.1,
        Some(CommandResult::GiveToPlayer {
            give_item: None,
            give_item_stack: Some((ref key, 1)),
            bypass_item_limits: false,
            ..
        }) if key == "비황석"
    ));

    let admin_numbered_counted_item = storage
        .execute("줘줘", &mut body, "전달대상별칭 2비황석", None, None, None)
        .unwrap();
    assert!(matches!(
        admin_numbered_counted_item.1,
        Some(CommandResult::GiveToPlayer {
            give_item: None,
            give_item_stack: Some((ref key, 1)),
            bypass_item_limits: true,
            ..
        }) if key == "비황석"
    ));

    let mixed_bulk = storage
        .execute("줘", &mut body, "전달대상별칭 비황석 2", None, None, None)
        .unwrap();
    assert!(matches!(
        mixed_bulk.1,
        Some(CommandResult::GiveToPlayer {
            give_item: Some((ref name, 1, 1)),
            give_item_stack: Some((ref key, 1)),
            bypass_item_limits: false,
            ..
        }) if name == "비황석" && key == "비황석"
    ));

    let admin_mixed_bulk = storage
        .execute("줘줘", &mut body, "전달대상별칭 비황석 2", None, None, None)
        .unwrap();
    assert!(matches!(
        admin_mixed_bulk.1,
        Some(CommandResult::GiveToPlayer {
            give_item: Some((ref name, 1, 1)),
            give_item_stack: Some((ref key, 1)),
            bypass_item_limits: true,
            ..
        }) if name == "비황석" && key == "비황석"
    ));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&giver);
    world.remove_player_position(&target);
    drop(world);
    clear_precomputed_room_view_players();
}

#[test]
fn admin_give_silver_preserves_python_negative_amount() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("음수지급관리자-{suffix}");
    let target = format!("음수지급대상-{suffix}");
    let zone = format!("음수지급존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut player = rhai::Map::new();
    player.insert("이름".into(), Dynamic::from(target.clone()));
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![Dynamic::from(player)],
    )]));
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 2000_i64);

    let result = ScriptStorage::default()
        .execute(
            "줘줘",
            &mut body,
            &format!("{target} 은전 -7"),
            None,
            None,
            None,
        )
        .unwrap();

    assert!(matches!(
        result.1,
        Some(CommandResult::GiveToPlayer {
            give_silver: Some(-7),
            ..
        })
    ));
    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin);
    world.remove_player_position(&target);
}

#[test]
fn install_command_creates_reloadable_box_and_python_success_text() {
    use crate::command::handler::CommandResult;
    use crate::object::Object;
    use crate::script::party::set_precomputed_party_context;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player_name = format!("설치회귀-{suffix}");
    let observer = format!("설치목격자-{suffix}");
    let zone = format!("설치회귀존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    let room_path = room_dir.join("1.json");
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::write(
        &room_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "맵정보": {
                "이름": "설치 시험방", "존이름": zone,
                "주인": player_name, "설치리스트": [],
                "설명": [], "출구": []
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    let mut item = Object::new();
    item.set("이름", "시험보관함");
    item.set("반응이름", "시험보관함\r\n보관함");
    item.set("종류", "설치아이템");
    item.set("보관수량", 10_i64);
    item.set("보관최대수량", 20_i64);
    item.set("보관증가은전", 100_i64);
    body.object.append(Arc::new(Mutex::new(item)));
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.set_player_position(
            &player_name,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
        world.set_player_position(
            &observer,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
    }
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(observer.clone()));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("hp".into(), Dynamic::from(41_i64));
    person.insert("max_hp".into(), Dynamic::from(52_i64));
    person.insert("mp".into(), Dynamic::from(6_i64));
    person.insert("max_mp".into(), Dynamic::from(7_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);

    let storage = ScriptStorage::default();
    let whitespace = storage
        .execute("설치", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법: [대상] 설치"]);
    let installed = storage
        .execute("설치", &mut body, "시험보관함", None, None, None)
        .unwrap();
    assert_eq!(
        installed.0.join("\r\n"),
        "당신이 \x1b[0;36m시험보관함\x1b[37m을 설치합니다."
    );
    let sends = match installed.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected install delivery: {other:?}"),
    };
    assert_eq!(sends.len(), 1);
    assert_eq!(sends[0].0, observer);
    assert_eq!(
        sends[0].1,
        format!(
            "{}\r\n\x1b[1m{player_name}\x1b[0;37m가 \x1b[0;36m시험보관함\x1b[37m을 설치합니다.\r\n\r\n\x1b[0;37;40m[ 41/52, 6/7 ] ",
            RAW_USER_MESSAGE_PREFIX,
        )
    );
    assert!(body.object.objs.is_empty());
    let box_path = std::path::Path::new("data/box").join(format!("{player_name}_시험보관함.json"));
    assert!(
        box_path.exists(),
        "설치 상자는 loader가 읽는 단일 .json 경로에 저장"
    );
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&box_path).unwrap()).unwrap();
    assert_eq!(saved["상자정보"]["이름"], "시험보관함");

    let _ = std::fs::remove_file(box_path);
    let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
    let _ = std::fs::remove_dir_all(room_dir);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player_name);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&observer);
    set_precomputed_party_context(rhai::Map::new());
}

#[test]
fn install_command_matches_python_exact_alias_guild_permission_and_string_list() {
    use crate::object::Object;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("방파설치회귀-{suffix}");
    let guild = format!("설치방파-{suffix}");
    let zone = format!("방파설치존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    let room_path = room_dir.join("1.json");
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::write(
        &room_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "맵정보": {
                "이름": "방파 설치방", "존이름": zone,
                "주인": "", "방파주인": guild,
                "설치리스트": "기존설치물", "설명": [], "출구": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));

    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("소속", guild.as_str());
    let mut item = Object::new();
    item.set("이름", "공용시험보관함");
    item.set("반응이름", "공용시험보관함\r\n공용함");
    item.set("종류", "설치아이템");
    item.set("보관수량", 10_i64);
    item.set("보관최대수량", 20_i64);
    body.object.append(Arc::new(Mutex::new(item)));
    let storage = ScriptStorage::default();

    let partial = storage
        .execute("설치", &mut body, "공용", None, None, None)
        .unwrap();
    assert_eq!(partial.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);
    let denied = storage
        .execute("설치", &mut body, "공용함", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 이곳에 설치할 허가권이 없습니다."]);
    assert_eq!(body.object.objs.len(), 1);

    body.object.objs[0]
        .lock()
        .unwrap()
        .set("아이템속성", "공용보관함");
    let installed = storage
        .execute("설치", &mut body, "공용함", None, None, None)
        .unwrap();
    assert_eq!(
        installed.0,
        vec!["당신이 \x1b[0;36m공용시험보관함\x1b[37m을 설치합니다."]
    );
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
    assert_eq!(
        saved["맵정보"]["설치리스트"],
        serde_json::json!(["기존설치물", "공용시험보관함"])
    );

    let box_path = std::path::Path::new("data/box").join(format!("{guild}_공용시험보관함.json"));
    let _ = std::fs::remove_file(box_path);
    let _ = std::fs::remove_file(format!("data/user/{player}.json"));
    let _ = std::fs::remove_dir_all(room_dir);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
}

#[test]
fn borrow_and_return_commands_match_python_branch_messages_and_state() {
    use crate::object::Object;
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player_name = format!("대여회귀-{suffix}");
    let zone = format!("대여회귀존-{suffix}");
    let catalog_path = std::env::temp_dir().join(format!("muc-book-command-{suffix}.json"));
    crate::book::save(
        &catalog_path,
        &[serde_json::json!({
            "이름": "철퇴",
            "고유번호": "command-book-id",
            "등록자": "등록자",
            "대여가능": true,
            "대여": "",
            "인덱스": "289",
            "attr": {
                "이름": "철퇴",
                "반응이름": "철퇴",
                "계층": "무기",
                "종류": "무기"
            }
        })],
    )
    .unwrap();

    let mob_key = format!("{zone}:진영");
    let mut mob_data = RawMobData::new();
    mob_data.name = "등록관리인".to_string();
    mob_data.zone = zone.clone();
    mob_data.reaction_names = vec!["진영담당자".to_string()];
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), mob_data.clone());
        world
            .mob_cache
            .add_mob_instance(MobInstance::new(mob_key, zone.clone(), "1", &mob_data));
        world.set_player_position(
            &player_name,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
    }

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set(
        "__시험도서목록경로",
        catalog_path.to_string_lossy().as_ref(),
    );

    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .get_all_mobs_in_room_mut(&zone, "1")
            .unwrap()[0]
            .runtime_attrs
            .insert("투명상태".into(), Value::Int(1));
    }
    let hidden_agent = storage
        .execute("대여", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(hidden_agent.0, vec!["☞ 이곳에서는 불가능해요."]);

    // Python only checks whether Room.findObjName("진영") returned an
    // object; despite the local variable name, it never verifies that the
    // object is a mob. A visible matching floor item therefore enables all
    // book commands even while the actual attendant is invisible.
    let mut sign = Object::new();
    sign.set("이름", "진영표지석");
    sign.set("반응이름", "진영표지석\r\n진영");
    let sign = Arc::new(Mutex::new(sign));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(sign.clone());
        world.record_floor_item(&zone, "1", &sign);
    }
    let item_gate = storage
        .execute("대여목록", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(item_gate.0.len(), 1);
    assert!(item_gate.0[0].contains("철퇴"), "{:?}", item_gate.0);
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, "1", &sign);
        world
            .get_room_objs_mut(&zone, "1")
            .retain(|item| !Arc::ptr_eq(item, &sign));
    }
    get_world_state()
        .write()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room_mut(&zone, "1")
        .unwrap()[0]
        .runtime_attrs
        .insert("투명상태".into(), Value::Int(0));

    let usage = storage
        .execute("대여", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [물품번호] 대여"]);
    let whitespace_borrow = storage
        .execute("대여", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_borrow.0, vec!["☞ 사용법: [물품번호] 대여"]);
    let whitespace_return = storage
        .execute("반납", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_return.0, vec!["☞ 사용법: [물품] 반납"]);
    let invalid = storage
        .execute("대여", &mut body, "0", None, None, None)
        .unwrap();
    assert_eq!(invalid.0, vec!["☞ 대여 가능한 물품이 없습니다."]);
    let mut malformed_entries = crate::book::load(&catalog_path).unwrap();
    malformed_entries.push(serde_json::json!({
        "이름": "손상대여품", "고유번호": "broken-book-id", "등록자": "등록자",
        "대여가능": true, "대여": "", "인덱스": "289"
    }));
    crate::book::save(&catalog_path, &malformed_entries).unwrap();
    let malformed = storage
        .execute("대여", &mut body, "2", None, None, None)
        .unwrap();
    assert_eq!(malformed.0, vec!["☞ 대여 가능한 물품이 없습니다."]);
    assert!(body.object.objs.is_empty());
    let entries = crate::book::load(&catalog_path).unwrap();
    assert!(crate::book::dict_get_bool(&entries[1], "대여가능"));
    assert_eq!(crate::book::dict_get_string(&entries[1], "대여"), "");
    crate::book::save(&catalog_path, &entries[..1]).unwrap();

    // Python getInt accepts a decimal prefix even when text follows it.
    let borrowed = storage
        .execute("대여", &mut body, "1번", None, None, None)
        .unwrap();
    assert_eq!(borrowed.0, vec!["☞ 대여가 완료 되었습니다."]);
    assert_eq!(body.object.objs.len(), 1);
    assert_eq!(
        body.object.objs[0].lock().unwrap().getString("고유번호"),
        "command-book-id"
    );
    {
        let borrowed_item = body.object.objs[0].lock().unwrap();
        // Python assigns `item.attr = itm["attr"]` after deepclone:
        // catalogue attributes replace template attributes rather than
        // being merged into them. Rust retains only its separate index
        // bookkeeping key in addition to that map.
        assert_eq!(borrowed_item.getName(), "철퇴");
        assert_eq!(borrowed_item.getString("반응이름"), "철퇴");
        assert_eq!(borrowed_item.getString("계층"), "무기");
        assert_eq!(borrowed_item.getString("종류"), "무기");
        assert_eq!(borrowed_item.getString("인덱스"), "289");
        assert_eq!(borrowed_item.getString("설명"), "");
    }
    let entry = crate::book::load(&catalog_path).unwrap().remove(0);
    assert!(!crate::book::dict_get_bool(&entry, "대여가능"));
    assert_eq!(crate::book::dict_get_string(&entry, "대여"), player_name);

    let inventory_len = body.object.objs.len();
    let already_borrowed = storage
        .execute("대여", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(already_borrowed.0, vec!["☞ 현재 대여중 입니다."]);
    assert_eq!(body.object.objs.len(), inventory_len);

    let returned = storage
        .execute("반납", &mut body, "철퇴", None, None, None)
        .unwrap();
    assert_eq!(returned.0, vec!["☞ 반납이 완료 되었습니다."]);
    assert!(body.object.objs.is_empty());
    let entry = crate::book::load(&catalog_path).unwrap().remove(0);
    assert!(crate::book::dict_get_bool(&entry, "대여가능"));
    assert_eq!(crate::book::dict_get_string(&entry, "대여"), "");

    crate::book::mark_borrowed(&catalog_path, 1, &player_name).unwrap();
    let make_return_candidate = |in_use: bool, unique: &str| {
        let mut item = Object::new();
        item.set("이름", "반납후보");
        inventory_compat::set_item_json_field(
            &mut item,
            "반응이름",
            &serde_json::json!(["반납별칭", "다른별칭"]),
        );
        item.set("고유번호", unique);
        item.set("inUse", i64::from(in_use));
        Arc::new(Mutex::new(item))
    };
    let equipped_candidate = make_return_candidate(true, "equipped-id");
    let first_candidate = make_return_candidate(false, "wrong-id");
    let second_candidate = make_return_candidate(false, "command-book-id");
    body.object.objs = vec![
        equipped_candidate.clone(),
        first_candidate.clone(),
        second_candidate.clone(),
    ];
    let numbered_return = storage
        .execute("반납", &mut body, "2반납별칭", None, None, None)
        .unwrap();
    assert_eq!(numbered_return.0, vec!["☞ 반납이 완료 되었습니다."]);
    assert_eq!(body.object.objs.len(), 2);
    assert!(body
        .object
        .objs
        .iter()
        .any(|item| Arc::ptr_eq(item, &equipped_candidate)));
    assert!(body
        .object
        .objs
        .iter()
        .any(|item| Arc::ptr_eq(item, &first_candidate)));
    assert!(crate::book::dict_get_bool(
        &crate::book::load(&catalog_path).unwrap()[0],
        "대여가능"
    ));

    let mut ordinary = Object::new();
    ordinary.set("이름", "평범한물품");
    body.object.objs.clear();
    body.object.append(Arc::new(Mutex::new(ordinary)));
    let not_returnable = storage
        .execute("반납", &mut body, "평범한물품", None, None, None)
        .unwrap();
    assert_eq!(not_returnable.0, vec!["☞ 반납 가능한 물품이 아닙니다."]);

    body.object.objs.clear();
    let mut equipped = Object::new();
    equipped.set("이름", "등록시험철퇴");
    equipped.set("반응이름", "등록시험철퇴\r\n시험철퇴");
    equipped.set("인덱스", "289");
    equipped.set("종류", "무기");
    equipped.set("계층", "무기");
    equipped.set("inUse", 1_i64);
    body.object.append(Arc::new(Mutex::new(equipped)));
    let mut weapon = Object::new();
    weapon.set("이름", "등록시험철퇴");
    weapon.set("반응이름", "등록시험철퇴\r\n시험철퇴");
    weapon.set("인덱스", "289");
    weapon.set("종류", "무기");
    weapon.set("계층", "무기");
    inventory_compat::set_item_json_field(
        &mut weapon,
        "아이템속성",
        &serde_json::json!(["줄수없음확장"]),
    );
    body.object.append(Arc::new(Mutex::new(weapon)));
    let mut prefix_only = Object::new();
    prefix_only.set("이름", "접두전용등록품");
    prefix_only.set("인덱스", "289");
    prefix_only.set("종류", "무기");
    inventory_compat::set_item_json_field(
        &mut prefix_only,
        "반응이름",
        &serde_json::json!(["등록시험철퇴확장"]),
    );
    let prefix_only = Arc::new(Mutex::new(prefix_only));
    body.object.objs.insert(1, prefix_only.clone());
    let whitespace_register = storage
        .execute("등록", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_register.0, vec!["☞ 사용법: [물품] 등록"]);

    let missing_register = storage
        .execute("등록", &mut body, "없는등록품", None, None, None)
        .unwrap();
    assert_eq!(missing_register.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);

    let register_candidate = body.object.objs[2].clone();
    register_candidate.lock().unwrap().set("종류", "방어구");
    let non_weapon = storage
        .execute("등록", &mut body, "등록시험철퇴", None, None, None)
        .unwrap();
    assert_eq!(non_weapon.0, vec!["☞ 등록할 수 없습니다."]);
    register_candidate.lock().unwrap().set("종류", "무기");

    inventory_compat::set_item_json_field(
        &mut register_candidate.lock().unwrap(),
        "아이템속성",
        &serde_json::json!(["줄수없음"]),
    );
    let cannot_give = storage
        .execute("등록", &mut body, "등록시험철퇴", None, None, None)
        .unwrap();
    assert_eq!(cannot_give.0, vec!["☞ 등록할 수 없습니다."]);

    inventory_compat::set_item_json_field(
        &mut register_candidate.lock().unwrap(),
        "아이템속성",
        &serde_json::json!(["줄수없음확장"]),
    );
    register_candidate
        .lock()
        .unwrap()
        .set("고유번호", "기존-id");
    let already_unique = storage
        .execute("등록", &mut body, "등록시험철퇴", None, None, None)
        .unwrap();
    assert_eq!(already_unique.0, vec!["☞ 등록할 수 없습니다."]);
    register_candidate.lock().unwrap().set("고유번호", "");

    let registered = storage
        .execute("등록", &mut body, "1등록시험철퇴", None, None, None)
        .unwrap();
    assert_eq!(registered.0, vec!["☞ 등록 되었습니다."]);
    assert_eq!(body.object.objs.len(), 2);
    assert!(body.object.objs[0].lock().unwrap().getBool("inUse"));
    assert!(body
        .object
        .objs
        .iter()
        .any(|item| Arc::ptr_eq(item, &prefix_only)));
    body.object
        .objs
        .retain(|item| !Arc::ptr_eq(item, &prefix_only));
    let registered_entries = crate::book::load(&catalog_path).unwrap();
    assert_eq!(
        registered_entries[1]["attr"]["아이템속성"],
        serde_json::json!(["줄수없음확장"])
    );
    assert!(registered_entries[1]["attr"].get("인덱스").is_none());
    let registered_id =
        uuid::Uuid::parse_str(registered_entries[1]["고유번호"].as_str().unwrap()).unwrap();
    assert_eq!(registered_id.get_version_num(), 4);

    let list = storage
        .execute("대여목록", &mut body, "등록시험철퇴", None, None, None)
        .unwrap();
    assert_eq!(list.0.len(), 1);
    assert!(list.0[0].starts_with("2\t등록시험철퇴\t\t("));
    assert!(list.0[0].ends_with(")\t대여가능"));

    let whitespace_matches = storage
        .execute("대여목록", &mut body, " 등록시험철퇴 ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_matches.0, list.0);

    let mut entries = crate::book::load(&catalog_path).unwrap();
    entries.push(serde_json::json!({
        "이름": "\u{1b}[1;31m짧은검\u{1b}[0m",
        "고유번호": "ansi-list-id",
        "등록자": "색상등록자",
        "대여가능": false,
        "대여": "대여자",
        "인덱스": "289",
        "attr": {}
    }));
    crate::book::save(&catalog_path, &entries).unwrap();

    // Python compares the filter with stripANSI(name), preserves the
    // decorated name in output, and numbers by the original ledger index.
    let decorated = storage
        .execute("대여목록", &mut body, "짧은검", None, None, None)
        .unwrap();
    assert_eq!(
        decorated.0,
        vec!["3\t\x1b[1;31m짧은검\x1b[0m\t\t(색상등록자)\t대여중(대여자)"]
    );
    let absent = storage
        .execute("대여목록", &mut body, "없는검", None, None, None)
        .unwrap();
    assert_eq!(absent.0, vec!["☞ 대여가능한 품목이 없어요."]);

    body.set("관리자등급", 1001_i64);
    let admin_list = storage
        .execute("대여목록", &mut body, "짧은검", None, None, None)
        .unwrap();
    assert_eq!(
        admin_list.0,
        vec!["3\t\x1b[1;31m짧은검\x1b[0m\t\t(색상등록자)\t대여중(대여자)\tansi-list-id"]
    );
    body.set("관리자등급", 1000_i64);
    let boundary = storage
        .execute("대여목록", &mut body, "짧은검", None, None, None)
        .unwrap();
    assert_eq!(boundary.0, decorated.0);

    let mut entries = crate::book::load(&catalog_path).unwrap();
    entries.extend([
        serde_json::json!({
            "이름": "일이삼사오육칠", "고유번호": "seven-char-id",
            "등록자": "일곱등록자", "대여가능": true, "대여": "",
            "인덱스": "289", "attr": {}
        }),
        serde_json::json!({
            "이름": "일이삼사오육칠팔", "고유번호": "eight-char-id",
            "등록자": "여덟등록자", "대여가능": true, "대여": "",
            "인덱스": "289", "attr": {}
        }),
    ]);
    crate::book::save(&catalog_path, &entries).unwrap();
    let seven_chars = storage
        .execute("대여목록", &mut body, "일이삼사오육칠", None, None, None)
        .unwrap();
    assert_eq!(
        seven_chars.0,
        vec!["4\t일이삼사오육칠\t\t(일곱등록자)\t대여가능"]
    );
    let eight_chars = storage
        .execute("대여목록", &mut body, "일이삼사오육칠팔", None, None, None)
        .unwrap();
    assert_eq!(
        eight_chars.0,
        vec!["5\t일이삼사오육칠팔\t(여덟등록자)\t대여가능"]
    );
    entries.truncate(3);
    crate::book::save(&catalog_path, &entries).unwrap();

    let mut entries = crate::book::load(&catalog_path).unwrap();
    entries.pop();
    entries.push(serde_json::json!({
        "이름": "손상등록품", "고유번호": "broken-cancel-id",
        "등록자": player_name, "대여가능": true, "인덱스": "289"
    }));
    crate::book::save(&catalog_path, &entries).unwrap();

    let malformed_cancel = storage
        .execute("등록취소", &mut body, "3", None, None, None)
        .unwrap();
    assert_eq!(
        malformed_cancel.0,
        vec!["☞ 등록 취소 가능한 물품이 없습니다."]
    );
    let mut entries = crate::book::load(&catalog_path).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(
        crate::book::dict_get_string(&entries[2], "고유번호"),
        "broken-cancel-id"
    );
    entries.pop();
    crate::book::save(&catalog_path, &entries).unwrap();

    let whitespace_delete = storage
        .execute("등록삭제", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_delete.0, vec!["☞ 사용법: [물품번호] 등록취소"]);
    let whitespace_cancel = storage
        .execute("등록취소", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_cancel.0, vec!["☞ 사용법: [물품번호] 등록취소"]);

    crate::book::mark_borrowed(&catalog_path, 2, "대여중인사람").unwrap();
    let borrowed_cancel = storage
        .execute("등록취소", &mut body, "2", None, None, None)
        .unwrap();
    assert_eq!(borrowed_cancel.0, vec!["☞ 대여 중입니다."]);
    assert_eq!(crate::book::load(&catalog_path).unwrap().len(), 2);
    let borrowed_id =
        crate::book::dict_get_string(&crate::book::load(&catalog_path).unwrap()[1], "고유번호");
    crate::book::mark_returned(&catalog_path, &borrowed_id).unwrap();

    let canceled = storage
        .execute("등록취소", &mut body, "2번", None, None, None)
        .unwrap();
    assert_eq!(canceled.0, vec!["☞ 등록 취소 되었습니다."]);
    assert_eq!(body.object.objs.len(), 2);
    let restored = body
        .object
        .objs
        .iter()
        .find(|item| {
            item.lock().is_ok_and(|item| {
                !item.getBool("inUse")
                    && item.getName() == "등록시험철퇴"
                    && item.getString("고유번호").is_empty()
            })
        })
        .unwrap()
        .lock()
        .unwrap();
    assert_eq!(restored.getString("반응이름"), "등록시험철퇴\r\n시험철퇴");
    assert_eq!(restored.getString("종류"), "무기");
    assert_eq!(restored.getString("설명"), "");
    assert_eq!(
        inventory_compat::item_field_to_json(&restored, "아이템속성"),
        serde_json::json!(["줄수없음확장"])
    );
    drop(restored);

    let mut entries = crate::book::load(&catalog_path).unwrap();
    entries[0]["등록자"] = serde_json::Value::String("다른등록자".into());
    let mut own_entry = entries[0].clone();
    own_entry["등록자"] = serde_json::Value::String(player_name.clone());
    own_entry["고유번호"] = serde_json::Value::String("owner-delete-id".into());
    entries.push(own_entry);
    crate::book::save(&catalog_path, &entries).unwrap();
    body.set("관리자등급", 2000_i64);
    let not_owner = storage
        .execute("등록취소", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(not_owner.0, vec!["☞ 자신이 등록한 물품이 아닙니다."]);

    body.set("관리자등급", 999_i64);
    let delete_not_owner = storage
        .execute("등록삭제", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(delete_not_owner.0, vec!["☞ 자신이 등록한 물품이 아닙니다."]);
    assert_eq!(crate::book::load(&catalog_path).unwrap().len(), 2);

    let inventory_before_delete = body.object.objs.len();
    let owner_deleted = storage
        .execute("등록삭제", &mut body, "2번", None, None, None)
        .unwrap();
    assert_eq!(owner_deleted.0, vec!["☞ 등록 삭제 되었습니다."]);
    assert_eq!(body.object.objs.len(), inventory_before_delete);
    let remaining = crate::book::load(&catalog_path).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(
        crate::book::dict_get_string(&remaining[0], "등록자"),
        "다른등록자"
    );

    crate::book::mark_borrowed(&catalog_path, 1, "현재대여자").unwrap();

    body.set("관리자등급", 1000_i64);
    let deleted = storage
        .execute("등록삭제", &mut body, "1번", None, None, None)
        .unwrap();
    assert_eq!(deleted.0, vec!["☞ 등록 삭제 되었습니다."]);
    assert_eq!(body.object.objs.len(), inventory_before_delete);
    assert!(crate::book::load(&catalog_path).unwrap().is_empty());

    let invalid_delete = storage
        .execute("등록삭제", &mut body, "0", None, None, None)
        .unwrap();
    assert_eq!(
        invalid_delete.0,
        vec!["☞ 등록 취소 가능한 물품이 없습니다."]
    );

    let empty_catalogue = storage
        .execute("대여목록", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(empty_catalogue.0, vec!["☞ 대여가능한 품목이 없어요."]);

    std::fs::remove_file(&catalog_path).unwrap();
    let missing_catalogue = storage
        .execute("대여목록", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(missing_catalogue.0, vec!["☞ 대여가능한 품목이 없어요."]);
    let missing_delete = storage
        .execute("등록삭제", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(
        missing_delete.0,
        vec!["☞ 등록 취소 가능한 물품이 없습니다."]
    );

    let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player_name);
}

#[test]
fn return_command_distinguishes_missing_item_catalog_and_unmatched_unique_id() {
    use crate::object::Object;
    use crate::world::{get_world_state, PlayerPosition};
    use std::sync::{Arc, Mutex};

    let suffix = std::process::id();
    let player = format!("반납실패회귀-{suffix}");
    let zone = format!("반납실패존-{suffix}");
    let catalog = std::env::temp_dir().join(format!("muc-return-fail-{suffix}.json"));
    let _ = std::fs::remove_file(&catalog);
    let mut sign = Object::new();
    sign.set("이름", "진영표지");
    sign.set("반응이름", "진영");
    let sign = Arc::new(Mutex::new(sign));
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
        world.get_room_objs_mut(&zone, "1").push(sign.clone());
        world.record_floor_item(&zone, "1", &sign);
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("__시험도서목록경로", catalog.to_string_lossy().as_ref());

    assert_eq!(
        storage
            .execute("반납", &mut body, "대여검", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 그런 아이템이 소지품에 없어요."]
    );
    let mut candidate = Object::new();
    candidate.set("이름", "대여검");
    candidate.set("고유번호", "내-고유번호");
    body.object.append(Arc::new(Mutex::new(candidate)));
    assert_eq!(
        storage
            .execute("반납", &mut body, "대여검", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 반납 가능한 물품이 없습니다."]
    );
    crate::book::save(&catalog, &[]).unwrap();
    assert_eq!(
        storage
            .execute("반납", &mut body, "대여검", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 반납 가능한 물품이 없습니다."]
    );
    crate::book::save(
        &catalog,
        &[serde_json::json!({
            "고유번호": "다른-고유번호", "대여가능": false, "대여": player
        })],
    )
    .unwrap();
    assert_eq!(
        storage
            .execute("반납", &mut body, "대여검", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 반납 할 수 없습니다."]
    );
    assert_eq!(body.object.objs.len(), 1);

    let mut world = get_world_state().write().unwrap();
    world.remove_floor_item_record(&zone, "1", &sign);
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&player);
    drop(world);
    let _ = std::fs::remove_file(catalog);
}
