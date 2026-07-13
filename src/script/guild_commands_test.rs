use super::*;
#[test]
fn guild_applicants_round_trip_as_python_array_and_loaded_pipe_tokens() {
    let path = std::env::temp_dir().join(format!(
        "muc_guild_applicants_{}_{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut body = Body::new();
    body.set("이름", "입문신청저장검사");
    body.set("입문신청자", "첫신청자\r\n둘째신청자");
    assert!(save_body_to_json(&mut body, path.to_str().unwrap()));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        json["사용자오브젝트"]["입문신청자"],
        serde_json::json!(["첫신청자", "둘째신청자"])
    );

    let mut loaded = Body::new();
    assert!(load_body_from_json(&mut loaded, path.to_str().unwrap()));
    assert_eq!(loaded.get_string("입문신청자"), "첫신청자|둘째신청자");
    assert!(save_body_to_json(&mut loaded, path.to_str().unwrap()));
    let saved_again: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        saved_again["사용자오브젝트"]["입문신청자"],
        serde_json::json!(["첫신청자", "둘째신청자"])
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn guild_position_command_moves_python_role_lists_and_emits_group_layout() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let actor = format!("직위방주-{suffix}");
    let target = format!("직위대상-{suffix}");
    let guild = format!("직위시험방파-{suffix}");
    let zone = format!("직위시험존-{suffix}");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    crate::world::guild::guild_set(&guild, "이름", &guild);
    crate::world::guild::guild_set(&guild, "방주명칭", "시험방주");
    crate::world::guild::guild_set(&guild, "방주리스트", &actor);
    crate::world::guild::guild_set(&guild, "방파인리스트", &target);
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&actor, &target] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
        }
    }
    let online = [
        (&actor, "방주", "", 900_i64, 18_i64),
        (&target, "방파인", "", 700_i64, 12_i64),
    ]
    .into_iter()
    .map(|(name, position, config, hp, mp)| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name.clone()));
        map.insert("소속".into(), Dynamic::from(guild.clone()));
        map.insert("직위".into(), Dynamic::from(position));
        map.insert("설정상태".into(), Dynamic::from(config));
        map.insert("zone".into(), Dynamic::from(zone.clone()));
        map.insert("room".into(), Dynamic::from("1"));
        map.insert("현재체력".into(), Dynamic::from(hp));
        map.insert("최고체력".into(), Dynamic::from(hp));
        map.insert("현재내공".into(), Dynamic::from(mp));
        map.insert("최고내공".into(), Dynamic::from(mp));
        Dynamic::from(map)
    })
    .collect();
    set_precomputed_all_online(online);
    let room_player = |name: &str, position: &str, reaction: &str| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name.to_string()));
        map.insert("반응이름".into(), Dynamic::from(reaction.to_string()));
        map.insert("소속".into(), Dynamic::from(guild.clone()));
        map.insert("직위".into(), Dynamic::from(position.to_string()));
        Dynamic::from(map)
    };
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![
            room_player(&actor, "방주", "방주별칭"),
            room_player(&target, "방파인", "대상별칭"),
        ],
    )]));
    let mut body = Body::new();
    body.set("이름", actor.as_str());
    body.set("소속", guild.as_str());
    body.set("직위", "방주");
    let storage = ScriptStorage::default();

    let usage = storage
        .execute("직위임명", &mut body, "대상 제자", None, None, None)
        .unwrap();
    assert_eq!(
        usage.0,
        vec!["☞ 사용법 : [대상] [방주|부방주|장로|방파인] 직위임명"]
    );
    let mut collision = Object::new();
    collision.set("이름", "직위충돌물건");
    collision.set("반응이름", "대상별칭");
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let item_first = storage
        .execute("직위임명", &mut body, "대상별칭 방파인", None, None, None)
        .unwrap();
    assert_eq!(item_first.0, vec!["☞ 당신의 소속이 아닙니다."]);
    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        "1",
        crate::world::RoomObjectRef::Player(target.clone()),
    );
    let second_item = storage
        .execute("직위임명", &mut body, "2대상별칭 방파인", None, None, None)
        .unwrap();
    assert_eq!(second_item.0, vec!["☞ 당신의 소속이 아닙니다."]);
    let same = storage
        .execute("직위임명", &mut body, "대상별칭 방파인", None, None, None)
        .unwrap();
    assert_eq!(same.0, vec!["☞ 같은 직위입니다."]);
    let mob_key = format!("{zone}:직위충돌몹");
    let mut mob_data = RawMobData::new();
    mob_data.name = "직위충돌몹".into();
    mob_data.reaction_names = vec!["몹직위별칭".into()];
    mob_data.zone = zone.clone();
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), mob_data.clone());
        let mob = MobInstance::new(mob_key.clone(), zone.clone(), "1", &mob_data);
        let mob_id = mob.instance_id;
        world.mob_cache.add_mob_instance(mob);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(mob_id));
    }
    let mob_target = storage
        .execute("직위임명", &mut body, "몹직위별칭 장로", None, None, None)
        .unwrap();
    assert_eq!(mob_target.0, vec!["☞ 당신의 소속이 아닙니다."]);
    crate::world::guild::guild_set(&guild, "장로리스트", "장로1\r\n장로2\r\n장로3\r\n장로4");
    let full = storage
        .execute("직위임명", &mut body, "대상별칭 장로", None, None, None)
        .unwrap();
    assert_eq!(full.0, vec!["☞ 같은 직위의 인원이 너무 많습니다."]);
    assert_eq!(take_guild_position_request(&mut body), None);
    crate::world::guild::guild_set(&guild, "장로리스트", "");
    let changed = storage
        .execute("직위임명", &mut body, "대상별칭 장로", None, None, None)
        .unwrap();
    let actor_text = format!("\x1b[1m{actor}\x1b[0;37m{}", han_iga(&actor));
    let target_text = format!("\x1b[1m{target}\x1b[0;37m{}", han_eul(&target));
    let expected = format!(
            "\x1b[1m《\x1b[36m시험방주\x1b[37mː\x1b[36m{actor}\x1b[37m》\x1b[0;37m {actor_text} {target_text} \x1b[1m장로\x1b[0m로 직위를 임명합니다."
        );
    assert_eq!(changed.0, vec![expected.clone()]);
    let sends = match changed.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected result: {other:?}"),
    };
    assert_eq!(
        sends,
        vec![(
            target.clone(),
            format!("{expected}\r\n\x1b[0;37;40m[ 700/700, 12/12 ] ")
        )]
    );
    assert_eq!(
        take_guild_position_request(&mut body),
        Some((target.clone(), "장로".to_string()))
    );
    let guild_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("data/config/guild.json").unwrap()).unwrap();
    assert_eq!(guild_json[&guild]["방파인리스트"], serde_json::json!([]));
    assert_eq!(
        guild_json[&guild]["장로리스트"],
        serde_json::json!([target.clone()])
    );
    assert!(crate::world::guild::guild_kick_member(
        &guild, "장로", &target
    ));

    clear_precomputed_all_online();
    clear_precomputed_room_view_players();
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").clear();
        world.mob_cache.remove_mob(&mob_key);
        world.remove_player_position(&actor);
        world.remove_player_position(&target);
    }
    crate::world::guild::guild_remove(&guild);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
}

