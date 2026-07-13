use super::*;

#[test]
fn oneitem_cleanup_variants_preserve_python_storage_delete_difference() {
    let _oneitem_guard = ONEITEM_COMMAND_TEST_LOCK.lock().unwrap();
    let attr_before = std::fs::read("data/config/oneitem.json").unwrap();
    assert!(crate::oneitem::oneitem_have("77", "기연소유자 보관 추가"));
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 2000_i64);

    set_precomputed_connected_names(vec![Dynamic::from("기연소유자")]);
    let online = storage
        .execute("기연정리", &mut body, "간장검", None, None, None)
        .unwrap();
    assert_eq!(online.0, vec!["사용자가 접속중입니다.!"]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "기연소유자 보관 추가");

    set_precomputed_connected_names(Vec::new());
    let retained = storage
        .execute("기연정리", &mut body, "간장검", None, None, None)
        .unwrap();
    assert_eq!(retained.0, vec!["기연소유자 보관"]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "기연소유자 보관 추가");

    let removed = storage
        .execute("기연정리리", &mut body, "간장검", None, None, None)
        .unwrap();
    assert_eq!(removed.0, vec!["기연소유자 보관"]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "");

    clear_precomputed_all_online();
    std::fs::write("data/config/oneitem.json", attr_before).unwrap();
}

