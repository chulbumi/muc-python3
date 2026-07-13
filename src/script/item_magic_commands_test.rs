use super::*;
#[test]
fn set_option_command_selects_numbered_available_item_preserves_order_and_nests_ansi() {
    let mut body = Body::new();
    let storage = ScriptStorage::default();
    let denied = storage
        .execute("옵설정", &mut body, "검 힘 1", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    body.set("관리자등급", 2000_i64);
    for input in ["", "검", "검 힘"] {
        let usage = storage
            .execute("옵설정", &mut body, input, None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [대상] [키] [값] 옵설정"]);
    }
    let equipped = Arc::new(Mutex::new(Object::new()));
    equipped.lock().unwrap().set("이름", "옵션검");
    equipped.lock().unwrap().set("반응이름", "검");
    equipped.lock().unwrap().set("inUse", 1_i64);
    let first = Arc::new(Mutex::new(Object::new()));
    first.lock().unwrap().set("이름", "옵션검");
    first.lock().unwrap().set("반응이름", "검");
    let second = Arc::new(Mutex::new(Object::new()));
    second.lock().unwrap().set("이름", "옵션검");
    second.lock().unwrap().set("반응이름", "검");
    second
        .lock()
        .unwrap()
        .set("옵션", "힘 1\n무시되는줄\n민첩성 2");
    body.object
        .objs
        .extend([equipped.clone(), first.clone(), second.clone()]);
    let invalid_value = storage
        .execute("옵설정", &mut body, "검 힘 잘못", None, None, None)
        .unwrap();
    assert!(invalid_value.0.is_empty());
    assert!(first.lock().unwrap().getString("옵션").is_empty());
    assert_eq!(first.lock().unwrap().getString("이름"), "옵션검");

    let pure_number = storage
        .execute("옵설정", &mut body, "2 힘 77", None, None, None)
        .unwrap();
    assert_eq!(pure_number.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);
    assert!(second.lock().unwrap().getString("옵션").contains("힘 1"));

    let result = storage
        .execute("옵설정", &mut body, "2검 운 3 뒤는무시", None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["☞ 설정되었습니다."]);
    assert!(equipped.lock().unwrap().getString("옵션").is_empty());
    assert!(first.lock().unwrap().getString("옵션").is_empty());
    assert_eq!(
        second.lock().unwrap().getString("옵션"),
        "힘 1\n민첩성 2\n운 3"
    );
    assert_eq!(
        second.lock().unwrap().getString("이름"),
        "\x1b[1;34m옵션검\x1b[0;37m"
    );

    let again = storage
        .execute("옵설정", &mut body, "2검 힘 9", None, None, None)
        .unwrap();
    assert_eq!(again.0, vec!["☞ 설정되었습니다."]);
    assert_eq!(
        second.lock().unwrap().getString("옵션"),
        "힘 9\n민첩성 2\n운 3"
    );
    assert_eq!(
        second.lock().unwrap().getString("이름"),
        "\x1b[1;34m\x1b[1;34m옵션검\x1b[0;37m\x1b[0;37m"
    );
}
#[test]
fn clear_magic_command_honors_rest_numbering_in_use_and_empty_option_behavior() {
    let mut body = Body::new();
    let equipped = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = equipped.lock().unwrap();
        item.set("이름", "마법검");
        item.set("반응이름", "검");
        item.set("inUse", 1_i64);
        item.set("옵션", "힘 99");
    }
    let first = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = first.lock().unwrap();
        item.set("이름", "마법검");
        item.set("반응이름", "검");
        item.set("옵션", "");
        item.set("아이템속성", "버리지못함");
    }
    let second = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = second.lock().unwrap();
        item.set("이름", "마법검");
        item.set("반응이름", "검");
        item.set("옵션", "힘 3\n운 4");
        item.set("아이템속성", "줄수없음");
    }
    body.object
        .objs
        .extend([equipped.clone(), first.clone(), second.clone()]);
    let storage = ScriptStorage::default();

    body.act = crate::player::ActState::Rest;
    let resting = storage
        .execute("지워지워", &mut body, "2검", None, None, None)
        .unwrap();
    assert_eq!(resting.0, vec!["☞ 먹을 수 있는 상황이 아니네요. ^_^"]);
    assert_eq!(second.lock().unwrap().getString("옵션"), "힘 3\n운 4");

    body.act = crate::player::ActState::Stand;
    let numbered = storage
        .execute("지워지워", &mut body, "2검", None, None, None)
        .unwrap();
    assert_eq!(numbered.0, vec!["힘 3\n운 4"]);
    assert!(!second.lock().unwrap().attr.contains_key("옵션"));
    assert!(!second.lock().unwrap().attr.contains_key("아이템속성"));
    assert_eq!(equipped.lock().unwrap().getString("옵션"), "힘 99");

    let pure_number = storage
        .execute("지워지워", &mut body, "2", None, None, None)
        .unwrap();
    assert_eq!(pure_number.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);
    assert_eq!(equipped.lock().unwrap().getString("옵션"), "힘 99");

    let empty = storage
        .execute("지워지워", &mut body, "검", None, None, None)
        .unwrap();
    assert_eq!(empty.0, vec![""]);
    assert!(!first.lock().unwrap().attr.contains_key("옵션"));
    assert!(!first.lock().unwrap().attr.contains_key("아이템속성"));
}
#[test]
fn random_option_command_uses_first_word_numbered_item_and_always_wraps_name() {
    let mut body = Body::new();
    let storage = ScriptStorage::default();
    let denied = storage
        .execute("옵랜덤", &mut body, "시험약병", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    body.set("관리자등급", 2000_i64);
    body.set("레벨", 100_i64);
    let usage = storage
        .execute("옵랜덤", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [대상] 옵랜덤"]);
    let first = Arc::new(Mutex::new(Object::new()));
    first.lock().unwrap().set("이름", "시험약병");
    first.lock().unwrap().set("종류", "기타");
    let second = Arc::new(Mutex::new(Object::new()));
    second.lock().unwrap().set("이름", "시험약병");
    second.lock().unwrap().set("종류", "기타");
    body.object.objs.push(first.clone());
    body.object.objs.push(second.clone());

    let result = storage
        .execute(
            "옵랜덤",
            &mut body,
            "2시험약병 뒤의 단어는 무시",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(result.0, vec!["☞ 설정되었습니다."]);
    assert_eq!(first.lock().unwrap().getString("이름"), "시험약병");
    assert_eq!(
        second.lock().unwrap().getString("이름"),
        "\x1b[1;34m시험약병\x1b[0;37m"
    );
    assert!(second.lock().unwrap().getString("옵션").is_empty());

    let pure_number = storage
        .execute("옵랜덤", &mut body, "2", None, None, None)
        .unwrap();
    assert_eq!(pure_number.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);
    assert_eq!(first.lock().unwrap().getString("이름"), "시험약병");
    assert_eq!(
        second.lock().unwrap().getString("이름"),
        "\x1b[1;34m시험약병\x1b[0;37m"
    );
}
