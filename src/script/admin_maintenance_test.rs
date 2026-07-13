use super::*;
#[test]
fn cleanup_command_normalizes_input_and_uses_all_connected_players() {
    use crate::command::handler::CommandResult;

    let mut body = Body::new();
    body.set("관리자등급", 1000_i64);
    set_precomputed_connected_names(vec![Dynamic::from("비활성대상")]);
    let storage = ScriptStorage::default();

    let missing = storage
        .execute("정리", &mut body, "없는대상", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["없는대상"]);
    assert!(missing.1.is_none());

    let whitespace = storage
        .execute("정리", &mut body, " ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법: [사용자명] 정리"]);
    assert!(whitespace.1.is_none());

    let found = storage
        .execute("정리", &mut body, "비활성대상", None, None, None)
        .unwrap();
    assert_eq!(found.0, vec!["비활성대상", "☞ 정리하였습니다. *^_^*"]);
    assert!(matches!(
        found.1,
        Some(CommandResult::Kick { ref target_name, ref reason })
            if target_name == "비활성대상" && reason == "정리 명령"
    ));
    clear_precomputed_all_online();
}

#[test]
fn cleanup_rejects_non_admin_before_echoing_target() {
    let mut body = Body::new();
    body.set("관리자등급", 999_i64);
    set_precomputed_connected_names(vec![Dynamic::from("정리대상")]);

    let output = ScriptStorage::default()
        .execute("정리", &mut body, "정리대상", None, None, None)
        .unwrap();

    clear_precomputed_all_online();
    assert_eq!(output.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert!(output.1.is_none());
}

#[test]
fn admin_maintenance_commands_match_python_permissions_silence_and_buffers() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "관리유틸회귀");
    body.set("관리자등급", 999_i64);

    for command in ["기연리스트", "청소", "리부팅", "업데이트"] {
        let denied = storage
            .execute(command, &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(
            denied.0,
            vec!["☞ 무슨 말인지 모르겠어요. *^_^*"],
            "{command} permission text"
        );
        assert!(denied.1.is_none());
    }

    body.set("관리자등급", 1000_i64);
    let oneitems = storage
        .execute("기연리스트", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(oneitems.0.len(), 1);
    assert!(oneitems.0[0].starts_with("[단일아이템인덱스]\r\n"));
    for block in oneitems.0[0].split('#').skip(1) {
        assert!(block.contains("\r\n:"));
        assert!(block.ends_with("\r\n\r\n"));
    }

    let cleaned = storage
        .execute("청소", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(cleaned.0, vec!["* 청소중입니다. 0"]);

    for argument in ["", "무시", "  어떤 값  "] {
        let reboot = storage
            .execute("리부팅", &mut body, argument, None, None, None)
            .unwrap();
        assert!(reboot.0.is_empty(), "Python reboot has no success output");
        assert!(matches!(reboot.1, Some(CommandResult::Reboot)));
    }

    let update_usage = storage
        .execute("업데이트", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(
        update_usage.0,
        vec!["* 명령어, 무림별호, 도움말, 무공, 표현, 도우미, 메인설정, 스크립트 중에 선택하세요"]
    );
    let unknown = storage
        .execute("업데이트", &mut body, " 없는종류 ", None, None, None)
        .unwrap();
    assert!(
        unknown.0.is_empty(),
        "Python update has no final else output"
    );
}

#[test]
fn transparency_and_save_commands_match_python_state_output_and_requests() {
    let suffix = std::process::id();
    let name = format!("저장투명회귀-{suffix}");
    let path = format!("data/user/{name}.json");
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("관리자등급", 999_i64);
    body.set("투명상태", 0_i64);

    let before = chrono::Utc::now().timestamp();
    let denied = storage
        .execute("투명", &mut body, "무시", None, None, None)
        .unwrap();
    let after = chrono::Utc::now().timestamp();
    assert_eq!(denied.0.len(), 2);
    let timestamp = denied.0[0].parse::<i64>().unwrap();
    assert!((before..=after).contains(&timestamp));
    assert_eq!(denied.0[1], "☞ 무슨 말인지 모르겠어요. *^_^*");
    assert_eq!(body.get_int("투명상태"), 0);

    body.set("관리자등급", 1000_i64);
    let hidden = storage
        .execute("투명", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(hidden.0[1], "☞ 투명상태가 되었습니다");
    assert_eq!(body.get_int("투명상태"), 1);
    let visible = storage
        .execute("투명", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(visible.0[1], "☞ 투명상태가 해제되었습니다");
    assert_eq!(body.get_int("투명상태"), 0);

    let saved = storage
        .execute("저장", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(saved.0, vec!["* 저장 되었습니다."]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["사용자오브젝트"]["이름"], name);

    body.set("관리자등급", 1999_i64);
    let save_all_denied = storage
        .execute("모두저장", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(save_all_denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert!(!take_save_all_request(&mut body));
    body.set("관리자등급", 2000_i64);
    let save_all = storage
        .execute("모두저장", &mut body, "어떤 인자도 무시", None, None, None)
        .unwrap();
    assert!(save_all.0.is_empty(), "Python 모두저장은 성공 문구가 없음");
    assert!(take_save_all_request(&mut body));

    let _ = std::fs::remove_file(path);
}

#[test]
fn finish_all_queues_every_other_active_player_in_python_connection_order() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("모두끝관리자-{suffix}");
    let first = format!("모두끝첫째-{suffix}");
    let second = format!("모두끝둘째-{suffix}");
    let zone = format!("모두끝존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&admin, &first, &second] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
        }
    }
    // Production installs only ACTIVE clients in this Python Client.players
    // view, retaining connection insertion order.
    let online = [&first, &admin, &second]
        .into_iter()
        .map(|name| {
            let mut map = rhai::Map::new();
            map.insert("이름".into(), Dynamic::from(name.to_string()));
            Dynamic::from(map)
        })
        .collect();
    set_precomputed_all_online(online);
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1999_i64);
    let denied = ScriptStorage::default()
        .execute("모두끝", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert!(take_force_command_request(&mut body).is_empty());

    body.set("관리자등급", 2000_i64);
    let result = ScriptStorage::default()
        .execute("모두끝", &mut body, "어떤 인자도 무시", None, None, None)
        .unwrap();
    assert!(result.0.is_empty());
    assert_eq!(
        take_force_command_request(&mut body),
        vec![(first.clone(), "끝".into()), (second.clone(), "끝".into())]
    );

    clear_precomputed_all_online();
    let mut world = get_world_state().write().unwrap();
    for name in [&admin, &first, &second] {
        world.remove_player_position(name);
    }
}

#[test]
fn summon_all_reports_each_same_room_player_and_queues_each_remote_player() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin = format!("모두소환관리자-{suffix}");
    let same = format!("모두소환동실-{suffix}");
    let remote = format!("모두소환원격-{suffix}");
    let zone = format!("모두소환존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&admin, &same] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
        }
        world.set_player_position(&remote, PlayerPosition::new(zone.clone(), "2".into()));
    }
    let online = [&admin, &same, &remote]
        .into_iter()
        .map(|name| {
            let mut player = rhai::Map::new();
            player.insert("이름".into(), Dynamic::from(name.clone()));
            Dynamic::from(player)
        })
        .collect();
    set_precomputed_all_online(online);
    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1999_i64);
    let denied = ScriptStorage::default()
        .execute("모두소환", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert!(take_summon_player_request(&mut body).is_empty());

    body.set("관리자등급", 2000_i64);
    let result = ScriptStorage::default()
        .execute("모두소환", &mut body, "어떤 인자도 무시", None, None, None)
        .unwrap();
    // Python includes the issuing administrator in Client.players.
    assert_eq!(result.0, vec!["☞ 같은 곳이에요. ^^", "☞ 같은 곳이에요. ^^"]);
    assert_eq!(
        take_summon_player_request(&mut body),
        vec![(remote.clone(), zone.clone(), "1".to_string())]
    );

    clear_precomputed_all_online();
    let mut world = get_world_state().write().unwrap();
    for name in [&admin, &same, &remote] {
        world.remove_player_position(name);
    }
}