#[test]
fn guild_chat_aliases_match_python_guard_order_layout_and_prompt() {
    use crate::command::handler::CommandResult;

    let suffix = std::process::id();
    let sender = "길동".to_string();
    let recipient = format!("방파말수신-{suffix}");
    let rejecting = format!("방파말거부-{suffix}");
    let transparent = format!("방파말투명-{suffix}");
    let no_prompt = format!("방파말엘피-{suffix}");
    let other_guild = format!("다른방파원-{suffix}");
    let guild = format!("방파말시험-{suffix}");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    let _ = std::fs::remove_file(format!("data/log/group/{guild}"));
    let mut body = Body::new();
    body.set("이름", sender.as_str());
    let storage = ScriptStorage::default();

    for command in ["방파말", "똥파말"] {
        let no_guild = storage
            .execute(command, &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(no_guild.0, vec!["☞ 당신은 소속이 없습니다."]);
    }
    body.set("소속", guild.as_str());
    body.set("직위", "장로");
    let usage = storage
        .execute("똥파말", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법 : [내용] 방파말(])"]);

    body.set("설정상태", "방파말거부 1");
    let refused = storage
        .execute("방파말", &mut body, "기록되면 안됨", None, None, None)
        .unwrap();
    assert_eq!(refused.0, vec!["☞ 방파말 거부중 이에요. *^^*"]);
    assert!(!std::path::Path::new(&format!("data/log/group/{guild}")).exists());
    body.set("설정상태", "");

    let online = [
        (&sender, "", 0_i64, 900_i64, 18_i64),
        (&recipient, "", 0_i64, 700_i64, 12_i64),
        (&rejecting, "방파말거부 1", 0_i64, 600_i64, 11_i64),
        (&transparent, "", 1_i64, 500_i64, 10_i64),
        (&no_prompt, "엘피출력 1", 0_i64, 400_i64, 9_i64),
        (&other_guild, "", 0_i64, 300_i64, 8_i64),
    ]
    .into_iter()
    .map(|(name, config, hidden, hp, mp)| {
        let mut player = rhai::Map::new();
        player.insert("이름".into(), Dynamic::from(name.clone()));
        player.insert(
            "소속".into(),
            Dynamic::from(if name == &other_guild {
                "다른방파".to_string()
            } else {
                guild.clone()
            }),
        );
        player.insert("설정상태".into(), Dynamic::from(config));
        player.insert("투명상태".into(), Dynamic::from(hidden));
        player.insert("현재체력".into(), Dynamic::from(hp));
        player.insert("최고체력".into(), Dynamic::from(hp));
        player.insert("현재내공".into(), Dynamic::from(mp));
        player.insert("최고내공".into(), Dynamic::from(mp));
        Dynamic::from(player)
    })
    .collect();
    set_precomputed_all_online(online);
    let sent = storage
        .execute("방파말", &mut body, "모두 안녕", None, None, None)
        .unwrap();
    let line =
        format!("\x1b[1m《\x1b[36m장로\x1b[37mː\x1b[36m{sender}\x1b[37m》\x1b[0;37m 모두 안녕");
    assert_eq!(sent.0, vec![line.clone()]);
    let sends = match sent.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected guild chat result: {other:?}"),
    };
    assert_eq!(
        sends,
        vec![
            (
                recipient.clone(),
                format!("{line}\r\n\x1b[0;37;40m[ 700/700, 12/12 ] ")
            ),
            (
                transparent.clone(),
                format!("{line}\r\n\x1b[0;37;40m[ 500/500, 10/10 ] ")
            ),
            (no_prompt.clone(), line.clone())
        ]
    );
    let alias = storage
        .execute("똥파말", &mut body, "별칭 안녕", None, None, None)
        .unwrap();
    let alias_line =
        format!("\x1b[1m《\x1b[36m장로\x1b[37mː\x1b[36m{sender}\x1b[37m》\x1b[0;37m 별칭 안녕");
    assert_eq!(alias.0, vec![alias_line.clone()]);
    let alias_sends = match alias.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected alias guild chat result: {other:?}"),
    };
    assert_eq!(
        alias_sends,
        vec![
            (
                recipient.clone(),
                format!("{alias_line}\r\n\x1b[0;37;40m[ 700/700, 12/12 ] ")
            ),
            (
                transparent.clone(),
                format!("{alias_line}\r\n\x1b[0;37;40m[ 500/500, 10/10 ] ")
            ),
            (no_prompt.clone(), alias_line.clone())
        ]
    );
    let log = std::fs::read_to_string(format!("data/log/group/{guild}")).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(
        regex::Regex::new(r"^\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\] 길동        : 모두 안녕$")
            .unwrap()
            .is_match(lines[0]),
        "{}",
        lines[0]
    );
    assert!(lines[1].ends_with("길동        : 별칭 안녕"));

    // Player.sendGroup tests key membership, not truthiness.  A present
    // but empty custom title therefore renders an empty title.
    crate::world::guild::guild_set(&guild, "장로명칭", "");
    let empty_title = storage
        .execute("방파말", &mut body, "빈 명칭", None, None, None)
        .unwrap();
    assert_eq!(
        empty_title.0,
        vec![format!(
            "\x1b[1m《\x1b[36m\x1b[37mː\x1b[36m{sender}\x1b[37m》\x1b[0;37m 빈 명칭"
        )]
    );

    clear_precomputed_all_online();
    let _ = std::fs::remove_file(format!("data/log/group/{guild}"));
    crate::world::guild::guild_remove(&guild);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
}

#[test]
fn guild_list_matches_python_columns_count_leader_map_and_single_sendline() {
    let snapshot = std::fs::read("data/config/guild.json").unwrap_or_default();
    let suffix = std::process::id();
    let id = format!("리스트방파-{suffix}");
    guild_set(&id, "이름", &id);
    guild_set(&id, "방주이름", "홍길동");
    guild_set(&id, "방파원수", "7");
    guild_set(&id, "방파맵", "방파맵:청풍회");
    let storage = ScriptStorage::default();
    let mut normal = Body::new();
    let shown = storage
        .execute("방파리스트", &mut normal, "", None, None, None)
        .unwrap();
    assert_eq!(
        shown.0.len(),
        1,
        "Python builds one buffer and calls sendLine once"
    );
    let display = format!("[{id}]");
    let expected_normal = format!("{:<12} : {:<30}   {:>3} 명", display, "홍길동", 7);
    assert!(shown.0[0].contains(&format!("{expected_normal}\r\n")));
    assert!(!shown.0[0].contains("방파맵:청풍회\r\n"));

    let mut admin = Body::new();
    admin.set("관리자등급", 1000_i64);
    let admin_shown = storage
        .execute("방파리스트", &mut admin, "", None, None, None)
        .unwrap();
    let expected_admin = format!("{expected_normal} 방파맵:청풍회\r\n");
    assert!(admin_shown.0[0].contains(&expected_admin));
    assert!(admin_shown.0[0].starts_with("━━━━━"));
    assert!(admin_shown.0[0].ends_with("━━━━━"));

    // Python indexes 방주이름 directly.  An empty value stays empty; it
    // never falls back to the legacy 방주리스트 field.
    guild_set(&id, "방주이름", "");
    guild_set(&id, "방주리스트", "대체하면안되는방주");
    let empty_leader = storage
        .execute("방파리스트", &mut normal, "", None, None, None)
        .unwrap();
    let row = empty_leader.0[0]
        .split("\r\n")
        .find(|line| line.starts_with(&display))
        .unwrap();
    assert_eq!(row, format!("{:<12} : {:<30}   {:>3} 명", display, "", 7));
    assert!(!row.contains("대체하면안되는방주"));

    guild_remove(&id);
    let _ = std::fs::write("data/config/guild.json", snapshot);
}

#[test]
fn guild_status_uses_legacy_roles_stored_count_three_columns_and_active_filter() {
    let snapshot = std::fs::read("data/config/guild.json").unwrap_or_default();
    let suffix = std::process::id();
    let guild = format!("상태방파-{suffix}");
    guild_set(&guild, "방주이름", "방주홍");
    guild_set(&guild, "부방주리스트", "부일\r\n부이");
    guild_set(&guild, "장로리스트", "장로하나");
    guild_set(&guild, "방파인리스트", "방파원갑\r\n방파원을");
    guild_set(&guild, "방파원수", "6");
    let online = [
        ("방주홍", guild.as_str(), 0_i64),
        ("부일", guild.as_str(), 0_i64),
        ("숨은방파원", guild.as_str(), 1_i64),
        ("타방파원", "다른방파", 0_i64),
    ]
    .into_iter()
    .map(|(name, affiliation, transparent)| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name));
        map.insert("소속".into(), Dynamic::from(affiliation.to_string()));
        map.insert("투명상태".into(), Dynamic::from(transparent));
        Dynamic::from(map)
    })
    .collect();
    set_precomputed_all_online(online);
    let mut body = Body::new();
    body.set("소속", guild.as_str());
    let shown = ScriptStorage::default()
        .execute("방파상태", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(shown.0.len(), 1);
    let text = &shown.0[0];
    assert!(text.contains("방주홍        "));
    assert!(text.contains("부일         "));
    assert!(
        text.contains("부이         \r\n"),
        "leader + two deputies complete Python's first row"
    );
    assert!(text.contains("장로하나       "));
    assert!(text.contains("방파원갑       "));
    assert!(text.contains("방파원을       \r\n"));
    assert!(text.contains("방파총인원 : 6       ☞ 현재 2명이 활동중 입니다."));

    guild_set(&guild, "방주이름", "");
    guild_set(&guild, "방주리스트", "대체하면안되는방주");
    let empty_leader = ScriptStorage::default()
        .execute("방파상태", &mut body, "", None, None, None)
        .unwrap();
    assert!(!empty_leader.0[0].contains("대체하면안되는방주"));
    assert!(empty_leader.0[0]
        .contains("[\x1b[1m\x1b[31m방  주\x1b[0m\x1b[40m\x1b[37m]                "));

    clear_precomputed_all_online();
    guild_remove(&guild);
    let _ = std::fs::write("data/config/guild.json", snapshot);
}

