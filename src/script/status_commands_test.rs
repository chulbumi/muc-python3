use super::*;
#[test]
fn ansi_command_is_python_hp_bar_not_a_persisted_toggle() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "안시막대검사");
    body.set("체력", 45_i64);
    body.set("최고체력", 100_i64);
    body.set("안시", 77_i64);
    let shown = storage
        .execute("안시", &mut body, "꺼기", None, None, None)
        .unwrap();
    assert_eq!(shown.0, vec!["\x1b[32m━━━━\x1b[37m━━━━━━"]);
    assert_eq!(
        body.get_int("안시"),
        77,
        "Python ignores the command argument"
    );

    body.set("체력", -1_i64);
    let negative = storage
        .execute("안시", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(negative.0, vec!["\x1b[32m\x1b[37m━━━━━━━━━━━"]);

    body.set("체력", 150_i64);
    let overfull = storage
        .execute("안시", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(overfull.0, vec!["\x1b[32m━━━━━━━━━━━━━━━\x1b[37m"]);
}
#[test]
fn test_recover_command_updates_canonical_hp_mp_attributes() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("회복"));
    let mut body = Body::new();
    body.set("이름", "회복검사");
    body.set("관리자등급", 1000i64);
    body.set("최고체력", 321i64);
    body.set("최고내공", 123i64);
    body.set("체력", 1i64);
    body.set("내공", 2i64);

    let result = storage.execute("회복", &mut body, "", None, None, None);
    assert!(result.is_ok(), "회복 실행 실패: {:?}", result.err());
    assert_eq!(body.get_hp(), 321);
    assert_eq!(body.get_mp(), 123);
}
#[test]
fn stand_up_notifies_same_room_with_python_crlf_prompt_and_state() {
    use crate::script::party::set_precomputed_party_context;

    let self_name = "기상방알림본인";
    let other = "기상방알림상대";
    {
        let mut world = get_world_state().write().unwrap();
        for name in [self_name, other] {
            world.set_player_position(name, PlayerPosition::new("기상방알림존".into(), "1".into()));
        }
    }
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(other));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("rejects_other_combat_output".into(), Dynamic::from(false));
    person.insert("hp".into(), Dynamic::from(41_i64));
    person.insert("max_hp".into(), Dynamic::from(99_i64));
    person.insert("mp".into(), Dynamic::from(7_i64));
    person.insert("max_mp".into(), Dynamic::from(22_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);

    let mut body = Body::new();
    body.set("이름", self_name);
    body.act = crate::player::ActState::Rest;
    let result = ScriptStorage::default()
        .execute("일어나", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(body.act, crate::player::ActState::Stand);
    assert_eq!(result.0, vec!["당신이 운기조식을 마치며 벌떡 일어섭니다."]);
    assert!(matches!(
        result.1,
        Some(CommandResult::OutputAndSendToUsers(_, ref sends))
            if sends == &vec![(
                other.to_string(),
                format!(
                    "{}\r\n\x1b[1m{}\x1b[0;37m{} 운기조식을 마치며 벌떡 일어섭니다.\r\n\r\n\x1b[0;37;40m[ 41/99, 7/22 ] ",
                    RAW_USER_MESSAGE_PREFIX,
                    self_name,
                    han_iga(self_name)
                )
            )]
    ));

    let rejected = "기상출력거부상대";
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(other);
        world.set_player_position(
            rejected,
            PlayerPosition::new("기상방알림존".into(), "1".into()),
        );
    }
    let mut rejecting_person = rhai::Map::new();
    rejecting_person.insert("name".into(), Dynamic::from(rejected));
    rejecting_person.insert("show_prompt".into(), Dynamic::from(true));
    rejecting_person.insert("rejects_other_combat_output".into(), Dynamic::from(true));
    rejecting_person.insert("hp".into(), Dynamic::from(1_i64));
    rejecting_person.insert("max_hp".into(), Dynamic::from(1_i64));
    rejecting_person.insert("mp".into(), Dynamic::from(1_i64));
    rejecting_person.insert("max_mp".into(), Dynamic::from(1_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(rejecting_person)]),
    );
    set_precomputed_party_context(context);
    body.act = crate::player::ActState::Rest;
    let rejected_result = ScriptStorage::default()
        .execute("일어나", &mut body, "", None, None, None)
        .unwrap();
    assert!(
        matches!(
            rejected_result.1,
            Some(CommandResult::OutputAndSendToUsers(_, ref sends))
                if sends == &vec![(
                    rejected.to_string(),
                    format!(
                        "{}\r\n\x1b[1m{}\x1b[0;37m{} 운기조식을 마치며 벌떡 일어섭니다.\r\n\r\n\x1b[0;37;40m[ 1/1, 1/1 ] ",
                        RAW_USER_MESSAGE_PREFIX,
                        self_name,
                        han_iga(self_name)
                    )
                )]
        ),
        "Python sendRoom does not apply 타인전투출력거부"
    );

    for (act, expected) in [
        (
            crate::player::ActState::Stand,
            "☞ 이미 서있는 상태입니다. ^^",
        ),
        (
            crate::player::ActState::Fight,
            "☞ 운기조식중인 상태가 아닙니다.",
        ),
        (
            crate::player::ActState::Death,
            "☞ 운기조식중인 상태가 아닙니다.",
        ),
    ] {
        body.act = act;
        let state = ScriptStorage::default()
            .execute("일어나", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(state.0, vec![expected]);
        assert_eq!(body.act, act);
    }

    set_precomputed_party_context(rhai::Map::new());
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(self_name);
    world.remove_player_position(other);
    world.remove_player_position(rejected);
}
#[test]
fn rest_command_matches_python_forbidden_and_act_guard_order() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("휴식분기검사-{suffix}");
    let zone = format!("휴식분기존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for (room, properties) in [("1", vec!["쉼금지"]), ("2", Vec::<&str>::new())] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보": {
                "이름": format!("휴식 시험방 {room}"),
                "존이름": zone,
                "설명": [],
                "맵속성": properties,
                "출구": []
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
    body.act = crate::player::ActState::Fight;

    // Python checks the room prohibition before the current action state.
    let forbidden = storage
        .execute("쉬어", &mut body, "무시되는 인자", None, None, None)
        .unwrap();
    assert_eq!(
        forbidden.0,
        vec!["☞ 이곳에서 운기조식하기엔 적당하지 않군요."]
    );
    assert_eq!(body.act, crate::player::ActState::Fight);

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "2".to_string()));
    let fighting = storage
        .execute("쉬어", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(fighting.0, vec!["☞ 지금 쉬기에는 좋지가 않네요. ^^"]);
    assert_eq!(body.act, crate::player::ActState::Fight);

    body.act = crate::player::ActState::Stand;
    let started = storage
        .execute("쉬어", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        started.0,
        vec!["당신이 자세를 편안히 하며 운기조식에 들어갑니다."]
    );
    assert_eq!(body.act, crate::player::ActState::Rest);

    let already = storage
        .execute("쉬어", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(already.0, vec!["☞ 벌써 쉬고 있어요. ^^"]);
    assert_eq!(body.act, crate::player::ActState::Rest);

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn test_rest_notifies_only_players_in_the_same_room() {
    use crate::script::party::set_precomputed_party_context;

    let self_name = "휴식방알림본인";
    let same_room_name = "휴식방알림동일방";
    let other_room_name = "휴식방알림다른방";
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            self_name,
            PlayerPosition::new("휴식방알림존".to_string(), "1".to_string()),
        );
        world.set_player_position(
            same_room_name,
            PlayerPosition::new("휴식방알림존".to_string(), "1".to_string()),
        );
        world.set_player_position(
            other_room_name,
            PlayerPosition::new("휴식방알림존".to_string(), "2".to_string()),
        );
    }
    let mut person = rhai::Map::new();
    person.insert("name".into(), Dynamic::from(same_room_name));
    person.insert("show_prompt".into(), Dynamic::from(true));
    person.insert("hp".into(), Dynamic::from(33_i64));
    person.insert("max_hp".into(), Dynamic::from(44_i64));
    person.insert("mp".into(), Dynamic::from(5_i64));
    person.insert("max_mp".into(), Dynamic::from(6_i64));
    let mut context = rhai::Map::new();
    context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(person)]),
    );
    set_precomputed_party_context(context);

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", self_name);
    let (output, special) = storage
        .execute("쉬어", &mut body, "", None, None, None)
        .unwrap();

    assert_eq!(
        output,
        vec!["당신이 자세를 편안히 하며 운기조식에 들어갑니다."]
    );
    assert!(matches!(
        special,
        Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
            if own == "당신이 자세를 편안히 하며 운기조식에 들어갑니다."
                && sends == &vec![(
                    same_room_name.to_string(),
                    format!(
                        "{}\r\n\x1b[1m{}\x1b[0;37m{} 자세를 편안히 하며 운기조식에 들어갑니다.\r\n\r\n\x1b[0;37;40m[ 33/44, 5/6 ] ",
                        RAW_USER_MESSAGE_PREFIX,
                        self_name,
                        han_iga(self_name)
                    )
                )]
    ));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(self_name);
    world.remove_player_position(same_room_name);
    world.remove_player_position(other_room_name);
    set_precomputed_party_context(rhai::Map::new());
}
#[test]
fn test_rest_command_uses_python_act_rest_value() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("쉬어"));
    let mut body = Body::new();
    body.set("이름", "휴식검사");

    let result = storage.execute("쉬어", &mut body, "", None, None, None);
    assert!(result.is_ok(), "쉬어 실행 실패: {:?}", result.err());
    assert_eq!(body.act, crate::player::ActState::Rest);
    assert_eq!(body.act.to_i32(), 4);
}

