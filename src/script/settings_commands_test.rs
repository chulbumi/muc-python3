use super::*;
#[test]
fn settings_command_lists_python_cfg_and_toggles_with_python_text() {
    let player_name = format!("설정회귀-{}", std::process::id());
    let player_path = format!("data/user/{player_name}.json");
    let _ = std::fs::remove_file(&player_path);
    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("설정상태", "자동습득 1\n전음거부 0");
    let storage = ScriptStorage::default();

    let listed = storage
        .execute("설정", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(listed.0[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    assert_eq!(
        listed.0[1],
        "\x1b[47m\x1b[30m◁               설      정      상      태               ▷\x1b[40m\x1b[37m"
    );
    assert!(listed
        .0
        .join("\r\n")
        .contains("자동습득         [\x1b[1m설  정\x1b[0;37m]"));
    assert!(listed.0.join("\r\n").contains("전음거부         [비설정]"));
    assert!(listed.0.join("\r\n").contains("자동채널입장     [비설정]"));
    assert_eq!(listed.0.last().unwrap(), "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let whitespace = storage
        .execute("설정", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, listed.0);

    let enabled = storage
        .execute("설정", &mut body, "전음거부", None, None, None)
        .unwrap();
    assert_eq!(
        enabled.0,
        vec!["☞ 전음거부를 \x1b[1m[설정]\x1b[0;37m 하였습니다."]
    );
    assert!(config_is_enabled(&body.get_string("설정상태"), "전음거부"));
    assert_eq!(body.get_string("설정상태"), "자동습득 1\n전음거부 1");
    assert!(!std::path::Path::new(&player_path).exists());

    let disabled = storage
        .execute("설정", &mut body, "전음거부", None, None, None)
        .unwrap();
    assert_eq!(
        disabled.0,
        vec!["☞ 전음거부를 \x1b[1m[비설정]\x1b[0;37m 하였습니다."]
    );
    assert!(!config_is_enabled(&body.get_string("설정상태"), "전음거부"));
    assert_eq!(body.get_string("설정상태"), "자동습득 1\n전음거부 0");
    assert!(!std::path::Path::new(&player_path).exists());

    let invalid = storage
        .execute("설정", &mut body, "없는설정", None, None, None)
        .unwrap();
    assert_eq!(invalid.0, vec!["☞ 그런 설정은 없어요. ^^"]);
    let _ = std::fs::remove_file(player_path);
}
#[test]
fn user_alias_json_round_trip_uses_python_array_without_touching_user_data() {
    let path = std::env::temp_dir().join(format!(
        "muc_alias_round_trip_{}_{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let entries = vec![
        ("대상".to_string(), "* 쳐;봐".to_string()),
        ("파이프".to_string(), "값|그대로 전음".to_string()),
    ];
    let mut body = Body::new();
    body.set("이름", "임시줄임말검사");
    body.set(ALIAS_LIST_ATTR, encode_alias_entries(&entries));
    assert!(save_body_to_json(&mut body, path.to_str().unwrap()));

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        json["사용자오브젝트"][ALIAS_LIST_ATTR],
        serde_json::json!(["대상 * 쳐;봐", "파이프 값|그대로 전음"])
    );

    let mut loaded = Body::new();
    assert!(load_body_from_json(&mut loaded, path.to_str().unwrap()));
    assert_eq!(
        decode_alias_entries(&loaded.get_string(ALIAS_LIST_ATTR)),
        entries
    );
    let _ = std::fs::remove_file(path);
}
#[test]
fn automatic_route_whitespace_is_empty_and_deletes_existing_route() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set(
        ALIAS_LIST_ATTR,
        encode_alias_entries(&[("길".to_string(), "동;서".to_string())]),
    );
    body.temp_mut()
        .insert("_auto_move_count".to_string(), Value::Int(2));

    let whitespace = storage
        .execute("자동경로", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 자동경로가 삭제되었습니다."]);
    assert_eq!(take_auto_move_request(&mut body).as_deref(), Some(""));

    let selected = storage
        .execute("자동경로", &mut body, "길", None, None, None)
        .unwrap();
    assert_eq!(selected.0, vec!["☞ 자동경로를 설정하였어요. ^^"]);
    assert_eq!(take_auto_move_request(&mut body).as_deref(), Some("동;서"));

    let missing = storage
        .execute("자동경로", &mut body, "없는길", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 해당 줄임말이 없어요. ^^"]);
    assert!(take_auto_move_request(&mut body).is_none());

    body.temp_mut()
        .insert("_auto_move_count".to_string(), Value::Int(0));
    let none = storage
        .execute("자동경로", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(none.0, vec!["☞ 사용법: [지정한줄임말] 자동경로"]);
    assert!(take_auto_move_request(&mut body).is_none());
}
#[test]
fn user_alias_rhai_enforces_python_hundred_entry_limit() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "줄임말제한검사");
    let entries: Vec<(String, String)> = (0..100)
        .map(|index| (format!("키{}", index), "북".to_string()))
        .collect();
    body.set(ALIAS_LIST_ATTR, encode_alias_entries(&entries));

    let (output, _) = storage
        .execute("줄임말", &mut body, "초과 남", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 줄임말이 너무 많아요. ^^"]);
    assert_eq!(
        decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR)).len(),
        100
    );
}
#[test]
fn user_alias_rhai_matches_python_messages_and_state_rules() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("줄임말"));
    let mut body = Body::new();
    body.set("이름", "줄임말검사");

    let (output, _) = storage
        .execute("줄임말", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 줄임말이 설정되어 있지 않아요. ^^"]);

    let (output, _) = storage
        .execute("줄임말", &mut body, "길 동;서", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 줄임말을 설정하였어요. ^^"]);
    assert_eq!(
        decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR)),
        vec![("길".to_string(), "동;서".to_string())]
    );

    let (output, _) = storage
        .execute("줄임말", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        output,
        vec![
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            "\x1b[47m\x1b[30m◁ 줄임말 ▷                                                                  \x1b[40m\x1b[37m",
            "───────────────────────────────────────",
            "[길] 동;서",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        ]
    );

    let (output, _) = storage
        .execute("줄임말", &mut body, "길 북", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 이미 설정되어 있는 줄임말입니다."]);

    for invalid in ["자기 자기", "중첩 길"] {
        let (output, _) = storage
            .execute("줄임말", &mut body, invalid, None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 중첩된 줄임말은 사용할 수 없어요. ^^"]);
    }

    let (output, _) = storage
        .execute("줄임말", &mut body, "길", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 줄임말을 제거하였어요. ^^"]);
    let (output, _) = storage
        .execute("줄임말", &mut body, "길", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 줄임말이 설정되어 있지 않아요. ^^"]);
}

#[test]
fn settings_preserve_python_first_prefix_and_duplicate_toggle_behavior() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "설정접두회귀");
    body.set("설정상태", "전음거부추가 1\n전음거부 0\n임의설정 1");

    let listed = storage
        .execute("설정", &mut body, "", None, None, None)
        .unwrap();
    let joined = listed.0.join("\r\n");
    assert!(
        joined.contains("전음거부         [\x1b[1m설  정\x1b[0;37m]"),
        "Python _checkConfig accepts the first startswith match"
    );
    assert_eq!(listed.0.len(), 14);
    assert_eq!(listed.0[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    assert_eq!(listed.0[13], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let toggled = storage
        .execute("설정", &mut body, "전음거부", None, None, None)
        .unwrap();
    assert_eq!(
        toggled.0,
        vec!["☞ 전음거부를 \x1b[1m[설정]\x1b[0;37m 하였습니다."]
    );
    assert_eq!(
        body.get_string("설정상태"),
        "전음거부추가 0\n전음거부 1\n임의설정 1"
    );

    let relisted = storage
        .execute("설정", &mut body, "", None, None, None)
        .unwrap();
    assert!(
        relisted
            .0
            .join("\r\n")
            .contains("전음거부         [비설정]"),
        "the first matching prefix remains authoritative after both entries toggle"
    );
}

#[test]
fn settings_require_exact_cfg_argument_after_command_whitespace_normalization() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    let spaced = storage
        .execute("설정", &mut body, " 전음거부 ", None, None, None)
        .unwrap();
    assert_eq!(
        spaced.0,
        vec!["☞ 전음거부를 \x1b[1m[설정]\x1b[0;37m 하였습니다."]
    );
    let suffix = storage
        .execute("설정", &mut body, "전음거부 추가", None, None, None)
        .unwrap();
    assert_eq!(suffix.0, vec!["☞ 그런 설정은 없어요. ^^"]);
}
