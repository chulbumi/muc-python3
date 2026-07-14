use super::*;
#[test]
fn test_nickname_command_rejects_legacy_duplicate() {
    if !Path::new("data/config/nickname.json").exists() {
        return;
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "별호중복검사");
    body.set("무림별호", "");
    body.set("이벤트설정리스트", "무림별호설정");

    let (output, special) = storage
        .execute("무림별호", &mut body, "감정노동자", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 다른 무림인이 사용중인 별호입니다. ^^"]);
    assert!(special.is_none());
    assert_eq!(body.get_string("무림별호"), "");
}
#[test]
fn test_nickname_command_usage_path_executes() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("무림별호"));
    let mut body = Body::new();
    body.set("이름", "별호검사");

    let (output, special) = storage
        .execute("무림별호", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 사용법: [별호이름] 무림별호"]);
    assert!(special.is_none());
}

#[test]
fn nickname_success_sends_python_global_departure_arrival_wires_with_prompts() {
    let suffix = std::process::id();
    let actor = format!("별호성공자-{suffix}");
    let source_observer = format!("별호출발목격-{suffix}");
    let destination_observer = format!("별호도착목격-{suffix}");
    let nickname = format!("별호{suffix}");
    let source_zone = format!("별호출발존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&actor, PlayerPosition::new(source_zone.clone(), "1".into()));
        world.set_player_position(
            &source_observer,
            PlayerPosition::new(source_zone, "1".into()),
        );
        world.set_player_position(
            &destination_observer,
            PlayerPosition::new("사용자맵".into(), actor.clone()),
        );
    }
    let online = [
        (&source_observer, 31_i64, 62_i64),
        (&destination_observer, 41, 82),
    ]
    .into_iter()
    .map(|(name, hp, max_hp)| {
        let mut player = rhai::Map::new();
        player.insert("이름".into(), Dynamic::from(name.clone()));
        player.insert("show_prompt".into(), Dynamic::from(true));
        player.insert("현재체력".into(), Dynamic::from(hp));
        player.insert("현재최고체력".into(), Dynamic::from(max_hp));
        player.insert("현재내공".into(), Dynamic::from(7_i64));
        player.insert("현재최고내공".into(), Dynamic::from(9_i64));
        Dynamic::from(player)
    })
    .collect();
    set_precomputed_all_online(online);

    let mut body = Body::new();
    body.set("이름", actor.as_str());
    body.set("이벤트설정리스트", "무림별호설정\n무림별호 사파");
    let result = ScriptStorage::default()
        .execute("무림별호", &mut body, &nickname, None, None, None)
        .unwrap();
    assert_eq!(body.get_string("무림별호"), nickname);
    assert_eq!(body.get_string("성격"), "사파");
    assert_eq!(body.get_string("귀환지맵"), format!("사용자맵:{actor}"));
    let sends = match result.1 {
        Some(CommandResult::OutputAndSendToUsers(_, sends))
        | Some(CommandResult::SendToUsers(sends)) => sends,
        other => panic!("unexpected nickname deliveries: {other:?}"),
    };
    assert_eq!(sends.len(), 4, "global two + departure one + arrival one");
    let source_wires = sends
        .iter()
        .filter(|(name, _)| name == &source_observer)
        .map(|(_, wire)| wire)
        .collect::<Vec<_>>();
    assert_eq!(source_wires.len(), 2);
    assert!(source_wires.iter().all(|wire| {
        wire.starts_with(&format!("{}\r\n", RAW_USER_MESSAGE_PREFIX))
            && wire.ends_with("\r\n\r\n\x1b[0;37;40m[ 31/62, 7/9 ] ")
    }));
    let destination_wires = sends
        .iter()
        .filter(|(name, _)| name == &destination_observer)
        .map(|(_, wire)| wire)
        .collect::<Vec<_>>();
    assert_eq!(destination_wires.len(), 2);
    assert!(destination_wires.iter().all(|wire| {
        wire.starts_with(&format!("{}\r\n", RAW_USER_MESSAGE_PREFIX))
            && wire.ends_with("\r\n\r\n\x1b[0;37;40m[ 41/82, 7/9 ] ")
    }));

    clear_precomputed_all_online();
    crate::world::nickname::nickname_release(&nickname, &actor);
    let mut world = get_world_state().write().unwrap();
    for name in [&actor, &source_observer, &destination_observer] {
        world.remove_player_position(name);
    }
    drop(world);
    let _ = std::fs::remove_file(format!("data/map/사용자맵/{actor}.json"));
    let _ = std::fs::remove_file(format!("data/user/{actor}.json"));
}