#[test]
fn guild_status_handles_empty_and_stale_affiliation_without_python_key_error() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();

    let empty = storage
        .execute("방파상태", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(empty.0, vec!["☞ 당신은 소속이 없습니다."]);

    let stale = format!("삭제된방파-{}", std::process::id());
    guild_remove(&stale);
    body.set("소속", stale);
    let missing = storage
        .execute("방파상태", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 당신은 소속이 없습니다."]);
}

#[test]
fn guild_acceptance_consumes_leader_applicant_and_adds_python_member_role() {
    use crate::world::{get_world_state, PlayerPosition};
    let snapshot = std::fs::read("data/config/guild.json").unwrap_or_default();
    let suffix = std::process::id();
    let leader = format!("입문방주-{suffix}");
    let target = format!("입문대상-{suffix}");
    let guild = format!("입문방파-{suffix}");
    let zone = format!("입문존-{suffix}");
    guild_set(&guild, "이름", &guild);
    guild_set(&guild, "방주이름", &leader);
    guild_set(&guild, "방파원수", "1");
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&leader, &target] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
        }
    }
    let mut target_view = rhai::Map::new();
    target_view.insert("이름".into(), Dynamic::from(target.clone()));
    target_view.insert("반응이름".into(), Dynamic::from("입문별칭 긴별칭"));
    let mut room_views = HashMap::new();
    room_views.insert(format!("{zone}:1"), vec![Dynamic::from(target_view)]);
    set_precomputed_room_view_players(room_views);
    let mut body = Body::new();
    body.set("이름", leader.as_str());
    body.set("직위", "방주");
    body.set("소속", guild.as_str());
    body.set("반응이름", "입문방주별칭");
    let storage = ScriptStorage::default();

    body.set("직위", "방파인");
    let unauthorized = storage
        .execute("방파입문", &mut body, &target, None, None, None)
        .unwrap();
    assert_eq!(unauthorized.0, vec!["☞ 방파의 방주만이 할 수 있습니다."]);
    body.set("직위", "방주");
    let usage = storage
        .execute("방파입문", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법 : [대상] 방파입문"]);
    let self_target = storage
        .execute("방파입문", &mut body, "입문방주별칭", None, None, None)
        .unwrap();
    assert_eq!(self_target.0, vec!["☞ 자기 자신입니다."]);
    let not_requested = storage
        .execute("방파입문", &mut body, &target, None, None, None)
        .unwrap();
    assert_eq!(
        not_requested.0,
        vec!["☞ 방파를 신청한 그런 무림인이 없습니다."]
    );

    // Python JSON arrays are represented internally with `|` after load.
    body.set("입문신청자", format!("다른신청자|{target}"));
    let mut recipient = rhai::Map::new();
    recipient.insert("name".into(), Dynamic::from(target.clone()));
    recipient.insert("show_prompt".into(), Dynamic::from(true));
    recipient.insert("hp".into(), Dynamic::from(700_i64));
    recipient.insert("max_hp".into(), Dynamic::from(900_i64));
    recipient.insert("mp".into(), Dynamic::from(12_i64));
    recipient.insert("max_mp".into(), Dynamic::from(18_i64));
    let mut party_context = rhai::Map::new();
    party_context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(recipient)]),
    );
    crate::script::party::set_precomputed_party_context(party_context);
    let result = storage
        .execute("방파입문", &mut body, "입문별", None, None, None)
        .unwrap();
    assert_eq!(
        result.0,
        vec![format!(
            "당신이 \x1b[1m{target}\x1b[0;37m{} 방파에 입문시켰음을 선포합니다.",
            han_eul(&target)
        )]
    );
    assert!(matches!(
        result.1,
        Some(CommandResult::OutputAndSendToUsers(_, ref sends))
            if sends == &vec![(
                target.clone(),
                format!("{}\r\n\x1b[1m{leader}\x1b[0;37m{} 당신을 방파에 입문시켰음을 선포합니다.\r\n\r\n\x1b[0;37;40m[ 700/900, 12/18 ] ", RAW_USER_MESSAGE_PREFIX, han_iga(&leader))
            )]
    ));
    assert_eq!(body.get_string("입문신청자"), "다른신청자");
    assert_eq!(
        crate::world::guild::guild_role_members(&guild, "방파인"),
        vec![target.clone()]
    );
    assert_eq!(guild_get(&guild, "방파원수"), "2");
    let guild_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("data/config/guild.json").unwrap()).unwrap();
    assert_eq!(
        guild_json[&guild]["방파인리스트"],
        serde_json::json!([target.clone()])
    );
    assert_eq!(
        take_guild_accept_request(&mut body),
        Some((target.clone(), guild.clone()))
    );

    // Python appends a duplicate and increments 방파원수 here. Rust repairs
    // that data bug, but the repeated accepted application remains a
    // successful command and is consumed atomically.
    body.set("입문신청자", target.as_str());
    let duplicate = ScriptStorage::default()
        .execute("방파입문", &mut body, "입문별", None, None, None)
        .unwrap();
    assert_eq!(duplicate.0.len(), 1);
    assert!(duplicate.0[0].contains("방파에 입문시켰음을 선포합니다."));
    assert_eq!(body.get_string("입문신청자"), "");
    assert_eq!(
        crate::world::guild::guild_role_members(&guild, "방파인"),
        vec![target.clone()]
    );
    assert_eq!(guild_get(&guild, "방파원수"), "2");
    assert_eq!(
        take_guild_accept_request(&mut body),
        Some((target.clone(), guild.clone()))
    );

    clear_precomputed_room_view_players();
    crate::script::party::clear_precomputed_party_context();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&leader);
    world.remove_player_position(&target);
    drop(world);
    guild_remove(&guild);
    let _ = std::fs::write("data/config/guild.json", snapshot);
}

