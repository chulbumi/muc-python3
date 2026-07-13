use super::*;
#[test]
fn tell_history_is_runtime_only_like_python_player_state() {
    let path = std::env::temp_dir().join(format!(
        "muc_tell_history_{}_{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut body = Body::new();
    body.set("이름", "임시전음기록검사");
    body.talk_history.push("현재 접속 기록".to_string());
    assert!(save_body_to_json(&mut body, path.to_str().unwrap()));

    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(json.get("대화기록").is_none());

    // 과거 Rust 파일의 발명된 필드도 새 접속에는 복원하지 않는다.
    json.as_object_mut()
        .unwrap()
        .insert("대화기록".to_string(), serde_json::json!(["오래된 기록"]));
    std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();
    let mut loaded = Body::new();
    loaded.talk_history.push("초기값".to_string());
    assert!(load_body_from_json(&mut loaded, path.to_str().unwrap()));
    assert!(loaded.talk_history.is_empty());
    let _ = std::fs::remove_file(path);
}

#[test]
fn direct_tell_without_sender_room_does_not_deliver_like_python_env_failure() {
    let mut body = Body::new();
    body.set("이름", "방없는전음자");
    set_precomputed_tell_players(vec![TellPlayerSnapshot::new(
        "방없는전음수신토큰".into(),
        "전음수신자".into(),
        true,
        false,
        "",
        1,
        10,
        10,
        10,
        10,
        false,
    )]);

    let result = ScriptStorage::default()
        .execute("전음", &mut body, "전음수신자 내용", None, None, None)
        .unwrap();

    set_precomputed_tell_players(Vec::new());
    assert!(result.0.is_empty());
    assert!(result.1.is_none());
}

#[test]
fn previous_talk_command_preserves_python_order_and_raw_lines() {
    let mut body = Body::new();
    body.talk_history = (0..22)
        .map(|index| format!("\x1b[3{}m대화-{index}\x1b[0m", index % 8))
        .collect();
    let storage = ScriptStorage::default();
    let talk = storage
        .execute("지난대화", &mut body, "무시되는 인자", None, None, None)
        .unwrap();
    assert_eq!(talk.0, body.talk_history);
}

#[test]
fn previous_chat_command_uses_python_class_history_order_and_24_line_cap() {
    CHAT_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .clear();
    for index in 0..25 {
        record_chat_history(&format!("\x1b[3{}m잡담-{index}\x1b[0;37m", index % 8));
    }
    let mut body = Body::new();
    body.set("이름", "지난잡담회귀");
    let shown = ScriptStorage::default()
        .execute("지난잡담", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(shown.0.len(), 24);
    assert_eq!(shown.0[0], "\x1b[31m잡담-1\x1b[0;37m");
    assert_eq!(shown.0[23], "\x1b[30m잡담-24\x1b[0;37m");
}

#[test]
fn reply_tell_uses_connection_identity_python_spacing_and_prompt_text() {
    let token = "reply-target-token";
    set_precomputed_tell_players(vec![TellPlayerSnapshot::new(
        token.to_string(),
        "답장대상".to_string(),
        true,
        true, // Python 반전음은 기억된 접속 객체의 투명 상태를 다시 검사하지 않음.
        "전음거부 0\n엘피출력 0",
        1,
        31,
        45,
        7,
        9,
        false,
    )]);
    let mut body = Body::new();
    body.set("이름", "답장발신자");
    body.temp_mut().insert(
        TELL_TALKER_TOKEN.to_string(),
        Value::String(token.to_string()),
    );
    let storage = ScriptStorage::default();

    let result = storage
        .execute("반전음", &mut body, "  여러   단어  ", None, None, None)
        .unwrap();
    assert!(result.0.is_empty());
    assert!(matches!(
        result.1,
        Some(CommandResult::Tell {
            ref target_token,
            ref sender_output,
            ref recipient_output,
            ref history_line,
        }) if target_token == token
            && sender_output == "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] 답장대상에게 보냄 : 여러 단어 \r\n\r\n"
            && recipient_output == "\r\n[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] 답장발신자 : 여러 단어 \r\n\r\n\x1b[0;37;40m[ 31/45, 7/9 ] "
            && history_line == "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] 답장발신자 : 여러 단어 "
    ));

    // 같은 이름의 재접속자가 있어도 token이 달라지면 Python의 이전
    // `_talker` 객체는 channel.players에 없는 것으로 취급한다.
    set_precomputed_tell_players(vec![TellPlayerSnapshot::new(
        "new-token".to_string(),
        "답장대상".to_string(),
        true,
        false,
        "",
        1,
        1,
        1,
        1,
        1,
        false,
    )]);
    let disconnected = storage
        .execute("반전음", &mut body, "답", None, None, None)
        .unwrap();
    assert_eq!(
        disconnected.0,
        vec!["☞ 전음이 전달될만한 상대가 없어요. ^^"]
    );
    assert!(!body.temp().contains_key(TELL_TALKER_TOKEN));
    set_precomputed_tell_players(Vec::new());
}

#[test]
fn direct_tell_matches_python_target_guards_room_order_and_self_wire_output() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let sender = format!("전음발신자-{suffix}");
    let zone = format!("전음시험존-{suffix}");
    let zone_path = Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&zone_path).unwrap();
    std::fs::write(
        zone_path.join("1.json"),
        serde_json::to_vec(&serde_json::json!({
            "맵정보": {
                "맵속성": ["모든통신금지"],
                "설명": [], "이름": "전음금지시험방", "존이름": zone,
                "출구": [], "몹": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let mut body = Body::new();
    body.set("이름", sender.as_str());
    body.set("설정상태", "전음거부 0\n엘피출력 0");
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.set_player_position(&sender, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let storage = ScriptStorage::default();

    let mut non_admin = Body::new();
    non_admin.set("이름", "값값권한검사");
    let denied = storage
        .execute("값값", &mut non_admin, "", None, None, None)
        .unwrap();
    assert_eq!(denied.0.len(), 2);
    assert!(denied.0[0].parse::<i64>().is_ok());
    assert_eq!(denied.0[1], "☞ 무슨 말인지 모르겠어요. *^_^*");
    for command in ["값설정", "값삭제"] {
        let denied = storage
            .execute(command, &mut non_admin, "", None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    }

    let usage = storage
        .execute("전음", &mut body, "대상", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [대상] [내용] 전음(/)"]);

    set_precomputed_tell_players(vec![TellPlayerSnapshot::new(
        "hidden-token".into(),
        "숨은대상".into(),
        true,
        true,
        "",
        1,
        1,
        1,
        1,
        1,
        false,
    )]);
    let hidden = storage
        .execute("전음", &mut body, "숨은대상 말", None, None, None)
        .unwrap();
    assert_eq!(hidden.0, vec!["☞ 전음이 전달될만한 상대가 없어요. ^^"]);

    set_precomputed_tell_players(vec![TellPlayerSnapshot::new(
        "refuse-token".into(),
        "거부대상".into(),
        true,
        false,
        "전음거부 1",
        1,
        1,
        1,
        1,
        1,
        false,
    )]);
    let refused = storage
        .execute("전음", &mut body, "거부대상 말", None, None, None)
        .unwrap();
    assert_eq!(refused.0, vec!["☞ 전음 거부중이에요. ^^"]);

    set_precomputed_tell_players(vec![TellPlayerSnapshot::new(
        "self-token".into(),
        sender.clone(),
        true,
        false,
        "전음거부 0\n엘피출력 0",
        1,
        30,
        40,
        5,
        9,
        true,
    )]);
    let blocked = storage
        .execute("전음", &mut body, &format!("{sender} 말"), None, None, None)
        .unwrap();
    assert_eq!(
        blocked.0,
        vec!["☞ 이지역에서는 어떠한 통신도 불가능합니다."]
    );
    get_world_state()
        .read()
        .unwrap()
        .room_cache
        .get_room_cached(&zone, "1")
        .unwrap()
        .write()
        .unwrap()
        .properties
        .clear();

    let sent = storage
        .execute(
            "전음",
            &mut body,
            &format!("{sender}  여러   단어 "),
            None,
            None,
            None,
        )
        .unwrap();
    let tag = "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] ";
    assert!(matches!(
        sent.1,
        Some(CommandResult::Tell {
            ref target_token,
            ref sender_output,
            ref recipient_output,
            ref history_line,
        }) if target_token == "self-token"
            && sender_output == &format!("{tag}{sender}에게 보냄 : 여러 단어 \r\n")
            && recipient_output == &format!(
                "\r\n{tag}{sender} : 여러 단어 \r\n\r\n\x1b[0;37;40m[ 30/40, 5/9 ] \r\n"
            )
            && history_line == &format!("{tag}{sender} : 여러 단어 ")
    ));

    set_precomputed_tell_players(Vec::new());
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&sender);
    world.get_room_attrs_mut(&zone, "1").clear();
    drop(world);
    let _ = std::fs::remove_dir_all(zone_path);
}
