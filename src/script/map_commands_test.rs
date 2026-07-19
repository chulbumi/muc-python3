use super::*;
#[test]
fn exit_admin_commands_toggle_and_persist_like_python() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player_name = format!("출구회귀-{suffix}");
    let zone = format!("출구회귀존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    let room_path = room_dir.join("1.json");
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::write(
        &room_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "맵정보": {
                "이름": "출구 시험방", "존이름": zone,
                "설명": ["시험방"], "출구": ["동    2  ", "비밀$ 3", "비밀 4"], "몹": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.set_player_position(
            &player_name,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
    }
    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("관리자등급", 1000_i64);
    let storage = ScriptStorage::default();

    let spaced_hide = storage
        .execute("출구숨김", &mut body, " 동 ", None, None, None)
        .unwrap();
    assert_eq!(spaced_hide.0, vec!["☞ 출구가 숨겨졌습니다."]);
    let restored_after_spaced = storage
        .execute("출구숨김", &mut body, "동", None, None, None)
        .unwrap();
    assert_eq!(restored_after_spaced.0, vec!["☞ 출구가 드러났습니다."]);
    let spaced_remove = storage
        .execute("출구제거", &mut body, " ", None, None, None)
        .unwrap();
    assert_eq!(spaced_remove.0, vec!["☞ 사용법: [출구] 출구숨김"]);

    let hidden = storage
        .execute("출구숨김", &mut body, "동", None, None, None)
        .unwrap();
    assert_eq!(hidden.0, vec!["☞ 출구가 숨겨졌습니다."]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
    assert!(json["맵정보"]["출구"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "동$ 2  "));

    let shown = storage
        .execute("출구숨김", &mut body, "동", None, None, None)
        .unwrap();
    assert_eq!(shown.0, vec!["☞ 출구가 드러났습니다."]);

    let duplicate_hidden = storage
        .execute("출구숨김", &mut body, "비밀", None, None, None)
        .unwrap();
    assert_eq!(duplicate_hidden.0, vec!["☞ 출구가 숨겨졌습니다."]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
    let exits = json["맵정보"]["출구"].as_array().unwrap();
    assert!(exits.iter().any(|value| value == "비밀 3"));
    assert!(exits.iter().any(|value| value == "비밀$ 4"));
    let duplicate_restored = storage
        .execute("출구숨김", &mut body, "비밀", None, None, None)
        .unwrap();
    assert_eq!(duplicate_restored.0, vec!["☞ 출구가 드러났습니다."]);

    let wandered = storage
        .execute("맴돌이", &mut body, "비밀", None, None, None)
        .unwrap();
    assert_eq!(wandered.0, vec!["☞ 출구가 맴돌이 되었습니다."]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
    assert!(json["맵정보"]["출구"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "비밀$ 3"));
    assert!(json["맵정보"]["출구"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "비밀 4"));
    let destination = get_world_state()
        .read()
        .unwrap()
        .room_cache
        .get_room_cached(&zone, "1")
        .unwrap()
        .read()
        .unwrap()
        .exits["비밀"]
        .destination(&zone);
    assert_eq!(destination, Some((zone.clone(), "1".to_string())));

    let spaced_wander = storage
        .execute("맴돌이", &mut body, " 비밀 ", None, None, None)
        .unwrap();
    assert_eq!(spaced_wander.0, vec!["☞ 출구가 맴돌이 되었습니다."]);
    let whitespace_wander = storage
        .execute("맴돌이", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace_wander.0, vec!["☞ 사용법: [출구] 맴돌이"]);

    let usage = storage
        .execute("출구제거", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [출구] 출구숨김"]);
    let removed = storage
        .execute("출구제거", &mut body, "동", None, None, None)
        .unwrap();
    assert_eq!(removed.0, vec!["☞ 출구가 제거되었습니다."]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
    assert!(!json["맵정보"]["출구"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().is_some_and(|v| v.starts_with("동 "))));

    let _ = std::fs::remove_dir_all(room_dir);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player_name);
}
#[test]
fn test_track_command_matches_python_messages_and_first_room() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("추적"));
    let mut body = Body::new();
    body.set("이름", "추적검사");

    let (output, _) = storage
        .execute("추적", &mut body, "청강석 낙양성", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 1000i64);
    for line in ["", "청강석"] {
        let (output, _) = storage
            .execute("추적", &mut body, line, None, None, None)
            .unwrap();
        assert_eq!(output, vec!["몹이름 존이름 추적"]);
    }

    let (output, _) = storage
        .execute(
            "추적",
            &mut body,
            "청강석 __존재하지_않는_존__",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(output, vec!["그런 존은 없어요!"]);

    let (output, _) = storage
        .execute(
            "추적",
            &mut body,
            "__존재하지_않는_몹__ 낙양성",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(output, vec!["못찾았음"]);

    // Python loadAllMob 순서에서 청강석의 첫 배치 방은 낙양성:4004이다.
    let (output, _) = storage
        .execute("추적", &mut body, "청강석 낙양성", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["4004"]);
}
#[test]
fn test_current_body_position_accepts_colon_and_legacy_slash() {
    let mut body = Body::new();
    body.set("이름", "위치형식검사전용");
    body.set("위치", "낙양성:42");
    assert_eq!(
        current_body_position(&body),
        Some(("낙양성".to_string(), "42".to_string()))
    );

    body.set("위치", "");
    body.set("현재방", "하북성/3001");
    assert_eq!(
        current_body_position(&body),
        Some(("하북성".to_string(), "3001".to_string()))
    );
}
#[test]
fn return_home_command_matches_python_guard_order_and_success_room_output() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("귀환회귀-{suffix}");
    // Difficulty zones strip trailing ASCII digits when loading map JSON.
    // Keep this synthetic zone from accidentally looking like one.
    let zone = format!("귀환회귀존-{suffix}가");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for (room, name, properties) in [
        ("1", "출발지", vec!["귀환금지"]),
        ("2", "귀환지", Vec::<&str>::new()),
    ] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보":{
                "이름": name, "존이름": zone, "설명": [format!("{name} 설명")],
                "맵속성": properties, "출구": []
            }})
            .to_string(),
        )
        .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.room_cache.get_room(&zone, "2").unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("귀환지맵", format!("{zone}:2"));
    body.set("체력", 100_i64);
    body.set("최고체력", 100_i64);

    let forbidden = storage
        .execute("귀환", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(forbidden.0, vec!["☞ 이곳에선 귀환하실 수 없어요. ^^"]);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&player)
            .unwrap()
            .room,
        "1"
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
    body.act = crate::player::ActState::Fight;
    let fighting = storage
        .execute("귀환", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(fighting.0, vec!["☞ 지금은 귀환할 상황이 아니에요. ^^"]);

    body.act = crate::player::ActState::Stand;
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "2".to_string()));
    let same = storage
        .execute("귀환", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(same.0, vec!["☞ 같은 자리에요. ^^"]);

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
    let old_observer = format!("귀환출발목격-{suffix}");
    let new_observer = format!("귀환도착목격-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            &old_observer,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
        world.set_player_position(
            &new_observer,
            PlayerPosition::new(zone.clone(), "2".to_string()),
        );
    }
    let observer = |name: &str, hp: i64, max_hp: i64, mp: i64, max_mp: i64| {
        let mut value = rhai::Map::new();
        value.insert("이름".into(), Dynamic::from(name.to_string()));
        value.insert("show_prompt".into(), Dynamic::from(true));
        value.insert("현재체력".into(), Dynamic::from(hp));
        value.insert("현재최고체력".into(), Dynamic::from(max_hp));
        value.insert("현재내공".into(), Dynamic::from(mp));
        value.insert("현재최고내공".into(), Dynamic::from(max_mp));
        Dynamic::from(value)
    };
    set_precomputed_all_online(vec![
        observer(&old_observer, 11, 22, 3, 4),
        observer(&new_observer, 55, 66, 7, 8),
    ]);
    body.set("설정상태", "간략설명 1\n나침반제거 1");
    let moved = storage
        .execute("귀환", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        moved.0[0],
        "당신이 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'"
    );
    assert!(
        moved.0.iter().any(|line| line.contains("귀환지")),
        "{moved:?}"
    );
    assert!(
        moved
            .0
            .iter()
            .any(|line| line.trim_start().starts_with("[출구] :")),
        "{moved:?}"
    );
    assert!(
        !moved.0.iter().any(|line| line.contains("귀환지 설명")),
        "{moved:?}"
    );
    assert!(!moved.0.iter().any(|line| line.contains('○')), "{moved:?}");
    let sends = match moved.1 {
        Some(CommandResult::OutputAndSendToUsers(_, sends)) => sends,
        other => panic!("expected return-home room deliveries, got {other:?}"),
    };
    assert_eq!(sends.len(), 2, "{sends:?}");
    assert_eq!(sends[0].0, old_observer);
    assert_eq!(
        sends[0].1,
        format!(
            "{}\r\n\x1b[1m{}\x1b[0;37m{} 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'\r\n\r\n\x1b[0;37;40m[ 11/22, 3/4 ] ",
            RAW_USER_MESSAGE_PREFIX,
            player,
            crate::hangul::han_iga(&player),
        )
    );
    assert_eq!(sends[1].0, new_observer);
    assert_eq!(
        sends[1].1,
        format!(
            "{}\r\n\x1b[1m{}\x1b[0;37m{} 하늘에서 사뿐히 내려 앉습니다. '척~~~'\r\n\r\n\x1b[0;37;40m[ 55/66, 7/8 ] ",
            RAW_USER_MESSAGE_PREFIX,
            player,
            crate::hangul::han_iga(&player),
        )
    );
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&player)
            .unwrap()
            .room,
        "2"
    );

    clear_precomputed_all_online();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.remove_player_position(&old_observer);
    world.remove_player_position(&new_observer);
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn summon_admin_move_enforces_python_destination_guards_but_direct_move_bypasses_them() {
    use crate::player::ActState;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("제한이동검사-{suffix}");
    let blocker = format!("제한방점유자-{suffix}");
    let zone = format!("제한이동존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    let rooms = [
        ("1", serde_json::json!({})),
        ("2", serde_json::json!({"레벨상한": 9})),
        ("3", serde_json::json!({"레벨제한": 11})),
        ("4", serde_json::json!({"힘상한제한": 19})),
        ("5", serde_json::json!({"민첩상한제한": 29})),
        ("6", serde_json::json!({"맵속성": ["사파출입금지"]})),
        ("7", serde_json::json!({"맵속성": ["정파출입금지"]})),
        ("8", serde_json::json!({"방파주인": "다른방파"})),
        ("9", serde_json::json!({"맵속성": ["인원제한 1"]})),
    ];
    for (room, extra) in rooms {
        let mut info = serde_json::json!({
            "이름": format!("제한방{room}"), "존이름": zone,
            "설명": [], "출구": []
        });
        info.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보": info}).to_string(),
        )
        .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        for room in 1..=9 {
            world.room_cache.get_room(&zone, &room.to_string()).unwrap();
        }
        world.set_player_position(&name, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&blocker, PlayerPosition::new(zone.clone(), "9".into()));
    }
    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("관리자등급", 2000_i64);
    body.set("레벨", 10_i64);
    body.set("힘", 20_i64);
    body.set("민첩성", 30_i64);
    assert_eq!(
        crate::script::check_summon_destination(&body, &zone, "1"),
        "same_place",
        "관리자 이동.py returns before enterRoom for the current room"
    );
    assert_eq!(
        crate::script::check_event_summon_destination(&body, &zone, "1"),
        "",
        "event.py always calls enterRoom even when $위치이동 points at the current room"
    );
    let storage = ScriptStorage::default();
    let cases = [
        ("2", "선인", "강한 무형의 기운이 당신을 압박합니다."),
        ("3", "선인", "강한 무형의 기운이 당신을 압박합니다."),
        ("4", "선인", "강한 무형의 기운이 당신을 압박합니다."),
        ("5", "선인", "강한 무형의 기운이 당신을 압박합니다."),
        ("6", "사파", "☞ 사파는 출입할 수 없는 곳이라네!"),
        ("7", "정파", "☞ 정파는 출입할 수 없는 곳이라네!"),
        (
            "8",
            "선인",
            "☞ 그곳은 타 방파의 지역이므로 출입하실 수 없습니다.",
        ),
        (
            "9",
            "선인",
            "☞ 알 수 없는 무형의 기운이 당신을 가로막습니다. ^_^",
        ),
    ];
    for (room, personality, expected) in cases {
        body.set("성격", personality);
        body.act = ActState::Fight;
        body.temp_mut()
            .insert("_pvp_target".into(), Value::String("상대".into()));
        let blocked = storage
            .execute(
                "이동",
                &mut body,
                &format!("{zone}:{room}"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(blocked.0, vec![expected], "room {room}");
        assert_eq!(body.act, ActState::Stand, "clearTarget before enterRoom");
        assert!(!body.temp().contains_key("_pvp_target"));
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_player_position(&name)
                .unwrap()
                .room,
            "1"
        );
    }

    body.set("성격", "사파");
    let direct = storage
        .execute("이동동", &mut body, &format!("{zone}:6"), None, None, None)
        .unwrap();
    assert!(direct.0.is_empty());
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&name)
            .unwrap()
            .room,
        "6",
        "Python direct insertion bypasses enterRoom restrictions"
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&name);
    world.remove_player_position(&blocker);
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn direct_admin_move_variants_are_silent_and_clear_combat_like_python() {
    use crate::player::ActState;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("직접이동검사-{suffix}");
    let zone = format!("직접이동존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for room in ["1", "007"] {
        std::fs::write(
                directory.join(format!("{room}.json")),
                serde_json::json!({"맵정보":{"이름":format!("직접방{room}"),"존이름":zone,"설명":[],"출구":[]}})
                    .to_string(),
            )
            .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.room_cache.get_room(&zone, "007").unwrap();
        world.set_player_position(&name, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let storage = ScriptStorage::default();
    for (index, command) in ["이동이동", "이동동"].into_iter().enumerate() {
        let mut body = Body::new();
        body.set("이름", name.as_str());
        body.set("관리자등급", 2000_i64);
        body.act = ActState::Fight;
        let destination = if index == 0 { "007" } else { "1" };
        let result = storage
            .execute(
                command,
                &mut body,
                &format!("{zone}:{destination}"),
                None,
                None,
                None,
            )
            .unwrap();
        assert!(
            result.0.is_empty(),
            "Python direct move is silent for {command}"
        );
        assert_eq!(body.act, ActState::Stand);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_player_position(&name)
                .unwrap()
                .room,
            destination
        );
    }

    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("관리자등급", 2000_i64);
    body.act = crate::player::ActState::Fight;
    let same = storage
        .execute("이동동", &mut body, &format!("{zone}:1"), None, None, None)
        .unwrap();
    assert_eq!(same.0, vec!["☞ 같은 자리에요. ^^"]);
    assert_eq!(
        body.act,
        ActState::Fight,
        "Python returns before clearTarget"
    );

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&name);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn admin_and_engraved_moves_preserve_exact_string_room_index() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("정확이동검사-{suffix}");
    let zone = format!("정확이동존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for room in ["1", "007"] {
        std::fs::write(
                directory.join(format!("{room}.json")),
                serde_json::json!({"맵정보":{"이름":format!("방{room}"),"존이름":zone,"설명":[],"출구":[]}})
                    .to_string(),
            )
            .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.room_cache.get_room(&zone, "007").unwrap();
        world.set_player_position(&name, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("관리자등급", 1000_i64);
    let storage = ScriptStorage::default();

    let spaced = storage
        .execute(
            "이동",
            &mut body,
            &format!("  {zone}:007  "),
            None,
            None,
            None,
        )
        .unwrap();
    assert!(spaced.0.iter().any(|line| line.contains("방007")));
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&name)
            .unwrap()
            .room,
        "007"
    );

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&name, PlayerPosition::new(zone.clone(), "1".into()));

    let moved = storage
        .execute("이동", &mut body, &format!("{zone}:007"), None, None, None)
        .unwrap();
    assert!(moved.0.iter().any(|line| line.contains("방007")));
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&name)
            .unwrap()
            .room,
        "007"
    );

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&name, PlayerPosition::new(zone.clone(), "1".into()));
    body.set("위치각인", format!("{zone}:007"));
    let teleported = storage
        .execute("이형환위", &mut body, "비학천룡", None, None, None)
        .unwrap();
    assert!(teleported.0.iter().any(|line| line.contains("방007")));
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .get_player_position(&name)
            .unwrap()
            .room,
        "007"
    );

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&name);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn map_display_requires_visible_exit_and_marks_python_grid_directions() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("지도표시-{suffix}");
    let zone = format!("지도표시존-{suffix}가");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    let write_room = |room: &str, exits: Vec<&str>| {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보":{
                "이름": format!("지도시험{room}"), "존이름": zone,
                "설명": [], "출구": exits
            }})
            .to_string(),
        )
        .unwrap();
    };
    write_room("1", vec![]);
    write_room("2", vec!["비밀$ 3"]);
    write_room("3", vec![]);
    write_room("4", vec!["동 5", "위 6", "아래 7", "비밀$ 3"]);
    write_room("5", vec!["서 4"]);
    write_room("6", vec![]);
    write_room("7", vec![]);
    {
        let mut world = get_world_state().write().unwrap();
        for room in ["1", "2", "3", "4", "5", "6", "7"] {
            world.room_cache.get_room(&zone, room).unwrap();
        }
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());

    let no_exit = storage
        .execute("지도", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(no_exit.0, vec!["☞ 아무것도 보이지 않습니다."]);
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "2".to_string()));
    let hidden_only = storage
        .execute("지도", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(hidden_only.0, vec!["☞ 아무것도 보이지 않습니다."]);

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "4".to_string()));
    let shown = storage
        .execute("지도", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(shown.0.len(), 1);
    let rows = shown.0[0].split("\r\n").collect::<Vec<_>>();
    assert_eq!(rows.len(), 12);
    assert!(rows[5].contains("\x1b[1;33m↕\x1b[37;0m─○"), "{:?}", rows[5]);

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn map_command_uses_python_first_word_and_raw_hidden_exit_membership() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("맵탐색검사-{suffix}");
    // A trailing ASCII digit denotes Python difficulty-zone suffixing, so
    // keep the synthetic base-zone name non-numeric.
    let zone = format!("맵탐색존-{suffix}가");
    let dir = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&dir).unwrap();
    let write_room = |number: &str, exits: Vec<&str>| {
        std::fs::write(
            dir.join(format!("{number}.json")),
            serde_json::to_string_pretty(&serde_json::json!({
                "맵정보": {
                    "이름": format!("지도방{number}"),
                    "존이름": zone,
                    "설명": ["지도 탐색 시험"],
                    "출구": exits,
                    "몹": []
                }
            }))
            .unwrap(),
        )
        .unwrap();
    };
    write_room("1", vec!["동 2", "서 3", "비밀$ 4"]);
    write_room("2", vec!["서 1"]);
    write_room("3", vec!["동 1"]);
    write_room("4", vec!["서 1"]);

    {
        let mut world = get_world_state().write().unwrap();
        for room in ["1", "2", "3", "4"] {
            world.room_cache.get_room(&zone, room).unwrap();
        }
        let loaded = world.room_cache.get_room_cached(&zone, "1").unwrap();
        assert!(loaded.read().unwrap().exits.contains_key("동"));
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 2000_i64);

    let direct = python_map_explore(&body, "동")
        .into_iter()
        .map(|value| value.into_string().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(direct, vec!["서"]);

    let single_word = storage
        .execute("맵", &mut body, "동", None, None, None)
        .unwrap();
    assert_eq!(single_word.0, vec!["서;"]);

    let extra_words = storage
        .execute("맵", &mut body, "  동   뒤의말은무시  ", None, None, None)
        .unwrap();
    assert_eq!(extra_words.0, vec!["서;"]);

    let hidden_raw_name = storage
        .execute("맵", &mut body, "비밀$", None, None, None)
        .unwrap();
    assert_ne!(hidden_raw_name.0, vec!["☞ 그 방향으로는 갈수가 없어요!."]);

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    drop(world);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn map_command_checks_python_usage_before_missing_position() {
    let mut body = Body::new();
    body.set("이름", "맵순서검사");
    body.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();
    let usage = storage
        .execute("맵", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [제외할방향] 맵"]);
    let invisible = storage
        .execute("맵", &mut body, "동", None, None, None)
        .unwrap();
    assert_eq!(invisible.0, vec!["\r\n* 아무것도 보이지 않습니다.\r\n"]);
}

#[test]
fn location_engraving_stores_exact_room_index_and_normalizes_whitespace() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("위치각인검사-{suffix}");
    let zone = format!("위치각인존-{suffix}");
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&name, PlayerPosition::new(zone.clone(), "007".into()));
    let mut body = Body::new();
    body.set("이름", name.as_str());
    let storage = ScriptStorage::default();

    let whitespace = storage
        .execute("위치각인", &mut body, " ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법: [대상] 위치각인"]);
    assert!(body.get_string("위치각인").is_empty());

    let engraved = storage
        .execute("위치각인", &mut body, "비학천룡", None, None, None)
        .unwrap();
    assert_eq!(engraved.0, vec!["☞ 현재 위치가 각인되었습니다."]);
    assert_eq!(body.get_string("위치각인"), format!("{zone}:007"));

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&name);
}