#[test]
fn guild_application_matches_python_personality_duplicate_and_output_rules() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let applicant = format!("신청자-{suffix}");
    let leader = format!("신청방주-{suffix}");
    let zone = format!("신청존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&applicant, &leader] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
        }
    }

    let snapshot = |applicants: &str| {
        let mut target = rhai::Map::new();
        target.insert("이름".into(), Dynamic::from(leader.clone()));
        target.insert("반응이름".into(), Dynamic::from("문주별칭"));
        target.insert("직위".into(), Dynamic::from("방주"));
        target.insert("성격".into(), Dynamic::from("기인"));
        target.insert("기존성격".into(), Dynamic::from("정파"));
        target.insert("입문신청자".into(), Dynamic::from(applicants.to_string()));
        HashMap::from([(format!("{zone}:1"), vec![Dynamic::from(target)])])
    };

    let mut body = Body::new();
    body.set("이름", applicant.as_str());
    body.set("성격", "정파");
    set_precomputed_room_view_players(snapshot("다른신청자"));
    let mut leader_person = rhai::Map::new();
    leader_person.insert("name".into(), Dynamic::from(leader.clone()));
    leader_person.insert("show_prompt".into(), Dynamic::from(true));
    leader_person.insert("hp".into(), Dynamic::from(500_i64));
    leader_person.insert("max_hp".into(), Dynamic::from(700_i64));
    leader_person.insert("mp".into(), Dynamic::from(30_i64));
    leader_person.insert("max_mp".into(), Dynamic::from(40_i64));
    let mut party_context = rhai::Map::new();
    party_context.insert(
        "room_players".into(),
        Dynamic::from(vec![Dynamic::from(leader_person)]),
    );
    crate::script::party::set_precomputed_party_context(party_context);
    let result = ScriptStorage::default()
        .execute("입문신청", &mut body, "문주별칭", None, None, None)
        .unwrap();
    assert_eq!(
        result.0,
        vec![format!(
            "당신이 \x1b[1m{leader}\x1b[0;37m의 방파에 입문을 신청합니다."
        )]
    );
    assert!(matches!(
        result.1,
        Some(CommandResult::OutputAndSendToUsers(_, ref sends))
            if sends == &vec![(
                leader.clone(),
                format!("{}\r\n\x1b[1m{applicant}\x1b[0;37m{} 당신의 방파에 입문을 신청합니다.\r\n\r\n\x1b[0;37;40m[ 500/700, 30/40 ] ", RAW_USER_MESSAGE_PREFIX, han_iga(&applicant))
            )]
    ));
    assert_eq!(
        take_guild_apply_request(&mut body),
        Some((leader.clone(), applicant.clone()))
    );

    set_precomputed_room_view_players(snapshot(&format!("다른신청자|{applicant}")));
    let duplicate = ScriptStorage::default()
        .execute("입문신청", &mut body, &leader, None, None, None)
        .unwrap();
    assert_eq!(duplicate.0, vec!["☞ 이미 입문 신청을 하였습니다."]);
    assert_eq!(take_guild_apply_request(&mut body), None);

    body.set("성격", "사파");
    set_precomputed_room_view_players(snapshot(""));
    let rejected = ScriptStorage::default()
        .execute("입문신청", &mut body, &leader, None, None, None)
        .unwrap();
    assert_eq!(rejected.0, vec!["☞ 방파에 입문 신청을 할 수 없습니다."]);
    assert_eq!(take_guild_apply_request(&mut body), None);

    clear_precomputed_room_view_players();
    crate::script::party::set_precomputed_party_context(rhai::Map::new());
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&applicant);
    world.remove_player_position(&leader);
}

#[test]
fn guild_application_keeps_python_early_validation_order() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let applicant = format!("입문검증자-{suffix}");
    let leader = format!("입문검증방주-{suffix}");
    let zone = format!("입문검증존-{suffix}");
    let mut body = Body::new();
    body.set("이름", applicant.as_str());
    let storage = ScriptStorage::default();

    body.set("소속", "이미가입한방파");
    let membership_first = storage
        .execute("입문신청", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        membership_first.0,
        vec!["☞ 방파에 입문 신청을 할 수 없습니다."]
    );
    body.set("소속", "");
    let usage = storage
        .execute("입문신청", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법 : [방주이름] 입문신청"]);
    let missing = storage
        .execute("입문신청", &mut body, "없는사람", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 이곳에 그런 무림인이 없습니다."]);

    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&applicant, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&leader, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let snapshot = |name: &str, position: &str| {
        let mut player = rhai::Map::new();
        player.insert("이름".into(), Dynamic::from(name.to_string()));
        player.insert("직위".into(), Dynamic::from(position.to_string()));
        player.insert("성격".into(), Dynamic::from("정파"));
        player.insert("기존성격".into(), Dynamic::from(""));
        player.insert("입문신청자".into(), Dynamic::from(""));
        Dynamic::from(player)
    };
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![snapshot(&applicant, ""), snapshot(&leader, "문도")],
    )]));
    let own = storage
        .execute("입문신청", &mut body, &applicant, None, None, None)
        .unwrap();
    assert_eq!(own.0, vec!["☞ 자기 자신입니다."]);
    let not_leader = storage
        .execute("입문신청", &mut body, &leader, None, None, None)
        .unwrap();
    assert_eq!(not_leader.0, vec!["☞ 방파의 방주만이 할 수 있습니다."]);

    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&applicant);
    world.remove_player_position(&leader);
}