#[test]
fn oneitem_admin_commands_cover_offline_age_inventory_delete_and_empty_list() {
    let _oneitem_guard = ONEITEM_COMMAND_TEST_LOCK.lock().unwrap();
    use crate::object::Object;

    let attr_before = std::fs::read("data/config/oneitem.json").unwrap();
    let index_before = std::fs::read("data/config/oneitem_index.json").unwrap();
    assert!(crate::oneitem::oneitem_clear());
    let suffix = std::process::id();
    let owner = format!("기연오프라인-{suffix}");
    let owner_path = format!("data/user/{owner}.json");
    let mut item = Object::new();
    item.set("이름", "간장검");
    item.set("인덱스", "77");
    let item = Arc::new(Mutex::new(item));
    let mut saved = Body::new();
    saved.set("이름", owner.as_str());
    let old_timestamp = chrono::Utc::now().timestamp() - 259_201;
    saved.set("마지막저장시간", old_timestamp);
    saved.object.append(item);
    assert!(save_body_to_json_without_timestamp(&mut saved, &owner_path));

    let mut admin = Body::new();
    admin.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();
    set_precomputed_connected_names(Vec::new());

    let mut non_admin = Body::new();
    for command in [
        "기연",
        "기연삭제",
        "기연삭제1",
        "기연정리",
        "기연정리리",
        "기연초기화",
    ] {
        let denied = storage
            .execute(command, &mut non_admin, "", None, None, None)
            .unwrap();
        assert_eq!(
            denied.0,
            vec!["☞ 무슨 말인지 모르겠어요. *^_^*"],
            "{command} permission"
        );
    }

    assert!(crate::oneitem::oneitem_have("77", &owner));
    let listed = storage
        .execute("기연", &mut admin, "무시", None, None, None)
        .unwrap();
    assert_eq!(
        listed.0,
        vec![format!("{:<16} ({:<16}) : {owner}\r\n", "간장검", "77")]
    );
    for command in ["기연정리", "기연정리리"] {
        let usage = storage
            .execute(command, &mut admin, "   ", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [기연이름] 기연정리"]);
        let unknown = storage
            .execute(command, &mut admin, "없는기연", None, None, None)
            .unwrap();
        assert_eq!(unknown.0, vec!["☞ 그런 아이템은 없습니다.!"]);
    }

    assert!(crate::oneitem::oneitem_have("77", "없는사용자"));
    let missing_owner = storage
        .execute("기연정리", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(missing_owner.0, vec!["존재하지않는 사용자입니다."]);
    assert!(crate::oneitem::oneitem_have("77", &owner));

    let string_index = format!("기연문자열-{suffix}");
    let array_index = format!("기연배열-{suffix}");
    let ordinary_index = format!("기연일반-{suffix}");
    for (index, name, attributes) in [
        (
            &string_index,
            "문자열부분기연",
            serde_json::json!("비단일아이템속성"),
        ),
        (
            &array_index,
            "배열기연",
            serde_json::json!(["기타", "단일아이템"]),
        ),
        (&ordinary_index, "일반아이템", serde_json::json!(["기타"])),
    ] {
        std::fs::write(
            format!("data/item/{index}.json"),
            serde_json::to_string(&serde_json::json!({
                "아이템정보": {
                    "이름": name,
                    "아이템속성": attributes
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }
    let indexed = storage
        .execute("기연리스트", &mut admin, "", None, None, None)
        .unwrap();
    assert!(indexed.0[0].contains(&format!("#문자열부분기연\r\n:{string_index}\r\n\r\n")));
    assert!(indexed.0[0].contains(&format!("#배열기연\r\n:{array_index}\r\n\r\n")));
    assert!(!indexed.0[0].contains(&ordinary_index));
    for index in [&string_index, &array_index, &ordinary_index] {
        let _ = std::fs::remove_file(format!("data/item/{index}.json"));
    }
    let spaced_name = storage
        .execute("기연정리", &mut admin, " 간장검 ", None, None, None)
        .unwrap();
    assert_eq!(
        spaced_name.0,
        vec![format!("{owner}의 간장검을 정리하였습니다.")]
    );
    assert_eq!(crate::oneitem::oneitem_get("77"), "");
    let whitespace_delete = storage
        .execute("기연삭제1", &mut admin, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_delete.0, vec!["☞ 사용법: [기연이름] 기연삭제"]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "");
    let mut loaded = Body::new();
    assert!(load_body_from_json(&mut loaded, &owner_path));
    assert!(loaded
        .object
        .objs
        .iter()
        .all(|item| item.lock().unwrap().getString("인덱스") != "77"));
    assert_eq!(loaded.get_int("마지막저장시간"), old_timestamp);

    assert!(crate::oneitem::oneitem_have("77", &owner));
    let absent_from_inventory = storage
        .execute("기연정리", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(absent_from_inventory.0, vec![format!("{owner} ")]);
    assert_eq!(crate::oneitem::oneitem_get("77"), owner);
    assert!(crate::oneitem::oneitem_destroy("77"));

    assert!(crate::oneitem::oneitem_have("77", &owner));
    let absent_removed = storage
        .execute("기연정리리", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(absent_removed.0, vec![format!("{owner} ")]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "");
    let saved_after_absent: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("data/config/oneitem.json").unwrap())
            .unwrap();
    assert!(saved_after_absent["단일아이템"].get("77").is_none());

    let mut alias_item = Object::new();
    alias_item.set("이름", "간장검");
    alias_item.set("인덱스", "77");
    saved.object.objs.clear();
    saved.object.append(Arc::new(Mutex::new(alias_item)));
    saved.set("마지막저장시간", old_timestamp);
    assert!(save_body_to_json_without_timestamp(&mut saved, &owner_path));
    assert!(crate::oneitem::oneitem_have("77", &owner));
    let alias_cleaned = storage
        .execute("기연정리리", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(
        alias_cleaned.0,
        vec![format!("{owner}의 간장검을 정리하였습니다.")]
    );
    assert_eq!(crate::oneitem::oneitem_get("77"), "");
    let mut alias_loaded = Body::new();
    assert!(load_body_from_json(&mut alias_loaded, &owner_path));
    assert!(alias_loaded
        .object
        .objs
        .iter()
        .all(|item| item.lock().unwrap().getString("인덱스") != "77"));

    saved.object.objs.clear();
    let mut recent_item = Object::new();
    recent_item.set("이름", "간장검");
    recent_item.set("인덱스", "77");
    saved.object.append(Arc::new(Mutex::new(recent_item)));
    saved.set("마지막저장시간", chrono::Utc::now().timestamp());
    assert!(save_body_to_json_without_timestamp(&mut saved, &owner_path));
    assert!(crate::oneitem::oneitem_have("77", &owner));
    let recent = storage
        .execute("기연정리", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(recent.0, vec!["아직 3일이 경과하지 않았습니다."]);
    assert_eq!(crate::oneitem::oneitem_get("77"), owner);

    let deleted = storage
        .execute("기연삭제", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(deleted.0, vec!["☞ 기연이 삭제되었습니다."]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "");
    let saved_after_delete: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("data/config/oneitem.json").unwrap())
            .unwrap();
    assert!(saved_after_delete["단일아이템"].get("77").is_none());
    let absent_delete = storage
        .execute("기연삭제", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(absent_delete.0, vec!["☞ 그런 아이템은 없습니다.!"]);

    assert!(crate::oneitem::oneitem_have("77", &owner));
    let deleted_alias = storage
        .execute("기연삭제1", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(deleted_alias.0, vec!["☞ 기연이 삭제되었습니다."]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "");

    assert!(crate::oneitem::oneitem_have("77", "이름  상태   추가"));
    let spaced_owner = storage
        .execute("기연정리", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(spaced_owner.0, vec!["이름 상태"]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "이름  상태   추가");
    assert!(crate::oneitem::oneitem_destroy("77"));
    assert!(crate::oneitem::oneitem_have("77", "하나 둘 셋 넷"));
    let malformed = storage
        .execute("기연정리", &mut admin, "간장검", None, None, None)
        .unwrap();
    assert_eq!(malformed.0, vec!["아무도 소지하고 있지 않습니다.!"]);

    let index_immediately_before_clear = std::fs::read("data/config/oneitem_index.json").unwrap();
    let initialized = storage
        .execute("기연초기화", &mut admin, "무시", None, None, None)
        .unwrap();
    assert_eq!(initialized.0, vec!["* 기연아이템 목록이 초기화되었습니다."]);
    assert_eq!(crate::oneitem::oneitem_get("77"), "");
    let cleared_file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("data/config/oneitem.json").unwrap())
            .unwrap();
    assert_eq!(cleared_file["단일아이템"], serde_json::json!({}));
    assert_eq!(
        std::fs::read("data/config/oneitem_index.json").unwrap(),
        index_immediately_before_clear,
        "Python ONEITEM.clear only saves attr and preserves the name/index catalog"
    );
    let empty = storage
        .execute("기연", &mut admin, "", None, None, None)
        .unwrap();
    assert_eq!(empty.0, vec![""]);

    set_precomputed_connected_names(Vec::new());
    let _ = std::fs::remove_file(owner_path);
    std::fs::write("data/config/oneitem.json", attr_before).unwrap();
    std::fs::write("data/config/oneitem_index.json", index_before).unwrap();
    assert!(crate::oneitem::oneitem_reload());
}