#[test]
fn admin_move_variants_keep_python_permission_usage_and_failure_order() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();

    for command in ["이동", "이동이동", "이동동"] {
        let denied = storage
            .execute(command, &mut body, "없는존:1", None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    }

    body.set("관리자등급", 1000_i64);
    for command in ["이동", "이동이동"] {
        let usage = storage
            .execute(command, &mut body, "   ", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [존이름:맵번호] 이동"]);
        let missing = storage
            .execute(command, &mut body, "없는존:없는방", None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["* 이동 실패!!!"]);
    }

    let high_denied = storage
        .execute("이동동", &mut body, "없는존:1", None, None, None)
        .unwrap();
    assert_eq!(high_denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    body.set("관리자등급", 2000_i64);
    let usage = storage
        .execute("이동동", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [존이름:맵번호] 이동"]);
    let missing = storage
        .execute("이동동", &mut body, "콜론없음", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["* 이동 실패!!!"]);
}

#[test]
fn room_editor_and_delete_match_python_whitespace_path_and_memory_only_removal() {
    use crate::command::handler::{CommandResult, PendingInput};
    use crate::world::get_world_state;

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("관리자등급", 0_i64);
    assert_eq!(
        storage
            .execute("방제작", &mut body, "\t  ", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 사용법: [존이름] [방이름] 방제작"]
    );
    assert_eq!(
        storage
            .execute("방제작", &mut body, "시험존 시험방", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^"]
    );
    body.set("관리자등급", 1000_i64);
    let editor = storage
        .execute(
            "방제작",
            &mut body,
            "  시험존\t시험방   추가단어 ",
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
            && relative_path == "map/시험존/시험방.json" && lines.is_empty()
    ));

    let suffix = std::process::id();
    let zone = format!("방제거존-{suffix}가");
    let dir = std::path::Path::new("data/map").join(&zone);
    let path = dir.join("이름방.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &path,
        format!(r#"{{"맵정보":{{"이름":"삭제방","존이름":"{zone}","설명":[],"출구":[]}}}}"#),
    )
    .unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "이름방").unwrap();
        world
            .get_room_attrs_mut(&zone, "이름방")
            .insert("임시속성".into(), "메모리값".into());
    }
    body.set("관리자등급", 1999_i64);
    assert_eq!(
        storage
            .execute(
                "방제거",
                &mut body,
                &format!("{zone}:이름방"),
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );
    body.set("관리자등급", 2000_i64);
    for invalid in ["", "콜론없음", ":이름방", &format!("{zone}:")] {
        let output = storage
            .execute("방제거", &mut body, invalid, None, None, None)
            .unwrap();
        let expected = if invalid.is_empty() {
            "☞ 사용법: [방번호] 방제거"
        } else {
            "존재하지않는 방입니다."
        };
        assert_eq!(output.0, vec![expected]);
    }
    assert_eq!(
        storage
            .execute(
                "방제거",
                &mut body,
                &format!("{zone}:이름방"),
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["방이 제거되었습니다."]
    );
    assert!(path.exists(), "Python deletes only Room.Zones entry");
    assert!(get_world_state()
        .read()
        .unwrap()
        .room_cache
        .get_room_cached(&zone, "이름방")
        .is_none());
    assert!(!get_world_state()
        .read()
        .unwrap()
        .room_attrs
        .contains_key(&format!("{zone}:이름방")));
    assert_eq!(
        storage
            .execute(
                "방제거",
                &mut body,
                &format!("{zone}:이름방"),
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["존재하지않는 방입니다."]
    );
    let reloaded = get_world_state()
        .write()
        .unwrap()
        .room_cache
        .get_room(&zone, "이름방")
        .unwrap();
    assert_eq!(reloaded.read().unwrap().display_name, "삭제방");
    assert_eq!(
        storage
            .execute(
                "방제거",
                &mut body,
                &format!("{zone}:이름방"),
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["방이 제거되었습니다."]
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn room_description_command_uses_python_level_guard_and_current_room_input_state() {
    use crate::command::handler::{CommandResult, PendingInput};
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("방설명명령-{suffix}");
    let zone = format!("방설명명령존-{suffix}");
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 999_i64);
    let storage = ScriptStorage::default();
    assert_eq!(
        storage
            .execute("방설명", &mut body, "무시", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^"]
    );
    body.set("관리자등급", 1000_i64);
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "이름방".into()));
    let started = storage
        .execute("방설명", &mut body, "완전히 무시", None, None, None)
        .unwrap();
    assert!(
        started.0.is_empty(),
        "Python starts the editor with write()"
    );
    assert!(matches!(
        started.1,
        Some(CommandResult::RequestInput {
            ref prompt,
            state: PendingInput::RoomDescription { zone: ref z, ref room, ref lines }
        }) if prompt == "방 설명 작성을 마치시려면 '.' 를 입력하세요.\r\n:"
            && z == &zone
            && room == "이름방" && lines.is_empty()
    ));
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
}

#[test]
fn room_name_updates_python_live_room_and_json_immediately() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("방이름명령-{suffix}");
    let zone = format!("방이름명령존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    let path = directory.join("이름방.json");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        &path,
        serde_json::json!({"맵정보": {
            "이름": "옛 방", "존이름": zone, "설명": [], "출구": [], "몹": []
        }})
        .to_string(),
    )
    .unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "이름방").unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "이름방".into()));
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 999_i64);
    assert_eq!(
        storage
            .execute("방이름", &mut body, "새 방", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
    );
    body.set("관리자등급", 1000_i64);
    assert_eq!(
        storage
            .execute("방이름", &mut body, " \t ", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 사용법: [이름] 방이름"]
    );
    assert_eq!(
        storage
            .execute("방이름", &mut body, "  새 방 이름  ", None, None, None)
            .unwrap()
            .0,
        vec!["방이 이름이 변경 되었습니다."]
    );
    let world = get_world_state().read().unwrap();
    assert_eq!(
        world
            .room_cache
            .get_room_cached(&zone, "이름방")
            .unwrap()
            .read()
            .unwrap()
            .name,
        "새 방 이름"
    );
    assert_eq!(
        world.room_attrs[&format!("{zone}:이름방")]["이름"],
        "새 방 이름"
    );
    drop(world);
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["맵정보"]["이름"], "새 방 이름");
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn wander_exit_is_runtime_only_and_matches_hidden_exact_name() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("맴돌이회귀-{suffix}");
    let zone = format!("맴돌이존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    let room_path = room_dir.join("1.json");
    std::fs::create_dir_all(&room_dir).unwrap();
    let original = serde_json::json!({
        "맵정보": {
            "이름": "맴돌이 시험방", "존이름": zone,
            "설명": ["시험방"], "출구": ["샛길$ 2", "동 3"], "몹": []
        }
    });
    std::fs::write(&room_path, serde_json::to_string_pretty(&original).unwrap()).unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 999_i64);
    let denied = storage
        .execute("맴돌이", &mut body, "샛길", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 1000_i64);
    let usage = storage
        .execute("맴돌이", &mut body, " \t ", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [출구] 맴돌이"]);
    let missing = storage
        .execute("맴돌이", &mut body, "샛", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 출구가 없습니다."]);

    let changed = storage
        .execute("맴돌이", &mut body, " 샛길 ", None, None, None)
        .unwrap();
    assert_eq!(changed.0, vec!["☞ 출구가 맴돌이 되었습니다."]);
    let room = get_world_state()
        .read()
        .unwrap()
        .room_cache
        .get_room_cached(&zone, "1")
        .unwrap();
    let room = room.read().unwrap();
    assert!(room.exits["샛길"].hidden);
    assert_eq!(
        room.exits["샛길"].destination(&zone),
        Some((zone.clone(), "1".into()))
    );
    assert_eq!(
        room.exits["동"].destination(&zone),
        Some((zone.clone(), "3".into()))
    );
    drop(room);

    let persisted: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
    assert_eq!(persisted, original);

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_dir_all(room_dir);
}

#[test]
fn map_explorer_never_reveals_hidden_compass_exit() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("숨김맵회귀-{suffix}");
    let zone = format!("숨김맵존-{suffix}가");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for (room, exits) in [
        ("1", vec!["동$ 2", "서 3"]),
        ("2", vec!["서$ 1"]),
        ("3", vec!["동 1"]),
    ] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보": {
                "이름": format!("숨김맵방{room}"), "존이름": zone,
                "설명": [], "출구": exits, "몹": []
            }})
            .to_string(),
        )
        .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        for room in ["1", "2", "3"] {
            world.room_cache.get_room(&zone, room).unwrap();
        }
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 1999_i64);
    let denied = storage
        .execute("맵", &mut body, "서", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 2000_i64);
    // Excluding the only visible compass exit leaves only `동$`. Python
    // filters that hidden raw name out and refuses to draw a route.
    let no_visible_branch = storage
        .execute("맵", &mut body, "서", None, None, None)
        .unwrap();
    assert_eq!(no_visible_branch.0, vec!["☞ 그 방향으로는 갈수가 없어요!."]);

    // The raw hidden name is a valid exclusion argument, but it still cannot
    // become an explored direction; the remaining visible west branch wins.
    let exclude_hidden = storage
        .execute("맵", &mut body, "동$", None, None, None)
        .unwrap();
    assert_eq!(exclude_hidden.0, vec!["서;"]);

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn map_explorer_uses_python_random_destination_instead_of_first_only() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("다중맵회귀-{suffix}");
    let zone = format!("다중맵존-{suffix}가");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for (room, exits) in [
        ("1", vec!["동 2 3", "서 4"]),
        ("2", vec!["서 1", "북 5"]),
        ("3", vec!["서 1", "남 6"]),
        ("4", vec!["동 1"]),
        ("5", vec!["남 2"]),
        ("6", vec!["북 3"]),
    ] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보": {
                "이름": format!("다중맵방{room}"), "존이름": zone,
                "설명": [], "출구": exits, "몹": []
            }})
            .to_string(),
        )
        .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        for room in ["1", "2", "3", "4", "5", "6"] {
            world.room_cache.get_room(&zone, room).unwrap();
        }
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());

    let first = super::movement::python_map_explore_with_roller(&body, "서", &mut |_| 0)
        .into_iter()
        .map(|direction| direction.into_string().unwrap())
        .collect::<Vec<_>>();
    let last =
        super::movement::python_map_explore_with_roller(&body, "서", &mut |length| length - 1)
            .into_iter()
            .map(|direction| direction.into_string().unwrap())
            .collect::<Vec<_>>();
    assert!(first.contains(&"북".to_string()), "{first:?}");
    assert!(last.contains(&"남".to_string()), "{last:?}");
    assert_ne!(first, last);

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_dir_all(directory);
}