#[test]
fn admin_room_edit_and_object_attributes_match_python_state_and_persistence() {
    use crate::command::{CommandResult, PendingInput};
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player = format!("방편집관리자-{suffix}");
    let zone = format!("방편집존-{suffix}");
    let directory = format!("data/map/{zone}");
    let path = format!("{directory}/1.json");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        &path,
        r#"{"맵정보":{"이름":"옛 방","설명":"옛 설명","출구":[]}}"#,
    )
    .unwrap();

    let mut floor = Object::new();
    floor.set("이름", "시험석");
    floor.set("반응이름", "돌 시험석");
    let floor = Arc::new(Mutex::new(floor));
    let mob_key = format!("{zone}:속성몹");
    let mut mob_data = RawMobData::new();
    mob_data.name = "속성시험몹".into();
    mob_data.reaction_names = vec!["돌".into()];
    mob_data.zone = zone.clone();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
        world.get_room_objs_mut(&zone, "1").push(floor.clone());
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), mob_data.clone());
        let mob = MobInstance::new(mob_key, zone.clone(), "1", &mob_data);
        let mob_id = mob.instance_id;
        world.mob_cache.add_mob_instance(mob);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(mob_id));
        // Python Room.insert places the latest object first.
        world.record_floor_item(&zone, "1", &floor);
    }

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 999_i64);
    let denied = storage
        .execute("방이름", &mut body, "새 방", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 2000_i64);
    let renamed = storage
        .execute("방이름", &mut body, "  새 방 이름  ", None, None, None)
        .unwrap();
    assert_eq!(renamed.0, vec!["방이 이름이 변경 되었습니다."]);
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["맵정보"]["이름"], "새 방 이름");
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .room_attrs
            .get(&format!("{zone}:1"))
            .unwrap()
            .get("이름")
            .unwrap(),
        "새 방 이름"
    );

    let description = storage
        .execute("방설명", &mut body, "무시", None, None, None)
        .unwrap();
    assert!(matches!(
        description.1,
        Some(CommandResult::RequestInput {
            ref prompt,
            state: PendingInput::RoomDescription { zone: ref z, ref room, ref lines }
        }) if prompt == "방 설명 작성을 마치시려면 '.' 를 입력하세요.\r\n:"
            && z == &zone && room == "1" && lines.is_empty()
    ));

    let numeric = storage
        .execute("속성추가", &mut body, "돌 숫자태그 -12", None, None, None)
        .unwrap();
    assert_eq!(numeric.0, vec!["☞ 속성이 추가 되었습니다."]);
    assert!(matches!(
        floor.lock().unwrap().get("숫자태그"),
        Value::Int(-12)
    ));
    let numeric_append = storage
        .execute("속성추가", &mut body, "돌 숫자태그 둘째", None, None, None)
        .unwrap();
    assert_eq!(numeric_append.0, vec!["☞ 속성추가를 실패했습니다."]);
    assert_eq!(floor.lock().unwrap().getInt("숫자태그"), -12);

    let numbered_mob = storage
        .execute("속성추가", &mut body, "2돌 몹태그 첫째", None, None, None)
        .unwrap();
    assert_eq!(numbered_mob.0, vec!["☞ 속성이 추가 되었습니다."]);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, "1")[0]
            .runtime_attrs
            .get("몹태그"),
        Some(&Value::String("첫째".into()))
    );
    let numbered_mob_remove = storage
        .execute("속성제거", &mut body, "2돌 몹태그 첫째", None, None, None)
        .unwrap();
    assert_eq!(numbered_mob_remove.0, vec!["☞ 속성이 제거 되었습니다."]);
    assert!(!get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room(&zone, "1")[0]
        .runtime_attrs
        .contains_key("몹태그"));

    let first = storage
        .execute("속성추가", &mut body, "돌 태그 첫째", None, None, None)
        .unwrap();
    assert_eq!(first.0, vec!["☞ 속성이 추가 되었습니다."]);
    assert_eq!(floor.lock().unwrap().getString("태그"), "첫째");
    let second = storage
        .execute("속성추가", &mut body, "돌 태그 둘째", None, None, None)
        .unwrap();
    assert_eq!(second.0, vec!["☞ 속성이 추가 되었습니다."]);
    assert_eq!(floor.lock().unwrap().getString("태그"), "첫째\r\n둘째");

    let removed = storage
        .execute("속성제거", &mut body, "돌 태그 첫째", None, None, None)
        .unwrap();
    assert_eq!(removed.0, vec!["☞ 속성이 제거 되었습니다."]);
    assert_eq!(floor.lock().unwrap().getString("태그"), "둘째");
    let absent = storage
        .execute("속성제거", &mut body, "돌 태그 없음", None, None, None)
        .unwrap();
    assert_eq!(absent.0, vec!["☞ 속성이 없습니다."]);

    body.set("태그", "자기값");
    let own_target_line = format!("{} 태그 자기값", body.get_name());
    let self_target = storage
        .execute("속성제거", &mut body, &own_target_line, None, None, None)
        .unwrap();
    assert_eq!(self_target.0, vec!["☞ 속성이 제거 되었습니다."]);
    assert_eq!(body.get_string("태그"), "");

    let mut carried = Object::new();
    carried.set("이름", "소지돌");
    carried.set("반응이름", "소지돌");
    carried.set("태그", "소지값");
    let carried = Arc::new(Mutex::new(carried));
    body.object.append(carried.clone());
    let inventory_target = storage
        .execute(
            "속성제거",
            &mut body,
            "소지돌 태그 소지값",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(inventory_target.0, vec!["☞ 그런 대상이 없어요!"]);
    assert_eq!(carried.lock().unwrap().getString("태그"), "소지값");

    get_world_state()
        .write()
        .unwrap()
        .get_room_attrs_mut(&zone, "1")
        .insert("태그".into(), "방값".into());
    let room_target = storage
        .execute("속성제거", &mut body, "방 태그 방값", None, None, None)
        .unwrap();
    assert_eq!(room_target.0, vec!["☞ 그런 대상이 없어요!"]);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .room_attrs
            .get(&format!("{zone}:1"))
            .and_then(|attrs| attrs.get("태그"))
            .map(String::as_str),
        Some("방값")
    );
    let missing = storage
        .execute("속성추가", &mut body, "없는돌 태그 값", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 대상이 없어요!"]);

    let other_player = format!("속성타인-{suffix}");
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&other_player, PlayerPosition::new(zone.clone(), "1".into()));
    let mut other_view = rhai::Map::new();
    other_view.insert("이름".into(), Dynamic::from(other_player.clone()));
    other_view.insert("반응이름".into(), Dynamic::from("속성타인별칭"));
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![Dynamic::from(other_view)],
    )]));
    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        "1",
        RoomObjectRef::Player(other_player.clone()),
    );
    body.temp_mut().insert(
        "_online_room_admin".into(),
        Value::String(
            serde_json::json!([{
                "name": other_player,
                "raw_attrs": {"표식": "첫째"}
            }])
            .to_string(),
        ),
    );
    let player_append = storage
        .execute(
            "속성추가",
            &mut body,
            "속성타인 표식 둘째",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(player_append.0, vec!["☞ 속성이 추가 되었습니다."]);
    assert_eq!(
        take_admin_set_player_value_request(&mut body),
        Some((
            other_player.clone(),
            "표식".to_string(),
            serde_json::Value::String("첫째\r\n둘째".to_string())
        ))
    );
    body.temp_mut().insert(
        "_online_room_admin".into(),
        Value::String(
            serde_json::json!([{
                "name": other_player,
                "raw_attrs": {"표식": "첫째\r\n둘째"}
            }])
            .to_string(),
        ),
    );
    let player_remove = storage
        .execute(
            "속성제거",
            &mut body,
            "속성타인 표식 첫째",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(player_remove.0, vec!["☞ 속성이 제거 되었습니다."]);
    assert_eq!(
        take_admin_set_player_value_request(&mut body),
        Some((
            other_player.clone(),
            "표식".to_string(),
            serde_json::Value::String("둘째".to_string())
        ))
    );
    clear_precomputed_room_view_players();

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.remove_player_position(&other_player);
    world.room_attrs.remove(&format!("{zone}:1"));
    world.remove_floor_item_record(&zone, "1", &floor);
    world.get_room_objs_mut(&zone, "1").clear();
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn guild_room_name_and_description_use_room_owner_and_persist_python_fields() {
    use crate::command::handler::{CommandResult, PendingInput};
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("방파방주-{suffix}");
    let guild = format!("방파방-{suffix}");
    let zone = format!("방파방존-{suffix}");
    let room = "1";
    let dir = format!("data/map/{zone}");
    let path = format!("{dir}/{room}.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &path,
        r#"{"맵정보":{"이름":"옛이름","설명":["옛설명"],"방파주인":"다른방파"}}"#,
    )
    .unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, room).unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.to_string()));
        world
            .get_room_attrs_mut(&zone, room)
            .insert("방파주인".into(), "다른방파".into());
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("직위", "방주");
    body.set("소속", guild.as_str());
    let scripts = ScriptStorage::default();

    body.set("직위", "방파인");
    let unauthorized = scripts
        .execute("방파방설명", &mut body, "작성", None, None, None)
        .unwrap();
    assert_eq!(unauthorized.0, vec!["☞ 방파의 방주만이 할 수 있습니다."]);
    body.set("직위", "방주");
    let usage = scripts
        .execute("방파방설명", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [이름] 방파방이름"]);

    let denied = scripts
        .execute("방파방이름", &mut body, "새이름", None, None, None)
        .unwrap();
    assert_eq!(
        denied.0,
        vec!["☞ 무림인은 아무곳에나 이름을 새기지 않는다네."]
    );

    get_world_state()
        .write()
        .unwrap()
        .get_room_attrs_mut(&zone, room)
        .insert("방파주인".into(), String::new());
    let unowned = scripts
        .execute("방파방설명", &mut body, "작성", None, None, None)
        .unwrap();
    assert_eq!(
        unowned.0,
        vec!["☞ 무림인은 아무곳에나 이름을 새기지 않는다네."]
    );

    get_world_state()
        .write()
        .unwrap()
        .get_room_attrs_mut(&zone, room)
        .insert("방파주인".into(), guild.clone());
    let too_long = scripts
        .execute("방파방이름", &mut body, &"가".repeat(21), None, None, None)
        .unwrap();
    assert_eq!(too_long.0, vec!["☞ 너무 길어요."]);
    let boundary_name = "가".repeat(20);
    let boundary = scripts
        .execute("방파방이름", &mut body, &boundary_name, None, None, None)
        .unwrap();
    assert_eq!(boundary.0, vec!["이름이 변경 되었습니다."]);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .room_cache
            .get_room_cached(&zone, room)
            .unwrap()
            .read()
            .unwrap()
            .name,
        boundary_name
    );
    let renamed = scripts
        .execute("방파방이름", &mut body, "새이름", None, None, None)
        .unwrap();
    assert_eq!(renamed.0, vec!["이름이 변경 되었습니다."]);
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["맵정보"]["이름"], "새이름");

    body.set("직위", "부방주");
    let description = scripts
        .execute("방파방설명", &mut body, "작성", None, None, None)
        .unwrap();
    assert!(matches!(
        description.1,
        Some(CommandResult::RequestInput {
            ref prompt,
            state: PendingInput::RoomDescription { ref zone, ref room, ref lines }
        }) if prompt == "방 설명 작성을 마치시려면 '.' 를 입력하세요.\r\n:"
            && zone == &format!("방파방존-{suffix}") && room == "1" && lines.is_empty()
    ));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.room_attrs.remove(&format!("{zone}:{room}"));
    drop(world);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn guild_nickname_updates_online_target_and_excludes_leader_from_group_echo() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let leader = format!("별호방주-{suffix}");
    let target = format!("별호대상-{suffix}");
    let guild = format!("별호방파-{suffix}");
    let zone = format!("별호존-{suffix}");
    let target_path = format!("data/user/{target}.json");
    let mut saved = Body::new();
    saved.set("이름", target.as_str());
    saved.set("소속", "오래된별호소속");
    assert!(save_body_to_json(&mut saved, &target_path));
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&leader, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let online = [(&leader, "방주", 900_i64), (&target, "방파인", 700_i64)]
        .into_iter()
        .map(|(name, position, hp)| {
            let mut player = rhai::Map::new();
            player.insert("이름".into(), Dynamic::from(name.clone()));
            player.insert("소속".into(), Dynamic::from(guild.clone()));
            player.insert("직위".into(), Dynamic::from(position));
            player.insert("zone".into(), Dynamic::from(zone.clone()));
            player.insert("room".into(), Dynamic::from("1"));
            player.insert("설정상태".into(), Dynamic::from(""));
            player.insert("현재체력".into(), Dynamic::from(hp));
            player.insert("최고체력".into(), Dynamic::from(hp));
            player.insert("현재내공".into(), Dynamic::from(10_i64));
            player.insert("최고내공".into(), Dynamic::from(10_i64));
            Dynamic::from(player)
        })
        .collect();
    set_precomputed_all_online(online);
    let mut target_view = rhai::Map::new();
    target_view.insert("이름".into(), Dynamic::from(target.clone()));
    target_view.insert("반응이름".into(), Dynamic::from("별호대상별칭"));
    target_view.insert("소속".into(), Dynamic::from(guild.clone()));
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![Dynamic::from(target_view)],
    )]));
    let mut body = Body::new();
    body.set("이름", leader.as_str());
    body.set("소속", guild.as_str());
    body.set("직위", "방주");
    body.set("소속", "");
    assert_eq!(
        ScriptStorage::default()
            .execute("방파별호", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 당신은 소속이 없습니다."]
    );
    body.set("소속", guild.as_str());
    body.set("직위", "부방주");
    assert_eq!(
        ScriptStorage::default()
            .execute("방파별호", &mut body, "별호대상 푸른별", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 방파의 방주만이 할 수 있습니다."]
    );
    body.set("직위", "방주");
    assert_eq!(
        ScriptStorage::default()
            .execute("방파별호", &mut body, "별호대상", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 사용법 : [대상] [무림별호] 방파별호"]
    );
    assert_eq!(
        ScriptStorage::default()
            .execute(
                "방파별호",
                &mut body,
                &format!("별호대상 {}", "가".repeat(11)),
                None,
                None,
                None,
            )
            .unwrap()
            .0,
        vec!["☞ 사용하시려는 별호가 너무 길어요."]
    );
    let whitespace = ScriptStorage::default()
        .execute("방파파문", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법 : [대상] 방파파문"]);
    let result = ScriptStorage::default()
        .execute("방파별호", &mut body, "별호대상 푸른별", None, None, None)
        .unwrap();
    assert_eq!(
        result.0.len(),
        1,
        "leader receives only the direct declaration"
    );
    assert!(
        result.0[0].contains("『\x1b[1;32m푸른별\x1b[0;37m』"),
        "{:?}",
        result.0
    );
    let sends = match result.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected result: {other:?}"),
    };
    assert_eq!(sends.len(), 1);
    assert_eq!(sends[0].0, target);
    assert!(!sends[0].1.starts_with("\r\n"));
    assert!(sends[0].1.contains(&format!("\x1b[1m{leader}\x1b[0;37m")));
    assert_eq!(
        take_guild_nickname_request(&mut body),
        Some((target.clone(), "푸른별".to_string()))
    );

    let mut collision = Object::new();
    collision.set("이름", leader.as_str());
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let self_item_first = ScriptStorage::default()
        .execute(
            "방파별호",
            &mut body,
            &format!("{leader} 청룡"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(self_item_first.0, vec!["☞ 이곳에 그런 무림인이 없습니다."]);
    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        "1",
        crate::world::RoomObjectRef::Player(leader.clone()),
    );

    let self_result = ScriptStorage::default()
        .execute(
            "방파별호",
            &mut body,
            &format!("{leader} 청룡"),
            None,
            None,
            None,
        )
        .unwrap();
    assert!(self_result.0[0].contains("\x1b[1m자신\x1b[0;37m의 방파별호"));
    assert_eq!(body.get_string("방파별호"), "청룡");
    assert_eq!(take_guild_nickname_request(&mut body), None);
    let self_sends = match self_result.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected self nickname result: {other:?}"),
    };
    assert_eq!(self_sends.len(), 1);
    assert_eq!(self_sends[0].0, target);
    assert!(self_sends[0].1.contains("\x1b[1m자신\x1b[0;37m의 방파별호"));

    clear_precomputed_all_online();
    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&leader);
    world.remove_player_position(&target);
    drop(world);
    let _ = std::fs::remove_file(target_path);
}

#[test]
fn guild_expulsion_updates_current_json_roster_and_python_deliveries() {
    use crate::command::handler::CommandResult;

    let suffix = std::process::id();
    let leader = format!("파문방주-{suffix}");
    let target = format!("파문대상-{suffix}");
    let guild = format!("파문방파-{suffix}");
    let target_path = format!("data/user/{target}.json");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    crate::world::guild::guild_set(&guild, "이름", &guild);
    crate::world::guild::guild_set(&guild, "방주리스트", &leader);
    crate::world::guild::guild_set(&guild, "방파인리스트", &target);
    crate::world::guild::guild_set(&guild, "방파원수", "2");
    let mut saved = Body::new();
    saved.set("이름", target.as_str());
    // Python uses the connected Player object.  A stale save must not
    // override the live target's current guild/position.
    saved.set("소속", "오래된소속");
    saved.set("직위", "방파인");
    saved.set("방파별호", "옛별호");
    assert!(save_body_to_json(&mut saved, &target_path));
    let online = [(&leader, 900_i64), (&target, 700_i64)]
        .into_iter()
        .map(|(name, hp)| {
            let mut player = rhai::Map::new();
            player.insert("이름".into(), Dynamic::from(name.clone()));
            player.insert("소속".into(), Dynamic::from(guild.clone()));
            player.insert(
                "직위".into(),
                Dynamic::from(if name == &leader {
                    "방주"
                } else {
                    "방파인"
                }),
            );
            player.insert("설정상태".into(), Dynamic::from(""));
            player.insert("현재체력".into(), Dynamic::from(hp));
            player.insert("최고체력".into(), Dynamic::from(hp));
            player.insert("현재내공".into(), Dynamic::from(10_i64));
            player.insert("최고내공".into(), Dynamic::from(10_i64));
            Dynamic::from(player)
        })
        .collect();
    set_precomputed_all_online(online);
    let mut body = Body::new();
    body.set("이름", leader.as_str());
    body.set("소속", guild.as_str());
    body.set("직위", "방주");
    let storage = ScriptStorage::default();
    body.set("직위", "부방주");
    let unauthorized = storage
        .execute("방파파문", &mut body, &target, None, None, None)
        .unwrap();
    assert_eq!(unauthorized.0, vec!["☞ 방파의 방주만이 할 수 있습니다."]);
    body.set("직위", "방주");
    let self_target = storage
        .execute("방파파문", &mut body, &leader, None, None, None)
        .unwrap();
    assert_eq!(self_target.0, vec!["☞ 자기 자신입니다."]);
    let absent_name = format!("존재하지않는파문대상-{suffix}");
    let _ = std::fs::remove_file(format!("data/user/{absent_name}.json"));
    let absent = storage
        .execute("방파파문", &mut body, &absent_name, None, None, None)
        .unwrap();
    assert_eq!(absent.0, vec!["☞ 그런 무림인은 아애 존재하지 않습니다."]);
    assert!(take_guild_kick_request(&mut body).is_none());

    let result = storage
        .execute("방파파문", &mut body, &target, None, None, None)
        .unwrap();
    assert_eq!(result.0.len(), 1);
    assert!(result.0[0].contains("방파에서 파문시킴을 선포합니다."));
    let sends = match result.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected expulsion result: {other:?}"),
    };
    assert_eq!(
        sends.len(),
        1,
        "expelled target only receives its private notice"
    );
    assert_eq!(sends[0].0, target);
    assert_eq!(
        sends[0].1,
        format!(
            "{}\r\n당신은 파문되었습니다.\r\n\r\n\x1b[0;37;40m[ 700/700, 10/10 ] ",
            RAW_USER_MESSAGE_PREFIX
        )
    );
    assert_eq!(take_guild_kick_request(&mut body), Some(target.clone()));
    let loaded: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&target_path).unwrap()).unwrap();
    assert_eq!(loaded["사용자오브젝트"]["소속"], "");
    assert_eq!(loaded["사용자오브젝트"]["직위"], "");
    assert!(loaded["사용자오브젝트"].get("방파별호").is_none());

    clear_precomputed_all_online();
    crate::world::guild::guild_remove(&guild);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
    let _ = std::fs::remove_file(target_path);
}