#[test]
fn status_view_renders_socketless_summoned_player_like_python_channel_player() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin_name = format!("소환상태관리자-{suffix}");
    let target_name = format!("소환상태대상-{suffix}");
    let zone = format!("소환상태존-{suffix}");
    let mut target = Body::new();
    target.set("이름", target_name.as_str());
    target.set("레벨", 12_i64);
    target.set("나이", 34_i64);
    target.set("체력", 345_i64);
    target.set("최고체력", 456_i64);
    target.set("내공", 78_i64);
    target.set("최고내공", 90_i64);
    target.set("힘", 21_i64);
    target.set("민첩성", 22_i64);
    target.set("맷집", 23_i64);
    target.set("성격", "정파");
    target.set("성별", "남");
    target.set("소속", "시험방파");
    target.set("직위", "문도");
    target.set("배우자", "동반자");
    target.set("현재경험치", 123_i64);
    target.set("은전", 9876_i64);
    target.set("특성치", 3_i64);

    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 1000_i64);
    admin.set("체력", 1_i64);
    admin.set("최고체력", 100_i64);
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.add_summoned_user(target, PlayerPosition::new(zone.clone(), "1".into()));
    }

    let output = ScriptStorage::default()
        .execute("상태보기", &mut admin, &target_name, None, None, None)
        .unwrap()
        .0;
    assert_eq!(output.len(), 20);
    assert_eq!(output[0], "┏━━━━━━━━━━━━━━━━━━━━━━━━━┑");
    assert!(output[1].contains(&format!("{target_name}의 현재 상태")));
    assert!(output[3].contains("[   12]") && output[3].contains("  34"));
    assert!(output[4].contains("345/1146") && output[4].contains("정파"));
    assert!(output[9].contains("123") && output[9].contains("0/210"));
    assert!(output[14].contains("9876"));
    assert!(output[16].contains("저승사자가 손짓"));
    assert_eq!(
        output[17],
        format!("★ \x1b[1m{target_name}\x1b[0;37m의 표국보험은 효력이 없습니다.")
    );
    assert_eq!(
        output[19],
        format!(
            "★ \x1b[1m{target_name}\x1b[0;37m{} 3개의 여유 특성치를 보유하고 있습니다.",
            han_eun(&target_name)
        )
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_summoned_user(&target_name);
    world.remove_player_position(&admin_name);
}
