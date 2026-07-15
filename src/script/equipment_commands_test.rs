use super::*;
#[test]
fn wear_matches_python_usage_missing_type_slot_and_empty_all_failures() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    let usage = storage
        .execute("입어", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [아이템 이름] 착용"]);
    let missing = storage
        .execute("입어", &mut body, "없는옷", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);
    let empty_all = storage
        .execute("입어", &mut body, "전부", None, None, None)
        .unwrap();
    assert_eq!(empty_all.0, vec!["☞ 더이상 착용할 장비가 없어요."]);

    let potion = Arc::new(Mutex::new(Object::new()));
    potion.lock().unwrap().set("이름", "시험환약");
    potion.lock().unwrap().set("종류", "기타");
    body.object.objs.push(potion);
    let wrong_type = storage
        .execute("입어", &mut body, "시험환약", None, None, None)
        .unwrap();
    assert_eq!(wrong_type.0, vec!["☞ 착용할 수 있는것이 아니에요."]);

    let worn = Arc::new(Mutex::new(Object::new()));
    worn.lock().unwrap().set("이름", "착용중상의");
    worn.lock().unwrap().set("종류", "방어구");
    worn.lock().unwrap().set("계층", "상의");
    worn.lock().unwrap().set("inUse", 1_i64);
    let spare = Arc::new(Mutex::new(Object::new()));
    spare.lock().unwrap().set("이름", "여분상의");
    spare.lock().unwrap().set("종류", "방어구");
    spare.lock().unwrap().set("계층", "상의");
    body.object.objs.extend([worn, spare.clone()]);
    let occupied = storage
        .execute("입어", &mut body, "여분상의", None, None, None)
        .unwrap();
    assert_eq!(occupied.0, vec!["☞ 더 이상 착용이 불가능해요."]);
    assert!(!spare.lock().unwrap().getBool("inUse"));
}
#[test]
fn wear_matches_python_mastery_custom_ansi_raw_input_and_room_prompt() {
    let self_name = "착용정밀본인";
    let other = "착용정밀상대";
    let rejected = "착용출력거부상대";
    {
        let mut world = get_world_state().write().unwrap();
        for name in [self_name, other, rejected] {
            world.set_player_position(name, PlayerPosition::new("착용정밀존".into(), "1".into()));
        }
    }
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(other));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("rejects_other_combat_output".into(), Dynamic::from(false));
    person.insert("hp".into(), Dynamic::from(8_i64));
    person.insert("max_hp".into(), Dynamic::from(10_i64));
    person.insert("mp".into(), Dynamic::from(3_i64));
    person.insert("max_mp".into(), Dynamic::from(4_i64));
    let mut rejecting = rhai::Map::new();
    rejecting.insert("name".into(), Dynamic::from(rejected));
    rejecting.insert("show_prompt".into(), Dynamic::from(true));
    rejecting.insert("rejects_other_combat_output".into(), Dynamic::from(true));
    rejecting.insert("hp".into(), Dynamic::from(1_i64));
    rejecting.insert("max_hp".into(), Dynamic::from(1_i64));
    rejecting.insert("mp".into(), Dynamic::from(1_i64));
    rejecting.insert("max_mp".into(), Dynamic::from(1_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person), Dynamic::from(rejecting)]),
    );
    crate::script::party::set_precomputed_party_context(context);

    let mut body = Body::new();
    body.set("이름", self_name);
    for weapon in 1..=5 {
        body.set(&format!("{weapon} 숙련도"), 1999_i64);
    }
    let robe = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = robe.lock().unwrap();
        item.set("이름", "비단옷");
        item.set("반응이름", "비단");
        item.set("종류", "방어구");
        item.set("계층", "상의");
        item.set("안시", "\x1b[35m");
        item.set("아이템속성", "올숙이천무기");
    }
    let all_named = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = all_named.lock().unwrap();
        item.set("이름", "모두");
        item.set("종류", "방어구");
        item.set("계층", "하의");
    }
    body.object.objs.extend([robe.clone(), all_named.clone()]);
    let storage = ScriptStorage::default();

    let denied = storage
        .execute("입어", &mut body, "비단", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 당신의 능력으로는 착용이 불가능해요."]);
    assert!(!robe.lock().unwrap().getBool("inUse"));

    let spaced = storage
        .execute("입어", &mut body, " 모두 ", None, None, None)
        .unwrap();
    assert!(!spaced.0.is_empty());
    assert!(all_named.lock().unwrap().getBool("inUse"));
    all_named.lock().unwrap().set("inUse", 0_i64);

    for weapon in 1..=5 {
        body.set(&format!("{weapon} 숙련도"), 2000_i64);
    }
    let worn = storage
        .execute("입어", &mut body, "비단", None, None, None)
        .unwrap();
    assert_eq!(
        worn.0,
        vec!["당신이 \x1b[35m비단옷\x1b[0;37m을 착용합니다."]
    );
    assert!(robe.lock().unwrap().getBool("inUse"));
    assert!(matches!(
        worn.1,
        Some(CommandResult::OutputAndSendToUsers(_, ref sends))
            if sends == &vec![
                (
                    other.to_string(),
                    format!("{}\r\n\x1b[1m{self_name}\x1b[0;37m{} \x1b[35m비단옷\x1b[0;37m을 착용합니다.\r\n\r\n\x1b[0;37;40m[ 8/10, 3/4 ] ", RAW_USER_MESSAGE_PREFIX, han_iga(self_name))
                ),
                (
                    rejected.to_string(),
                    format!("{}\r\n\x1b[1m{self_name}\x1b[0;37m{} \x1b[35m비단옷\x1b[0;37m을 착용합니다.\r\n\r\n\x1b[0;37;40m[ 1/1, 1/1 ] ", RAW_USER_MESSAGE_PREFIX, han_iga(self_name))
                )
            ]
    ));

    crate::script::party::set_precomputed_party_context(rhai::Map::new());
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(self_name);
    world.remove_player_position(other);
    world.remove_player_position(rejected);
}
#[test]
fn equipment_script_ignores_unlisted_slots_and_prints_python_bare_body_line() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "맨몸장비검사");
    body.armor = 0;
    body.attpower = 0;

    let unknown = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = unknown.lock().unwrap();
        item.set("이름", "표시되면안됨");
        item.set("계층", "없는계층");
        item.set("inUse", 1_i64);
    }
    body.object.objs.push(unknown);

    let output = storage
        .execute("장비", &mut body, "무시되는 인자", None, None, None)
        .unwrap()
        .0;
    assert_eq!(output.len(), 1);
    assert!(output[0].contains("\x1b[36m☞ 혈혈단신 맨몸으로 강호를 주유중입니다.\x1b[37m\r\n"));
    assert!(output[0].contains("【방어력】▷ 0    【공격력】▷ 0\r\n"));
    assert!(!output[0].contains("표시되면안됨"));
}
#[test]
fn equipment_script_matches_python_layout_and_item_order() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "장비검사");
    body.armor = 120;
    body.attpower = 44;

    let weapon = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = weapon.lock().unwrap();
        item.set("이름", "철검");
        item.set("계층", "무기");
        item.set("반응이름", "검 철검");
        item.set("inUse", 1i64);
    }
    body.object.objs.push(weapon);

    let helmet = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = helmet.lock().unwrap();
        item.set("이름", "Excalibur");
        item.set("계층", "투구");
        item.set("반응이름", "엑스칼리버 보검");
        item.set("inUse", 1i64);
    }
    body.object.objs.push(helmet);

    let boots = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = boots.lock().unwrap();
        item.set("이름", "Boots");
        item.set("계층", "신발");
        // Python list 호환 표현: list일 때는 첫 항목만 표시한다.
        item.set("반응이름", "장화\r\n부츠");
        item.set("inUse", 1i64);
    }
    body.object.objs.push(boots);

    let (output, special) = storage
        .execute("장비", &mut body, "", None, None, None)
        .unwrap();
    assert!(special.is_none());
    let header = fill_space_euc_kr(54, "▷ 당신은 초라한 방어구를 착용하고 있습니다.");
    let expected = format!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n\
             \x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m\r\n\
             ───────────────────────────\r\n\
             [투    구] \x1b[36mExcalibur(엑스칼리버 보검)\x1b[37m\r\n\
             [신    발] \x1b[36mBoots(장화)\x1b[37m\r\n\
             [무    기] \x1b[36m철검\x1b[37m\r\n\
             ───────────────────────────\r\n\
             【방어력】▷ 120    【공격력】▷ 44\r\n\
             ━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        header
    );
    assert_eq!(output, vec![expected]);
}
#[test]
fn unequip_uses_the_selected_items_real_name_ansi_particle_and_stats() {
    use crate::script::party::set_precomputed_party_context;
    use crate::world::{get_world_state, PlayerPosition};

    let storage = ScriptStorage::default();
    let suffix = std::process::id();
    let actor = format!("해제표시회귀-{suffix}");
    let observer = format!("해제관찰자-{suffix}");
    let zone = format!("해제표시존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&actor, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&observer, PlayerPosition::new(zone, "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", actor.as_str());
    body.attpower = 7;
    body._str = 4;

    let mut item = Object::new();
    item.set("이름", "Sword");
    item.set("반응이름", "검 칼");
    item.set("안시", "\x1b[35m");
    item.set("종류", "무기");
    item.set("공격력", 3_i64);
    item.set("inUse", 1_i64);
    item.set("옵션", "힘 2");
    body.object.objs.push(Arc::new(Mutex::new(item)));
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(observer.clone()));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("hp".into(), Dynamic::from(71_i64));
    person.insert("max_hp".into(), Dynamic::from(81_i64));
    person.insert("mp".into(), Dynamic::from(9_i64));
    person.insert("max_mp".into(), Dynamic::from(10_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);

    let result = storage
        .execute("벗어", &mut body, "칼", None, None, None)
        .unwrap();
    assert_eq!(
        result.0,
        vec!["당신이 \x1b[36m\x1b[35mSword\x1b[0;37m을\x1b[37m 착용해제 합니다."]
    );
    assert_eq!(body.attpower, 4);
    assert_eq!(body._str, 2);
    assert!(!body.object.objs[0].lock().unwrap().getBool("inUse"));
    let sends = match result.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected unequip result: {other:?}"),
    };
    assert_eq!(
        sends,
        vec![(
            observer.clone(),
            format!(
                "{}\r\n\x1b[1m{actor}\x1b[0;37m{} \x1b[36m\x1b[35mSword\x1b[0;37m을\x1b[37m 착용해제 합니다.\r\n\r\n\x1b[0;37;40m[ 71/81, 9/10 ] ",
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
fn unequip_rejoins_a_pristine_materialized_item_to_its_counted_stack() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.object.inv_stack.insert("361".into(), 2);

    storage
        .execute("입어", &mut body, "수박모자", None, None, None)
        .unwrap();
    assert_eq!(body.object.inv_stack.get("361"), Some(&1));
    assert_eq!(body.object.objs.len(), 1);
    assert!(body.object.objs[0].lock().unwrap().getBool("inUse"));

    storage
        .execute("벗어", &mut body, "수박모자", None, None, None)
        .unwrap();
    assert_eq!(body.object.inv_stack.get("361"), Some(&2));
    assert!(body.object.objs.is_empty());
}

#[test]
fn numbered_equip_counts_stateful_objects_before_pristine_quantity() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    let (modified, _) = object_from_item_json("361").unwrap();
    modified.lock().unwrap().set("판매가격", 99_i64);
    body.object.objs.push(modified.clone());
    body.object.inv_stack.insert("361".into(), 1);

    let equipped = storage
        .execute("입어", &mut body, "2수박모자", None, None, None)
        .unwrap();

    assert_eq!(equipped.0.len(), 1);
    assert!(!body.object.inv_stack.contains_key("361"));
    assert_eq!(body.object.objs.len(), 2);
    assert!(body.object.objs.iter().any(|item| {
        let item = item.lock().unwrap();
        item.getBool("inUse") && item.getInt("판매가격") == 9
    }));
    assert!(!modified.lock().unwrap().getBool("inUse"));

    storage
        .execute("벗어", &mut body, "수박모자", None, None, None)
        .unwrap();
    assert_eq!(body.object.inv_stack.get("361"), Some(&1));
    assert_eq!(body.object.objs.len(), 1);
    assert!(Arc::ptr_eq(&body.object.objs[0], &modified));
}

#[test]
fn all_mastery_item_list_matches_python_user_value_comparisons() {
    let suffix = std::process::id();
    let fixtures = [
        (format!("올숙숫자영-{suffix}"), serde_json::json!(0), false),
        (
            format!("올숙실수영-{suffix}"),
            serde_json::json!(0.0),
            false,
        ),
        (
            format!("올숙거짓-{suffix}"),
            serde_json::json!(false),
            false,
        ),
        (format!("올숙빈문자-{suffix}"), serde_json::json!(""), false),
        (
            format!("올숙소유자-{suffix}"),
            serde_json::json!("검사자"),
            true,
        ),
        (format!("올숙널-{suffix}"), serde_json::Value::Null, true),
    ];
    let mut paths = Vec::new();
    for (index, user, _) in &fixtures {
        let path = std::path::Path::new("data/item").join(format!("{index}.json"));
        std::fs::write(
            &path,
            serde_json::json!({"아이템정보":{"이름":index,"사용자":user}}).to_string(),
        )
        .unwrap();
        paths.push(path);
    }

    let mut body = Body::new();
    body.set("관리자등급", 999_i64);
    let denied = ScriptStorage::default()
        .execute("올숙리스트", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    body.set("관리자등급", 1000_i64);
    let output = ScriptStorage::default()
        .execute("올숙리스트", &mut body, "무시되는 인자", None, None, None)
        .unwrap();
    assert_eq!(output.0.len(), 1);
    assert!(output.0[0].starts_with(
            "[올숙아이템인덱스]\r\n#무극탈명도\r\n:초류빈_올숙무기\r\n\r\n#\x1b[1;30;47m흑도\x1b[1;37;40m요루\x1b[0;37;40m\r\n:철화접_올숙무기\r\n\r\n"
        ));
    for (index, _, included) in &fixtures {
        let entry = format!("#{index}\r\n:{index}\r\n\r\n");
        assert_eq!(
            output.0[0].contains(&entry),
            *included,
            "Python 사용자 != 0 and 사용자 != '' semantics for {index}"
        );
    }

    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}