#[test]
fn guild_reset_clears_flat_saved_members_and_queues_live_cleanup() {
    let suffix = std::process::id();
    let admin = format!("초기화관리자-{suffix}");
    let member = format!("초기화방파원-{suffix}");
    let guild = format!("초기화방파-{suffix}");
    let member_path = format!("data/user/{member}.json");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    crate::world::guild::guild_set(&guild, "이름", &guild);
    crate::world::guild::guild_set(&guild, "방파원수", "1");
    let mut saved = Body::new();
    saved.set("이름", member.as_str());
    saved.set("소속", guild.as_str());
    saved.set("직위", "방파인");
    assert!(save_body_to_json(&mut saved, &member_path));

    let mut body = Body::new();
    body.set("이름", admin.as_str());
    body.set("관리자등급", 1999_i64);
    let unauthorized = ScriptStorage::default()
        .execute("방파초기화", &mut body, &guild, None, None, None)
        .unwrap();
    assert_eq!(unauthorized.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert!(crate::world::guild::guild_has(&guild));
    body.set("관리자등급", 2000_i64);
    let whitespace = ScriptStorage::default()
        .execute("방파초기화", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법: [방파이름] 방파초기화"]);
    assert!(crate::world::guild::guild_has(&guild));
    let absent = ScriptStorage::default()
        .execute(
            "방파초기화",
            &mut body,
            &format!("없는방파-{suffix}"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(absent.0, vec!["* 그런 방파가 없습니다."]);
    assert!(take_guild_reset_request(&mut body).is_none());
    let result = ScriptStorage::default()
        .execute("방파초기화", &mut body, &guild, None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["* 방파가 초기화되었습니다."]);
    assert!(!crate::world::guild::guild_has(&guild));
    assert_eq!(take_guild_reset_request(&mut body), Some(guild.clone()));
    let loaded: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&member_path).unwrap()).unwrap();
    assert_eq!(loaded["사용자오브젝트"]["소속"], "");
    assert_eq!(loaded["사용자오브젝트"]["직위"], "");

    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
    let _ = std::fs::remove_file(member_path);
}

#[test]
fn specific_guild_reset_removes_only_requested_guild_despite_python_clear_all_bug() {
    let suffix = std::process::id();
    let selected = format!("특정초기화-{suffix}");
    let preserved = format!("보존방파-{suffix}");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    crate::world::guild::guild_set(&selected, "이름", &selected);
    crate::world::guild::guild_set(&preserved, "이름", &preserved);
    let mut body = Body::new();
    body.set("관리자등급", 2000_i64);

    let empty = ScriptStorage::default()
        .execute("특정방파초기화", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(empty.0, vec!["초기화할 방파를 입력하세요."]);
    let whitespace = ScriptStorage::default()
        .execute("특정방파초기화", &mut body, " ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["초기화할 방파를 입력하세요."]);

    let result = ScriptStorage::default()
        .execute("특정방파초기화", &mut body, &selected, None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["* 방파가 초기화되었습니다."]);
    assert!(!crate::world::guild::guild_has(&selected));
    assert!(crate::world::guild::guild_has(&preserved));
    assert_eq!(take_guild_reset_request(&mut body), Some(selected.clone()));

    crate::world::guild::guild_remove(&preserved);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
}

#[test]
fn guild_leader_transfer_requires_same_room_deputy_and_moves_python_roster() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let leader = format!("양도방주-{suffix}");
    let target = format!("양도부방주-{suffix}");
    let guild = format!("양도방파-{suffix}");
    let zone = format!("양도존-{suffix}");
    let target_path = format!("data/user/{target}.json");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    crate::world::guild::guild_set(&guild, "이름", &guild);
    crate::world::guild::guild_set(&guild, "방주이름", &leader);
    crate::world::guild::guild_set(&guild, "부방주리스트", &target);
    let mut saved = Body::new();
    saved.set("이름", target.as_str());
    saved.set("소속", "오래된양도소속");
    saved.set("직위", "방파인");
    saved.set("레벨", 1_i64);
    assert!(save_body_to_json(&mut saved, &target_path));
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&leader, PlayerPosition::new(zone.clone(), "1".into()));
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&target, PlayerPosition::new(zone.clone(), "1".into()));
    let mut target_view = Body::new();
    target_view.set("이름", target.as_str());
    target_view.set("반응이름", "양도대상별칭");
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![build_room_view_player_snapshot(&target_view)],
    )]));
    let online = |target_level: i64| {
        [(&leader, "방주", 900_i64), (&target, "부방주", 700_i64)]
            .into_iter()
            .map(|(name, position, hp)| {
                let mut player = rhai::Map::new();
                player.insert("이름".into(), Dynamic::from(name.clone()));
                player.insert("소속".into(), Dynamic::from(guild.clone()));
                player.insert("직위".into(), Dynamic::from(position));
                player.insert(
                    "레벨".into(),
                    Dynamic::from(if name == &target {
                        target_level
                    } else {
                        999_999_i64
                    }),
                );
                player.insert("zone".into(), Dynamic::from(zone.clone()));
                player.insert("room".into(), Dynamic::from("1"));
                player.insert("설정상태".into(), Dynamic::from(""));
                player.insert("현재체력".into(), Dynamic::from(hp));
                player.insert("최고체력".into(), Dynamic::from(hp));
                player.insert("현재내공".into(), Dynamic::from(10_i64));
                player.insert("최고내공".into(), Dynamic::from(10_i64));
                Dynamic::from(player)
            })
            .collect()
    };
    set_precomputed_all_online(online(499));
    let mut body = Body::new();
    body.set("이름", leader.as_str());
    body.set("소속", guild.as_str());
    body.set("직위", "방주");
    body.set("반응이름", "양도방주별칭");

    body.set("직위", "부방주");
    let unauthorized = ScriptStorage::default()
        .execute("방주권한양도", &mut body, &target, None, None, None)
        .unwrap();
    assert_eq!(unauthorized.0, vec!["☞ 방파의 방주만이 할 수 있습니다."]);
    body.set("직위", "방주");
    let empty = ScriptStorage::default()
        .execute("방주권한양도", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(empty.0, vec!["☞ 사용법 : [대상] 방주권한양도"]);
    let self_target = ScriptStorage::default()
        .execute("방주권한양도", &mut body, "양도방주별칭", None, None, None)
        .unwrap();
    assert_eq!(self_target.0, vec!["☞ 이미 당신은 방주 입니다."]);

    let mut collision = Object::new();
    collision.set("이름", "양도충돌패");
    collision.set("반응이름", "양도대상별칭");
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let item_first = ScriptStorage::default()
        .execute("방주권한양도", &mut body, "양도대상별칭", None, None, None)
        .unwrap();
    assert_eq!(item_first.0, vec!["☞ 이곳에 그런 무림인이 없습니다."]);
    let numbered_player = ScriptStorage::default()
        .execute("방주권한양도", &mut body, "2양도대상별칭", None, None, None)
        .unwrap();
    assert_eq!(
        numbered_player.0,
        vec!["☞ 방주가 되기에는 역량이 부족합니다."]
    );
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, "1", &collision);
        world
            .get_room_objs_mut(&zone, "1")
            .retain(|item| !Arc::ptr_eq(item, &collision));
    }
    let low = ScriptStorage::default()
        .execute("방주권한양도", &mut body, "양도대상별칭", None, None, None)
        .unwrap();
    assert_eq!(low.0, vec!["☞ 방주가 되기에는 역량이 부족합니다."]);
    assert_eq!(body.get_string("직위"), "방주");
    assert_eq!(take_guild_transfer_request(&mut body), None);

    set_precomputed_all_online(online(999_999));
    let result = ScriptStorage::default()
        .execute("방주권한양도", &mut body, "양도대상별칭", None, None, None)
        .unwrap();
    assert_eq!(body.get_string("직위"), "부방주");
    assert_eq!(take_guild_transfer_request(&mut body), Some(target.clone()));
    assert_eq!(crate::world::guild::guild_get(&guild, "방주이름"), target);
    assert!(result.0[0].contains("방주로 권한이양을 선포합니다."));
    let sends = match result.1.unwrap() {
        crate::command::handler::CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected transfer result: {other:?}"),
    };
    assert_eq!(
        sends.len(),
        2,
        "target receives obj.lpPrompt then sendGroup prompt"
    );
    assert_eq!(sends[0].0, target);
    assert_eq!(
        sends[0].1,
        format!(
            "{}\r\n\x1b[0;37;40m[ 700/700, 10/10 ] ",
            RAW_USER_MESSAGE_PREFIX
        )
    );
    assert_eq!(sends[1].0, target);
    assert!(!sends[1].1.starts_with("\r\n"));
    assert!(sends[1].1.contains("방주로 권한이양을 선포합니다."));

    clear_precomputed_all_online();
    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&leader);
    world.remove_player_position(&target);
    drop(world);
    crate::world::guild::guild_remove(&guild);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
    let _ = std::fs::remove_file(target_path);
}

