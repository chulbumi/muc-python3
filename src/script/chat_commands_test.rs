use super::*;
#[test]
fn adult_channel_disconnect_uses_leave_script_without_self_confirmation() {
    let storage = ScriptStorage::default();
    let self_id = "127.0.0.1:31911";
    let other_id = "127.0.0.1:31912";
    let mut actor = adult_channel_test_body("퇴장인", "", "");
    let other = adult_channel_test_body("남은인", "", "");
    let self_map = build_adult_channel_member_snapshot(self_id.to_string(), &actor, true, 1);
    let other_map = build_adult_channel_member_snapshot(other_id.to_string(), &other, true, 1);
    set_precomputed_adult_channel(vec![self_map, other_map], self_id.to_string(), true);
    actor
        .temp_mut()
        .insert(ADULT_CHANNEL_DISCONNECT_REQUEST.to_string(), Value::Int(1));

    storage
        .execute("채널퇴장", &mut actor, "", None, None, None)
        .unwrap();
    let (action, deliveries) = take_adult_channel_requests(&mut actor);
    assert_eq!(action.as_deref(), Some("leave"));
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].member_id, other_id);
    assert!(deliveries[0].raw_text.contains("퇴장하셨습니다."));
    clear_precomputed_all_online();
}
#[test]
fn adult_channel_scripts_use_ordered_membership_for_join_leave_chat_and_list() {
    let storage = ScriptStorage::default();
    let self_id = "127.0.0.1:31901";
    let other_id = "127.0.0.1:31902";
    let mut actor = adult_channel_test_body("입장인", "푸른별", "외침거부 0");
    let other = adult_channel_test_body("기존인", "", "외침거부 0");
    let other_map = build_adult_channel_member_snapshot(other_id.to_string(), &other, true, 1);

    set_precomputed_adult_channel(vec![other_map.clone()], self_id.to_string(), false);
    let (outputs, special) = storage
        .execute("채널입장", &mut actor, "", None, None, None)
        .unwrap();
    assert!(outputs.is_empty());
    assert!(special.is_none());
    let (action, deliveries) = take_adult_channel_requests(&mut actor);
    assert_eq!(action.as_deref(), Some("join"));
    assert_eq!(deliveries.len(), 2);
    assert_eq!(deliveries[0].member_id, self_id);
    assert_eq!(deliveries[0].raw_text, "☞ 채널에 입장합니다.\r\n\r\n");
    assert_eq!(deliveries[1].member_id, other_id);
    assert!(deliveries[1].raw_text.starts_with("\r\n\x1b[1;31m①⑨"));
    assert!(deliveries[1]
        .raw_text
        .ends_with("\r\n\x1b[0;37;40m[ 0/0, 0/0 ] "));
    clear_precomputed_all_online();

    actor
        .temp_mut()
        .insert(ADULT_CHANNEL_AUTO_JOIN_REQUEST.to_string(), Value::Int(1));
    set_precomputed_adult_channel(vec![other_map.clone()], self_id.to_string(), false);
    storage
        .execute("채널입장", &mut actor, "", None, None, None)
        .unwrap();
    let (action, deliveries) = take_adult_channel_requests(&mut actor);
    assert_eq!(action.as_deref(), Some("join"));
    assert_eq!(deliveries[0].member_id, other_id);
    assert_eq!(deliveries[1].member_id, self_id);
    clear_precomputed_all_online();

    set_precomputed_adult_channel(vec![], self_id.to_string(), true);
    let (outputs, _) = storage
        .execute("채널입장", &mut actor, "", None, None, None)
        .unwrap();
    assert_eq!(outputs, vec!["☞ 이미 입장하셨습니다."]);
    assert_eq!(take_adult_channel_requests(&mut actor), (None, vec![]));
    clear_precomputed_all_online();

    set_precomputed_adult_channel(vec![], self_id.to_string(), false);
    let (outputs, _) = storage
        .execute("채널잡담", &mut actor, "   ", None, None, None)
        .unwrap();
    assert_eq!(outputs, vec!["☞ 사용법: [내용] 채널잡담([)"]);
    let (outputs, _) = storage
        .execute("채널잡담", &mut actor, "안녕하세요", None, None, None)
        .unwrap();
    assert_eq!(outputs, vec!["☞ 먼저 채널에 입장하세요."]);

    let (outputs, _) = storage
        .execute("채널퇴장", &mut actor, "", None, None, None)
        .unwrap();
    assert_eq!(outputs, vec!["☞ 먼저 채널에 입장하세요."]);
    assert_eq!(take_adult_channel_requests(&mut actor), (None, vec![]));
    clear_precomputed_all_online();

    let self_map = build_adult_channel_member_snapshot(self_id.to_string(), &actor, true, 1);
    set_precomputed_adult_channel(
        vec![other_map.clone(), self_map.clone()],
        self_id.to_string(),
        true,
    );
    let (outputs, _) = storage
        .execute("채널퇴장", &mut actor, "", None, None, None)
        .unwrap();
    assert!(outputs.is_empty());
    let (action, deliveries) = take_adult_channel_requests(&mut actor);
    assert_eq!(action.as_deref(), Some("leave"));
    assert_eq!(deliveries.len(), 2);
    assert_eq!(deliveries[0].member_id, other_id);
    assert_eq!(deliveries[1].member_id, self_id);
    assert_eq!(deliveries[1].raw_text, "☞ 채널에서 퇴장합니다.\r\n\r\n");
    clear_precomputed_all_online();

    let inactive = adult_channel_test_body("잠든인", "잠든별", "외침거부 0");
    let refusing = adult_channel_test_body("거부인", "거부별", "외침거부 1");
    let inactive_map =
        build_adult_channel_member_snapshot("127.0.0.1:31903".into(), &inactive, false, 1);
    let refusing_map =
        build_adult_channel_member_snapshot("127.0.0.1:31904".into(), &refusing, true, 1);
    set_precomputed_adult_channel(
        vec![
            self_map.clone(),
            inactive_map,
            refusing_map,
            other_map.clone(),
        ],
        self_id.to_string(),
        true,
    );
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            "입장인",
            PlayerPosition::new("성인채널검증존".into(), "1".into()),
        );
    }
    storage
        .execute("채널잡담", &mut actor, "안녕하세요", None, None, None)
        .unwrap();
    let (action, deliveries) = take_adult_channel_requests(&mut actor);
    assert!(action.is_none());
    assert_eq!(deliveries.len(), 2);
    assert_eq!(deliveries[0].member_id, self_id);
    assert_eq!(deliveries[1].member_id, other_id);
    assert!(deliveries[0].raw_text.ends_with("안녕하세요\r\n\r\n"));
    assert_eq!(deliveries[1].raw_text.matches("안녕하세요").count(), 1);
    clear_precomputed_all_online();
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position("입장인");

    set_precomputed_adult_channel(vec![other_map, self_map], self_id.to_string(), true);
    let (outputs, _) = storage
        .execute("채널누구", &mut actor, "", None, None, None)
        .unwrap();
    assert_eq!(outputs.len(), 6);
    assert_eq!(outputs[0], "┌─────────────────────────────────────┐");
    assert!(outputs[3].contains("무명객"));
    assert!(outputs[3].contains("푸른별"));
    assert_eq!(outputs[5], " ★ 총 2명의 무림인이 활동하고 있습니다.");
    clear_precomputed_all_online();
}
fn adult_channel_test_body(name: &str, nickname: &str, config: &str) -> Body {
    let mut body = Body::new();
    body.set("이름", name);
    body.set("무림별호", nickname);
    body.set("성격", "정파");
    body.set("소속", "");
    body.set("투명상태", 0_i64);
    body.set("설정상태", config);
    body
}
#[test]
fn shout_variants_match_python_validation_order_and_no_room_behavior() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "외침검증자");

    for command in ["외쳐", "외쳐2"] {
        let usage = storage
            .execute(command, &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [내용] 외침(,)"]);
        let long = storage
            .execute(command, &mut body, &"가".repeat(161), None, None, None)
            .unwrap();
        assert_eq!(long.0, vec!["☞ 너무 길어요. ^^"]);
    }

    body.set("설정상태", "외침거부 1");
    for command in ["외쳐", "외쳐2"] {
        let refused = storage
            .execute(command, &mut body, "내용", None, None, None)
            .unwrap();
        assert_eq!(refused.0, vec!["☞ 외침거부중엔 외칠 수 없어요. ^^"]);
    }

    body.set("설정상태", "");
    body.act = crate::player::ActState::Rest;
    for command in ["외쳐", "외쳐2"] {
        let resting = storage
            .execute(command, &mut body, "내용", None, None, None)
            .unwrap();
        assert_eq!(
            resting.0,
            vec!["☞ 운기조식중에 외치게 되면 기가 흐트러집니다."]
        );
    }

    body.act = crate::player::ActState::Stand;
    set_precomputed_all_online(Vec::new());
    let no_room = storage
        .execute("외쳐", &mut body, "내용", None, None, None)
        .unwrap();
    clear_precomputed_all_online();
    assert!(no_room.0.is_empty());
    assert!(no_room.1.is_none(), "Python 외쳐 returns when env is None");
}
#[test]
fn tweet_uses_python_usage_and_recipient_time_ansi_preferences() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, PlayerPosition};

    let sender = "트윗발신자";
    let timed = "트윗시간수신자";
    let blocked = "트윗거부자";
    let waiting = "트윗입력대기자";
    let online = [
        (sender, "", 1_i64),
        (timed, "잡담시간보기 1\n사용자안시거부 1", 1_i64),
        (blocked, "외침거부 1", 1_i64),
        (waiting, "", 0_i64),
    ]
    .into_iter()
    .map(|(name, config, interactive)| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name));
        map.insert("설정상태".into(), Dynamic::from(config));
        map.insert("interactive".into(), Dynamic::from(interactive));
        map.insert("현재체력".into(), Dynamic::from(321_i64));
        map.insert("최고체력".into(), Dynamic::from(654_i64));
        map.insert("현재최고체력".into(), Dynamic::from(765_i64));
        map.insert("현재내공".into(), Dynamic::from(12_i64));
        map.insert("최고내공".into(), Dynamic::from(34_i64));
        map.insert("현재최고내공".into(), Dynamic::from(43_i64));
        Dynamic::from(map)
    })
    .collect();
    set_precomputed_all_online(online);
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(sender, PlayerPosition::new("트윗시험존".into(), "1".into()));
    let mut body = Body::new();
    body.set("이름", sender);
    body.set("act", 1_i64);
    let storage = ScriptStorage::default();

    let usage = storage
        .execute("트윗", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [내용] 외침(,)"]);

    let sent = storage
        .execute("트윗", &mut body, "{빨}안녕", None, None, None)
        .unwrap();
    assert!(sent.0.is_empty());
    let sends = match sent.1 {
        Some(CommandResult::SendToUsers(sends)) => sends,
        other => panic!("unexpected tweet result: {other:?}"),
    };
    assert_eq!(sends.len(), 3);
    let self_wire = sends
        .iter()
        .find(|(name, _)| name == sender)
        .unwrap()
        .1
        .clone();
    assert!(self_wire.starts_with(RAW_USER_MESSAGE_PREFIX));
    assert!(!self_wire[RAW_USER_MESSAGE_PREFIX.len()..].starts_with("\r\n"));
    assert!(self_wire.ends_with("\r\n"));
    assert!(self_wire.contains("\x1b[31m안녕"));
    let timed_wire = sends
        .iter()
        .find(|(name, _)| name == timed)
        .unwrap()
        .1
        .clone();
    assert!(timed_wire.starts_with(&format!("{}\r\n[", RAW_USER_MESSAGE_PREFIX)));
    assert!(!timed_wire.contains("\x1b[31m"));
    assert!(timed_wire.contains("안녕"));
    assert!(timed_wire.ends_with("\r\n\r\n\x1b[0;37;40m[ 321/765, 12/43 ] "));
    assert!(!sends.iter().any(|(name, _)| name == blocked));
    let waiting_wire = sends
        .iter()
        .find(|(name, _)| name == waiting)
        .unwrap()
        .1
        .clone();
    assert!(waiting_wire.ends_with("\x1b[0;37;40m\r\n"));
    assert!(!waiting_wire.contains("[ 321/765, 12/43 ]"));
    assert!(chat_history_snapshot()
        .last()
        .is_some_and(|line| line.contains("\x1b[31m안녕\x1b[0;37m")));

    body.set("성격", "선인");
    body.set("관리자등급", 2000_i64);
    let shouted = storage
        .execute("외쳐", &mut body, "{빨}호령", None, None, None)
        .unwrap();
    let shout_sends = match shouted.1 {
        Some(CommandResult::SendToUsers(sends)) => sends,
        other => panic!("unexpected shout result: {other:?}"),
    };
    assert!(shout_sends
        .iter()
        .find(|(name, _)| name == sender)
        .is_some_and(|(_, wire)| wire.contains("\x1b[0;35m사자후\x1b[0;37m")
            && wire.contains("\x1b[31m호령")));
    assert!(shout_sends
        .iter()
        .find(|(name, _)| name == timed)
        .is_some_and(|(_, wire)| wire.starts_with(&format!("{}\r\n[", RAW_USER_MESSAGE_PREFIX))
            && !wire.contains("\x1b[31m")
            && wire.ends_with("\r\n\r\n\x1b[0;37;40m[ 321/765, 12/43 ] ")));

    let shout2 = storage
        .execute("외쳐2", &mut body, "두번째", None, None, None)
        .unwrap();
    let shout2_sends = match shout2.1 {
        Some(CommandResult::SendToUsers(sends)) => sends,
        other => panic!("unexpected shout2 result: {other:?}"),
    };
    assert!(shout2_sends
        .iter()
        .all(|(_, wire)| wire.contains(" \x1b[1;32m밍밍이지렁~\x1b[0;37m")));
    assert!(shout2_sends
        .iter()
        .find(|(name, _)| name == timed)
        .is_some_and(|(_, wire)| wire.ends_with("\r\n\r\n\x1b[0;37;40m[ 321/765, 12/43 ] ")));

    clear_precomputed_all_online();
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(sender);
}
#[test]
fn expression_command_matches_python_usage_room_guard_and_delivery_text() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("표현회귀-{suffix}");
    let zone = format!("표현회귀존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    let room_path = room_dir.join("1.json");
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::write(
            &room_path,
            r#"{"맵정보":{"맵속성":["모든통신금지"],"이름":"표현시험방","존이름":"표현시험존","설명":[],"출구":[]}}"#,
        )
        .unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());

    let usage = storage
        .execute("표현", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [내용] 표현(')"]);
    let spaces = storage
        .execute("표현", &mut body, "  ", None, None, None)
        .unwrap();
    assert_eq!(spaces.0, vec!["☞ 사용법: [내용] 표현(')"]);
    let forbidden = storage
        .execute("표현", &mut body, "고개를 끄덕입니다.", None, None, None)
        .unwrap();
    assert_eq!(
        forbidden.0,
        vec!["☞ 이지역에서는 어떠한 통신도 불가능합니다."]
    );
    assert!(forbidden.1.is_none());

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
    let delivered = storage
        .execute("표현", &mut body, "고개를 끄덕입니다.", None, None, None)
        .unwrap();
    assert!(matches!(
        delivered.1,
        Some(CommandResult::EmotionToRoom(ref own, ref room, None))
            if own == "당신이 고개를 끄덕입니다."
                && room == &format!("{}{} 고개를 끄덕입니다.", player, han_iga(&player))
    ));
    let normalized = storage
        .execute("표현", &mut body, "  고개를 숙입니다.  ", None, None, None)
        .unwrap();
    assert!(matches!(
        normalized.1,
        Some(CommandResult::EmotionToRoom(ref own, ref room, None))
            if own == "당신이 고개를 숙입니다."
                && room == &format!("{}{} 고개를 숙입니다.", player, han_iga(&player))
    ));

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_dir_all(&room_dir);
}

#[test]
fn shout2_without_room_does_not_broadcast_like_python_failed_env_access() {
    let mut body = Body::new();
    body.set("이름", "방없는외침자");
    body.set("act", 1_i64);
    let mut recipient = rhai::Map::new();
    recipient.insert("이름".into(), Dynamic::from("외침수신자"));
    recipient.insert("설정상태".into(), Dynamic::from(""));
    recipient.insert("interactive".into(), Dynamic::from(1_i64));
    set_precomputed_all_online(vec![Dynamic::from(recipient)]);

    let output = ScriptStorage::default()
        .execute("외쳐2", &mut body, "전달되면 안 됨", None, None, None)
        .unwrap();

    clear_precomputed_all_online();
    assert!(output.0.is_empty());
    assert!(output.1.is_none());
}

#[test]
fn tweet_without_room_does_not_broadcast_like_python_failed_env_access() {
    let mut body = Body::new();
    body.set("이름", "방없는트윗자");
    body.set("act", 1_i64);
    let mut recipient = rhai::Map::new();
    recipient.insert("이름".into(), Dynamic::from("트윗수신자"));
    recipient.insert("설정상태".into(), Dynamic::from(""));
    recipient.insert("interactive".into(), Dynamic::from(1_i64));
    set_precomputed_all_online(vec![Dynamic::from(recipient)]);

    let output = ScriptStorage::default()
        .execute("트윗", &mut body, "전달되면 안 됨", None, None, None)
        .unwrap();

    clear_precomputed_all_online();
    assert!(output.0.is_empty());
    assert!(output.1.is_none());
}

#[test]
fn expression_without_room_does_not_emit_an_emotion() {
    let mut body = Body::new();
    body.set("이름", "방없는표현자");
    let output = ScriptStorage::default()
        .execute("표현", &mut body, "손을 흔듭니다.", None, None, None)
        .unwrap();
    assert!(output.0.is_empty());
    assert!(output.1.is_none());
}
use crate::command::handler::CommandResult;

#[test]
fn say_matches_python_send_room_ansi_and_each_players_prompt() {
    let storage = ScriptStorage::default();
    let zone = format!("말프롬프트존-{}", std::process::id());
    {
        let mut world = crate::world::get_world_state().write().unwrap();
        for name in ["철수", "영희", "길동", "잠든이"] {
            world.set_player_position(
                name,
                crate::world::PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
    }
    let mut speaker = Body::new();
    speaker.set("이름", "철수");
    speaker.set("체력", 900_i64);
    speaker.set("최고체력", 1000_i64);
    speaker.set("내공", 18_i64);
    speaker.set("최고내공", 20_i64);

    let empty = storage
        .execute("말", &mut speaker, "", None, None, None)
        .unwrap();
    assert_eq!(empty.0, vec!["\r\nSay What???"]);

    let mut recipient = Body::new();
    recipient.set("이름", "영희");
    recipient.set("체력", 700_i64);
    recipient.set("최고체력", 800_i64);
    recipient.set("내공", 12_i64);
    recipient.set("최고내공", 14_i64);
    let mut no_lp = Body::new();
    no_lp.set("이름", "길동");
    no_lp.set("설정상태", "엘피출력 1");
    no_lp.set("체력", 500_i64);
    no_lp.set("최고체력", 600_i64);
    no_lp.set("내공", 9_i64);
    no_lp.set("최고내공", 10_i64);
    let mut noninteractive = Body::new();
    noninteractive.set("이름", "잠든이");

    set_cast_room_players(vec![
        CastRoomPlayerRef::new(&mut recipient),
        CastRoomPlayerRef::new(&mut no_lp),
        CastRoomPlayerRef::new_with_interactive(&mut noninteractive, 0),
    ]);
    let spoken = storage
        .execute("말", &mut speaker, "{빨}안녕{어}!", None, None, None)
        .unwrap();
    clear_cast_room_players();

    let own = "당신이 말합니다 : '\x1b[31m안녕\x1b[0m!\x1b[0;40;37m'";
    let room = "\x1b[33m철수\x1b[37m가 말합니다 : '\x1b[31m안녕\x1b[0m!\x1b[0;40;37m'";
    assert_eq!(
        spoken.0,
        vec![own, room, "\r\n\x1b[0;37;40m[ 900/1000, 18/20 ] "]
    );
    let sends = match spoken.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected say result: {other:?}"),
    };
    assert_eq!(
        sends,
        vec![
            (
                "영희".to_string(),
                format!("{}{room}\r\n\r\n\x1b[0;37;40m[ 700/800, 12/14 ] ", RAW_USER_MESSAGE_PREFIX)
            ),
            ("길동".to_string(), format!("{}{room}\r\n", RAW_USER_MESSAGE_PREFIX)),
            ("잠든이".to_string(), format!("{}{room}\r\n", RAW_USER_MESSAGE_PREFIX)),
        ]
    );

    speaker.set("설정상태", "엘피출력 1");
    set_cast_room_players(Vec::new());
    let hidden_own_prompt = storage
        .execute("말", &mut speaker, "조용히", None, None, None)
        .unwrap();
    clear_cast_room_players();
    assert_eq!(hidden_own_prompt.0.len(), 2);

    let mut world = crate::world::get_world_state().write().unwrap();
    for name in ["철수", "영희", "길동", "잠든이"] {
        world.remove_player_position(name);
    }
}