#[test]
fn head_and_tail_commands_match_python_character_limit_state_and_room_snapshot() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    let name = format!("꼬리말시험-{}", std::process::id());
    let path = format!("data/user/{name}.json");
    body.set("이름", name.as_str());

    let usage = storage
        .execute("꼬리말", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [내용] 꼬리말"]);
    assert_eq!(body.get_string("꼬리말"), "");

    let spaces = storage
        .execute("꼬리말", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(spaces.0, vec!["☞ 사용법: [내용] 꼬리말"]);
    assert_eq!(body.get_string("꼬리말"), "");

    let twenty = "가".repeat(20);
    let accepted = storage
        .execute("꼬리말", &mut body, &twenty, None, None, None)
        .unwrap();
    assert_eq!(accepted.0, vec!["☞ 꼬리말을 설정 하였습니다."]);
    assert_eq!(body.get_string("꼬리말"), twenty);
    let snapshot = build_room_view_player_snapshot(&body)
        .try_cast::<rhai::Map>()
        .unwrap();
    assert_eq!(snapshot["tail"].clone().into_string().unwrap(), twenty);

    let rejected = storage
        .execute("꼬리말", &mut body, &"나".repeat(21), None, None, None)
        .unwrap();
    assert_eq!(rejected.0, vec!["☞ 너무 깁니다."]);
    assert_eq!(body.get_string("꼬리말"), twenty);

    let ansi = "\x1b[31m붉음\x1b[0m";
    let colored = storage
        .execute("꼬리말", &mut body, ansi, None, None, None)
        .unwrap();
    assert_eq!(colored.0, vec!["☞ 꼬리말을 설정 하였습니다."]);
    assert_eq!(body.get_string("꼬리말"), ansi);
    assert!(save_body_to_json(&mut body, &path));
    let mut loaded = Body::new();
    assert!(load_body_from_json(&mut loaded, &path));
    assert_eq!(loaded.get_string("꼬리말"), ansi);

    let removed = storage
        .execute("꼬리말제거", &mut body, "무시되는 입력", None, None, None)
        .unwrap();
    assert_eq!(removed.0, vec!["☞ 꼬리말을 제거 하였습니다."]);
    assert_eq!(body.get_string("꼬리말"), "");

    let head_value = "앞".repeat(20);
    let head = storage
        .execute("머리말", &mut body, &head_value, None, None, None)
        .unwrap();
    assert_eq!(head.0, vec!["☞ 머리말을 설정 하였습니다."]);
    assert_eq!(body.get_string("머리말"), head_value);
    let head_spaces = storage
        .execute("머리말", &mut body, "  ", None, None, None)
        .unwrap();
    assert_eq!(head_spaces.0, vec!["☞ 사용법: [내용] 머리말"]);
    assert_eq!(body.get_string("머리말"), head_value);
    let head_too_long = storage
        .execute("머리말", &mut body, &"머".repeat(21), None, None, None)
        .unwrap();
    assert_eq!(head_too_long.0, vec!["☞ 너무 깁니다."]);
    assert_eq!(body.get_string("머리말"), head_value);
    let snapshot = build_room_view_player_snapshot(&body)
        .try_cast::<rhai::Map>()
        .unwrap();
    assert_eq!(snapshot["head"].clone().into_string().unwrap(), head_value);

    assert!(save_body_to_json(&mut body, &path));
    let mut head_loaded = Body::new();
    assert!(load_body_from_json(&mut head_loaded, &path));
    assert_eq!(head_loaded.get_string("머리말"), head_value);

    let head_removed = storage
        .execute("머리말제거", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(head_removed.0, vec!["☞ 머리말을 제거 하였습니다."]);
    assert_eq!(body.get_string("머리말"), "");
    let removed_again = storage
        .execute(
            "머리말제거",
            &mut body,
            "어떤 인자도 무시",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(removed_again.0, vec!["☞ 머리말을 제거 하였습니다."]);
    assert_eq!(body.get_string("머리말"), "");
    assert!(save_body_to_json(&mut body, &path));
    let mut removed_loaded = Body::new();
    assert!(load_body_from_json(&mut removed_loaded, &path));
    assert_eq!(removed_loaded.get_string("머리말"), "");
    let _ = std::fs::remove_file(path);
}