#[test]
fn guild_title_setting_persists_and_announces_with_new_sender_title() {
    let suffix = std::process::id();
    let leader = format!("명칭방주-{suffix}");
    let recipient = format!("명칭수신-{suffix}");
    let rejecting = format!("명칭거부-{suffix}");
    let outsider = format!("명칭타방파-{suffix}");
    let guild = format!("명칭방파-{suffix}");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    crate::world::guild::guild_set(&guild, "이름", &guild);
    crate::world::guild::guild_set(&guild, "방주명칭", "옛방주");
    let online = [
        (&leader, &guild, "", 900_i64, 10_i64),
        (&recipient, &guild, "", 700_i64, 12_i64),
        (&rejecting, &guild, "방파말거부 1", 600_i64, 11_i64),
        (&outsider, &"다른방파".to_string(), "", 500_i64, 9_i64),
    ]
    .into_iter()
    .map(|(name, affiliation, config, hp, mp)| {
        let mut player = rhai::Map::new();
        player.insert("이름".into(), Dynamic::from(name.clone()));
        player.insert("소속".into(), Dynamic::from(affiliation.clone()));
        player.insert("설정상태".into(), Dynamic::from(config));
        player.insert("현재체력".into(), Dynamic::from(hp));
        player.insert("최고체력".into(), Dynamic::from(hp));
        player.insert("현재내공".into(), Dynamic::from(mp));
        player.insert("최고내공".into(), Dynamic::from(mp));
        Dynamic::from(player)
    })
    .collect();
    set_precomputed_all_online(online);
    let mut body = Body::new();
    body.set("이름", leader.as_str());
    body.set("소속", guild.as_str());
    body.set("직위", "방주");
    let usage = ScriptStorage::default()
        .execute("명칭설정", &mut body, "제자 이름", None, None, None)
        .unwrap();
    assert_eq!(
        usage.0,
        vec!["☞ 사용법 : [방주|부방주|장로|방파인] [이름] 명칭설정"]
    );
    let result = ScriptStorage::default()
        .execute(
            "명칭설정",
            &mut body,
            "방주 대종사 무시되는말",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(crate::world::guild::guild_get(&guild, "방주명칭"), "대종사");
    let announcement = format!(
            "\x1b[1m《\x1b[36m대종사\x1b[37mː\x1b[36m{leader}\x1b[37m》\x1b[0;37m \x1b[1m{leader}\x1b[0;37m{} 방주의 명칭을 \x1b[1m대종사\x1b[0;37m로 변경하여 선포합니다.",
            han_iga(&leader)
        );
    assert_eq!(result.0, vec![announcement.clone()]);
    let sends = match result.1.unwrap() {
        crate::command::handler::CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected title announcement: {other:?}"),
    };
    assert_eq!(
        sends,
        vec![(
            recipient.clone(),
            format!("{announcement}\r\n\x1b[0;37;40m[ 700/700, 12/12 ] ")
        )]
    );
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("data/config/guild.json").unwrap()).unwrap();
    assert_eq!(saved[&guild]["방주명칭"], serde_json::json!("대종사"));

    clear_precomputed_all_online();
    crate::world::guild::guild_remove(&guild);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
}

#[test]
fn guild_signboard_matches_python_guard_order_schema_rooms_cost_item_and_notice() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("현판방주-{suffix}");
    let guild = format!("현판{suffix}");
    let zone = format!("현판존-{suffix}");
    let dir = format!("data/map/{zone}");
    let home_path = format!("{dir}/1.json");
    let entrance_path = format!("{dir}/2.json");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &home_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "맵정보": {"이름":"방파터", "방파자리":["가능"], "방파입구":["2"]}
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        &entrance_path,
        serde_json::to_string_pretty(&serde_json::json!({"맵정보":{"이름":"입구"}})).unwrap(),
    )
    .unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
    }
    let scripts = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("레벨", 399_i64);
    body.set("은전", 20_000_000_i64);

    get_world_state()
        .write()
        .unwrap()
        .get_room_attrs_mut(&zone, "1")
        .insert("방파주인".into(), "이미있는방파".into());
    let room_first = scripts
        .execute("현판걸어", &mut body, &guild, None, None, None)
        .unwrap();
    assert_eq!(room_first.0, vec!["☞ 이곳엔 현판을 걸 수 없습니다."]);
    get_world_state()
        .write()
        .unwrap()
        .get_room_attrs_mut(&zone, "1")
        .insert("방파주인".into(), "".into());

    let low = scripts
        .execute("현판걸어", &mut body, &guild, None, None, None)
        .unwrap();
    assert_eq!(low.0, vec!["☞ 당신은 방파를 세울 수 없습니다."]);
    body.set("레벨", 400_i64);
    body.set("방파금지", "금지");
    let banned = scripts
        .execute("현판걸어", &mut body, &guild, None, None, None)
        .unwrap();
    assert_eq!(banned.0, vec!["☞ 당신은 방파를 세울 수 없습니다."]);
    body.set("방파금지", "");
    body.set("은전", 9_999_999_i64);
    let poor = scripts
        .execute("현판걸어", &mut body, &guild, None, None, None)
        .unwrap();
    assert_eq!(
        poor.0,
        vec!["☞ 방파를 세우는데는 은전 10,000,000개 이상이 필요합니다."]
    );

    body.set("은전", 20_000_000_i64);
    body.set("성격", "정파");
    body.set("무림별호", "청협");
    let created = scripts
        .execute("현판걸어", &mut body, &guild, None, None, None)
        .unwrap();
    assert_eq!(
        created.0,
        vec!["당신이 현판을 세우는데 은전 10000000개를 사용합니다."]
    );
    assert_eq!(body.get_string("소속"), guild);
    assert_eq!(body.get_string("직위"), "방주");
    assert_eq!(body.get_int("은전"), 10_000_000);
    assert!(body.object.objs.iter().any(|item| {
        item.lock()
            .ok()
            .is_some_and(|item| item.getName() == "보관함")
    }));
    assert_eq!(crate::world::guild::guild_get(&guild, "방주이름"), player);
    assert_eq!(crate::world::guild::guild_get(&guild, "방파원수"), "1");
    assert_eq!(
        crate::world::guild::guild_get(&guild, "방파맵"),
        format!("{zone}:1")
    );
    for key in ["방주명칭", "부방주명칭", "장로명칭", "방파인명칭"] {
        assert!(!crate::world::guild::guild_get(&guild, key).is_empty());
    }
    for path in [&home_path, &entrance_path] {
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(json["맵정보"]["방파주인"], guild);
    }
    let notice = match created.1.unwrap() {
        CommandResult::Notice(text) => text,
        other => panic!("unexpected guild creation result: {other:?}"),
    };
    assert!(notice.starts_with("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n"));
    assert!(notice.contains(&format!(
            "[\x1b[1;32m청협\x1b[0;37m] \x1b[1;36m{player}\x1b[37m{} 방파 『{guild}』{} 창설했습니다.\x1b[0m",
            han_iga(&player),
            han_eul(&guild)
        )));
    assert!(notice.ends_with("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));

    crate::world::guild::guild_remove(&guild);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.room_attrs.remove(&format!("{zone}:1"));
    world.room_attrs.remove(&format!("{zone}:2"));
    drop(world);
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(format!("data/user/{player}.json"));
}
