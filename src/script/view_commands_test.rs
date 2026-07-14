use super::*;

#[test]
fn numeric_look_and_attack_resolve_the_same_first_room_mob() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player = format!("숫자대상일관성-{suffix}");
    let zone = format!("숫자대상일관성존-{suffix}");
    let room = "1";
    let mut ids = Vec::new();
    for label in ["먼저등록몹", "나중등록몹"] {
        let key = format!("{zone}:{label}");
        let mut data = RawMobData::new();
        data.name = label.into();
        data.mob_type = 1;
        data.max_hp = 100;
        let instance = MobInstance::new(key.clone(), zone.clone(), room, &data);
        ids.push((instance.instance_id, key));
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(ids.last().unwrap().1.clone(), data);
        world.mob_cache.add_mob_instance(instance);
        world.record_test_room_object(&zone, room, RoomObjectRef::Mob(ids.last().unwrap().0));
    }
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));

    let mut body = Body::new();
    body.set("이름", player.as_str());
    let attack_target = super::cast::find_cast_target(&body, "1").cast::<rhai::Map>();
    let attack_name = attack_target["name"].clone().into_string().unwrap();
    let looked = ScriptStorage::default()
        .execute("봐", &mut body, "1", None, None, None)
        .unwrap()
        .0;
    assert!(
        looked.iter().any(|line| line.contains(&attack_name)),
        "봐={looked:?}, 쳐 대상={attack_name}"
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, room) {
        mobs.clear();
    }
}

#[test]
fn room_look_does_not_turn_terminal_delimiter_into_extra_blank_output() {
    use crate::world::{get_world_state, PlayerPosition};

    let player = format!("봐끝개행-{}", std::process::id());
    {
        let mut world = get_world_state().write().unwrap();
        if world.room_cache.get_room_cached("낙양성", "42").is_none() {
            world.room_cache.preload_zone("낙양성").unwrap();
        }
        world.set_player_position(
            &player,
            PlayerPosition::new("낙양성".to_string(), "42".to_string()),
        );
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    let output = ScriptStorage::default()
        .execute("봐", &mut body, "", None, None, None)
        .unwrap()
        .0;
    assert!(!output.is_empty());
    assert_ne!(output.last().map(String::as_str), Some(""));

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
}

#[test]
fn compare_uses_integrated_item_mob_collision_order() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player = format!("비교충돌자-{suffix}");
    let zone = format!("비교충돌존-{suffix}");
    let room = "1";
    let key = format!("{zone}:몹");
    let mut data = RawMobData::new();
    data.name = "비교충돌몹".into();
    data.reaction_names = vec!["충돌별칭".into()];
    data.max_hp = 100;
    data.strength = 10;
    let instance = MobInstance::new(key.clone(), zone.clone(), room, &data);
    let mob_id = instance.instance_id;
    let mut item = Object::new();
    item.set("이름", "비교충돌물건");
    item.set("반응이름", "충돌별칭");
    let item = Arc::new(Mutex::new(item));
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data);
        world.mob_cache.add_mob_instance(instance);
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
        world.record_test_room_object(&zone, room, RoomObjectRef::Mob(mob_id));
        world.get_room_objs_mut(&zone, room).push(item.clone());
        world.record_floor_item(&zone, room, &item);
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("힘", 50_i64);
    body.set("최고내공", 100_i64);
    body.set("최고체력", 500_i64);
    let storage = ScriptStorage::default();

    let item_first = storage
        .execute("비교", &mut body, "충돌별칭", None, None, None)
        .unwrap();
    assert_eq!(item_first.0, vec!["자신의 상태를 통탄해 합니다. @_@"]);

    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        room,
        RoomObjectRef::Mob(mob_id),
    );
    let mob_first = storage
        .execute("비교", &mut body, "충돌별칭", None, None, None)
        .unwrap();
    assert_eq!(mob_first.0[1], "▶ \x1b[1m비교충돌몹\x1b[0;37m과의 상대비교");
    let numbered_item_second = storage
        .execute("비교", &mut body, "2충돌별칭", None, None, None)
        .unwrap();
    assert_eq!(
        numbered_item_second.0,
        vec!["자신의 상태를 통탄해 합니다. @_@"]
    );

    let target = format!("비교충돌대상-{suffix}");
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&target, PlayerPosition::new(zone.clone(), room.into()));
    let mut target_map = rhai::Map::new();
    for (key, value) in [
        ("이름", Dynamic::from(target.clone())),
        ("반응이름", Dynamic::from("충돌별칭")),
        ("zone", Dynamic::from(zone.clone())),
        ("room", Dynamic::from(room)),
        ("설정상태", Dynamic::from("")),
    ] {
        target_map.insert(key.into(), value);
    }
    for key in [
        "힘",
        "최고내공",
        "공격력",
        "숙련도차이",
        "맷집",
        "방어력",
        "최고체력",
    ] {
        target_map.insert(key.into(), Dynamic::from(100_i64));
    }
    set_precomputed_all_online(vec![Dynamic::from(target_map)]);
    let player_first = storage
        .execute("비교", &mut body, "충돌별칭", None, None, None)
        .unwrap();
    assert!(player_first.0[1].contains(&target));

    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        room,
        RoomObjectRef::Mob(mob_id),
    );
    let mob_before_player = storage
        .execute("비교", &mut body, "충돌별칭", None, None, None)
        .unwrap();
    assert!(mob_before_player.0[1].contains("비교충돌몹"));
    clear_precomputed_all_online();

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.remove_player_position(&target);
    world.get_room_objs_mut(&zone, room).clear();
    world.mob_cache.remove_instance(&zone, room, &key);
    world.mob_cache.remove_mob(&key);
}
#[test]
fn compare_command_targets_mob_and_restores_python_interface_and_table() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player = format!("비교회귀-{suffix}");
    let zone = format!("비교회귀존-{suffix}");
    let mob_key = format!("{zone}:상대");
    let hidden_key = format!("{zone}:숨김상대");
    let mut mob_data = RawMobData::new();
    mob_data.name = "비교허수아비".into();
    mob_data.zone = zone.clone();
    mob_data.max_hp = 1000;
    mob_data.strength = 20;
    mob_data.arm = 5;
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), mob_data.clone());
        let mut injured = MobInstance::new(mob_key.clone(), zone.clone(), "1", &mob_data);
        injured.hp = 100;
        world.mob_cache.add_mob_instance(injured);
        let mut hidden_data = RawMobData::new();
        hidden_data.name = "숨김허수아비".into();
        hidden_data.zone = zone.clone();
        hidden_data.mob_type = 7;
        hidden_data.max_hp = 9999;
        world
            .mob_cache
            .insert_mob_data(hidden_key.clone(), hidden_data.clone());
        world.mob_cache.add_mob_instance(MobInstance::new(
            hidden_key.clone(),
            zone.clone(),
            "1",
            &hidden_data,
        ));
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("힘", 100_i64);
    body.set("최고내공", 500_i64);
    body.set("최고체력", 2000_i64);
    body.attpower = 100;
    let storage = ScriptStorage::default();

    let usage = storage
        .execute("비교", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [대상] 비교"]);
    let missing = storage
        .execute("비교", &mut body, "없는대상", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["자신의 상태를 통탄해 합니다. @_@"]);
    let compared = storage
        .execute("비교", &mut body, "비교허수아비", None, None, None)
        .unwrap();
    assert_eq!(compared.0.len(), 7);
    assert_eq!(compared.0[0], "━━━━━━━━━━━━━━━");
    assert_eq!(
        compared.0[1],
        "▶ \x1b[1m비교허수아비\x1b[0;37m와의 상대비교"
    );
    assert!(compared.0[3].starts_with("☞ 당신의 승률 오차ː"));
    assert!(compared.0[4].starts_with("☞ 상대의 승률 오차ː"));
    let trailing_words = storage
        .execute("비교", &mut body, "비교허수아비 뒤의단어", None, None, None)
        .unwrap();
    assert_eq!(trailing_words.0[1], compared.0[1]);
    let opponent_rounds = compared.0[4]
        .strip_prefix("☞ 상대의 승률 오차ː")
        .unwrap()
        .parse::<i64>()
        .unwrap();
    assert!(
        (2..=3).contains(&opponent_rounds),
        "Python uses configured HP 1000, not injured runtime HP 100: {:?}",
        compared.0
    );
    let numbered = storage
        .execute("비교", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(
        numbered.0[1], "▶ \x1b[1m비교허수아비\x1b[0;37m와의 상대비교",
        "Python numeric lookup skips mob_type 7"
    );
    let hidden = storage
        .execute("비교", &mut body, "숨김허수아비", None, None, None)
        .unwrap();
    assert_eq!(hidden.0, vec!["☞ 그런 비교대상이 없어요. ^^"]);
    let dot = storage
        .execute("비교", &mut body, ".", None, None, None)
        .unwrap();
    assert_eq!(dot.0[1], numbered.0[1]);

    body.set("설정상태", "비교거부 1");
    let refused = storage
        .execute("비교", &mut body, "비교허수아비", None, None, None)
        .unwrap();
    assert_eq!(
        refused.0,
        vec!["☞ 진정한 승부란 비무를 통해서 알 수 있는 것 이지"]
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.mob_cache.remove_mob(&mob_key);
    world.mob_cache.remove_mob(&hidden_key);
}
#[test]
fn admin_look_at_mob_adds_python_index_and_runtime_table() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player = format!("봐관리자-{suffix}");
    let zone = format!("봐관리자존-{suffix}");
    let key = format!("{zone}:시험몹파일");
    let mut data = RawMobData::new();
    data.name = "관리자조회몹".into();
    data.zone = zone.clone();
    data.level = 25;
    data.max_hp = 500;
    data.inner_power = 80;
    data.strength = 31;
    data.arm = 12;
    data.agility = 17;
    data.skills = vec![
        ("가의신공".into(), 10, 100),
        ("고영신공".into(), 10, 100),
        ("존재하지않는무공".into(), 10, 100),
    ];
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        let mut instance = MobInstance::new(key.clone(), zone.clone(), "1", &data);
        instance.skills.push("고영신공".into());
        instance.skill_effects.push(crate::world::MobSkillEffect {
            name: "고영신공".into(),
            anti_type: "회복".into(),
            expires_at: chrono::Utc::now().timestamp() + 150,
            str_bonus: 0,
            dex_bonus: 0,
            arm_bonus: 0,
            mp_bonus: 0,
            max_mp_bonus: 0,
            hp_bonus: 0,
            max_hp_bonus: 0,
        });
        world.mob_cache.add_mob_instance(instance);
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 1000_i64);
    let output = ScriptStorage::default()
        .execute("봐", &mut body, "관리자조회몹", None, None, None)
        .unwrap()
        .0;
    assert_eq!(output[0], "Index : 시험몹파일");
    assert!(output
        .iter()
        .any(|line| line.contains("│ [레  벨]              25")));
    assert!(output.iter().any(|line| line.contains("500/500")));
    assert!(output
        .iter()
        .any(|line| line.contains("│ [맷  집]              12")));
    let attack = output
        .iter()
        .position(|line| line == "★ 공격 스킬 목록")
        .unwrap();
    assert!(output[attack + 1].contains("가의신공"));
    assert!(!output[attack + 1].contains("고영신공"));
    let other = output
        .iter()
        .position(|line| line == "★ 기타 스킬 목록")
        .unwrap();
    assert!(output[other + 1].contains("고영신공"));
    assert!(!output.iter().any(|line| line.contains("존재하지않는무공")));
    let active = output
        .iter()
        .position(|line| line == "★ 무공집결상태")
        .unwrap();
    assert!(output[active + 1].contains("고영신공"));
    assert!(output[active + 1].contains("전투체력상승"));
    assert_eq!(output.last().unwrap(), "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.mob_cache.remove_mob(&key);
}

#[test]
fn look_mob_matches_python_live_equipment_hp_and_corpse_inventory() {
    use crate::world::{MobInstance, RawMobData};

    let mut data = RawMobData::new();
    data.name = "상처입은검객".into();
    data.max_hp = 100;
    data.hp_display_type = "사람".into();
    data.desc2 = vec!["상처를 입은 검객입니다.".into()];
    data.use_items = vec![
        ("1-1".into(), 1, 100, 1),
        ("1-5".into(), 1, 100, 1),
        ("없는장비".into(), 1, 100, 1),
    ];
    let mut mob = MobInstance::new("시험존:검객".into(), "시험존".into(), "1", &data);
    mob.hp = 50;

    let live = mob_view(&mob, &data);
    assert_eq!(
        live[1],
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {:<47}\x1b[0m\x1b[37m\x1b[40m",
            "상처입은검객"
        )
    );
    assert!(live.iter().any(|line| line.contains("죽검")));
    assert!(live.iter().any(|line| line.contains("뇌전성적")));
    assert!(live.iter().any(|line| {
        line == "★ 상처입은검객이 신음소리를 내며 쓰러질것 같이 휘청 거립니다"
    }));
    assert!(live
        .iter()
        .any(|line| { line == &format!("☆ {} (50)", get_hp_bar_string(50, 100)) }));

    mob.alive = false;
    mob.act = 2;
    let mut first = Object::new();
    first.set("이름", "첫유품");
    let mut second = Object::new();
    second.set("이름", "둘째유품");
    mob.inventory.push(Arc::new(Mutex::new(first)));
    mob.inventory.push(Arc::new(Mutex::new(second)));
    let corpse = mob_view(&mob, &data);
    assert_eq!(
        corpse[1],
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {:<49}\x1b[0m\x1b[37m\x1b[40m",
            "상처입은검객의 시체"
        )
    );
    assert_eq!(
        corpse[3],
        "\x1b[36m첫유품\x1b[37m\r\n\x1b[36m둘째유품\x1b[37m"
    );
}
#[test]
fn look_command_renders_python_detailed_box_layout() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("봐상자회귀-{suffix}");
    let zone = format!("봐상자존-{suffix}");
    let mut box_object = Object::new();
    box_object.set("이름", "무기고");
    box_object.set("인덱스", "시험무기고");
    box_object.set("주인", player.as_str());
    box_object.set("보관수량", 3_i64);
    box_object.set("보관최대수량", 5_i64);
    box_object.set("보관증가은전", 100_i64);
    box_object.set("은전", 40_i64);
    let mut sword = Object::new();
    sword.set("이름", "청룡검");
    sword.set("옵션", "힘 +3");
    let mut unique = Object::new();
    unique.set("이름", "간장검");
    unique.set("아이템속성", "단일아이템");
    assert!(box_commands::prepare_installed_box(
        &mut box_object,
        &player,
        "무기고"
    ));
    box_object.objs.push(Arc::new(Mutex::new(sword)));
    box_object.objs.push(Arc::new(Mutex::new(unique)));
    box_commands::register_installed_box(&zone, "1", Arc::new(Mutex::new(box_object)));
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("관리자등급", 1000_i64);
    let mut inventory_collision = Object::new();
    inventory_collision.set("이름", "무기고");
    inventory_collision.set("반응이름", "무기고");
    inventory_collision.set("종류", "일반아이템");
    inventory_collision.set("설명2", "소지품의 무기고 모형입니다.");
    body.object
        .objs
        .push(Arc::new(Mutex::new(inventory_collision)));
    let inventory_first = ScriptStorage::default()
        .execute("봐", &mut body, "무기고", None, None, None)
        .unwrap()
        .0;
    assert!(inventory_first
        .iter()
        .any(|line| line == "소지품의 무기고 모형입니다."));
    assert!(!inventory_first
        .iter()
        .any(|line| line.starts_with("Index : ")));
    body.object.objs.clear();
    let output = ScriptStorage::default()
        .execute("봐", &mut body, "무기고", None, None, None)
        .unwrap()
        .0;
    assert_eq!(output[0], format!("Index : {player}_무기고"));
    assert_eq!(output[1], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    assert!(output[2].starts_with(&format!("\x1b[1m\x1b[44m\x1b[37m◁ {player}의 무기고 ▷")));
    assert!(output[4].contains("[   1] 청룡검 힘(3)"));
    assert!(output[4].contains("[   2] \x1b[1;36m간장검\x1b[0;37m"));
    assert_eq!(output[5], "───────────────────────────────────────");
    assert!(output[6].contains("◆ 수량 (2/3)  ◆ 최대수량 (5)  ◆ 확장에 필요한 은전 (40/100)"));
    assert_eq!(output[7], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_file(format!("data/box/{player}_무기고.json"));
}
#[test]
fn look_inventory_order_uses_python_bare_numeric_prefix() {
    let mut body = Body::new();
    body.set("이름", "봐순번검사");
    for kind in ["첫번째", "두번째"] {
        let mut item = Object::new();
        item.set("이름", "검");
        item.set("반응이름", "검");
        item.set("종류", kind);
        item.set("설명2", format!("{kind} 검입니다."));
        body.object.objs.push(Arc::new(Mutex::new(item)));
    }

    let output = ScriptStorage::default()
        .execute("봐", &mut body, "2검", None, None, None)
        .unwrap()
        .0;
    assert!(output.iter().any(|line| line.contains("◆ 종류 ▷ 두번째")));
    assert!(output.iter().any(|line| line == "두번째 검입니다."));
    assert!(!output.iter().any(|line| line == "첫번째 검입니다."));
}

#[test]
fn look_keeps_python_exact_and_prefix_match_counters_separate() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("봐분리카운터-{suffix}");
    let zone = format!("봐분리카운터존-{suffix}");
    let mut objects = Vec::new();
    for (name, reaction, description) in [
        ("접두물건", "검객", "접두 첫째"),
        ("검", "", "정확 첫째"),
        ("검", "", "정확 둘째"),
    ] {
        let mut item = Object::new();
        item.set("이름", name);
        item.set("반응이름", reaction);
        item.set("설명2", description);
        objects.push(Arc::new(Mutex::new(item)));
    }
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
        world.get_room_objs_mut(&zone, "1").extend(objects.clone());
        for object in objects.iter().rev() {
            world.record_floor_item(&zone, "1", object);
        }
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    let output = ScriptStorage::default()
        .execute("봐", &mut body, "2검", None, None, None)
        .unwrap()
        .0;
    assert!(output.iter().any(|line| line == "정확 둘째"));
    assert!(!output.iter().any(|line| line == "정확 첫째"));
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.get_room_objs_mut(&zone, "1").clear();
}

#[test]
fn look_item_matches_python_header_empty_description_defense_and_options() {
    let mut body = Body::new();
    body.set("이름", "봐아이템형식검사");
    let mut item = Object::new();
    item.set("이름", "빈설명검");
    item.set("반응이름", "빈설명검");
    item.set("종류", "무기");
    item.set("설명1", "Python은 이 설명을 대신 출력하지 않습니다.");
    item.set("설명2", "");
    body.object.objs.push(Arc::new(Mutex::new(item)));

    let empty = ScriptStorage::default()
        .execute("봐", &mut body, "빈설명검", None, None, None)
        .unwrap()
        .0;
    assert_eq!(
        empty,
        vec![
            "━━━━━━━━━━━━━━━━━━━━━".to_string(),
            format!(
                "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m",
                fill_space_euc_kr(42, "◆ 이름 ▷ 빈설명검")
            ),
            format!(
                "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m",
                fill_space_euc_kr(42, "◆ 종류 ▷ 무기")
            ),
            "─────────────────────".to_string(),
            String::new(),
            "━━━━━━━━━━━━━━━━━━━━━".to_string(),
        ]
    );

    let item = body.object.objs[0].clone();
    item.lock().unwrap().set("설명2", "첫 줄\n방어력 - 이전값");
    item.lock().unwrap().set("방어력", 37_i64);
    item.lock().unwrap().set("옵션", "힘 3");
    let detailed = ScriptStorage::default()
        .execute("봐", &mut body, "빈설명검", None, None, None)
        .unwrap()
        .0;
    assert_eq!(&detailed[4..7], ["첫 줄", "방어력 - 37", "힘(3)"]);
}

#[test]
fn look_self_matches_python_player_header_euc_kr_widths() {
    let mut body = Body::new();
    body.set("이름", "관찰자");
    body.set("무림별호", "푸른별");
    body.set("성격", "정파");
    body.set("배우자", "동반자");
    body.set("나이", 27_i64);
    body.set("성별", "여자");

    let output = ScriptStorage::default()
        .execute("봐", &mut body, "나", None, None, None)
        .unwrap()
        .0;
    assert_eq!(
        output[1],
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}{}\x1b[0m\x1b[37m\x1b[40m",
            fill_space_euc_kr(41, "◆ 이  름 ▷ 『푸른별』 관찰자"),
            fill_space_euc_kr(19, "◆ 성격 ▷ 『정파』")
        )
    );
    assert_eq!(
        output[2],
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}{}\x1b[0m\x1b[37m\x1b[40m",
            fill_space_euc_kr(41, "◆ 배우자 ▷ 『동반자』"),
            fill_space_euc_kr(19, "◆ 나이 ▷ 27살(여자)")
        )
    );
}

#[test]
fn look_self_uses_python_guild_title_mapping_and_character_padding() {
    let suffix = std::process::id();
    let guild = format!("조회방파-{suffix}");
    guild_set(&guild, "방주명칭", "천하방주");
    let mut body = Body::new();
    body.set("이름", "방파조회자");
    body.set("소속", guild.as_str());
    body.set("직위", "방주");
    body.set("방파별호", "청룡");

    let output = ScriptStorage::default()
        .execute("봐", &mut body, "나", None, None, None)
        .unwrap()
        .0;
    assert_eq!(
        output[3],
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{:<60}\x1b[0m\x1b[37m\x1b[40m",
            format!("■ 소  속 ▷ 『{guild}』")
        )
    );
    assert_eq!(
        output[4],
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{:<60}\x1b[0m\x1b[37m\x1b[40m",
            "■ 직  위 ▷ 『천하방주(청룡)』"
        )
    );
    guild_remove(&guild);
}
#[test]
fn look_command_uses_plain_python_failure_and_silences_missing_environment() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("봐실패회귀-{suffix}");
    let mut body = Body::new();
    body.set("이름", name.as_str());
    let storage = ScriptStorage::default();
    let no_environment = storage
        .execute("봐", &mut body, "없는것", None, None, None)
        .unwrap();
    assert!(no_environment.0.is_empty());

    get_world_state().write().unwrap().set_player_position(
        &name,
        PlayerPosition::new(format!("봐실패존-{suffix}"), "1".into()),
    );
    let missing = storage
        .execute("봐", &mut body, "없는것", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&name);
}

#[test]
fn look_does_not_invent_python_missing_exit_objects() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("봐출구검사-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room("낙양성", "42").unwrap();
        world.set_player_position(&player, PlayerPosition::new("낙양성".into(), "42".into()));
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    let output = ScriptStorage::default()
        .execute("봐", &mut body, "서", None, None, None)
        .unwrap()
        .0;
    assert_eq!(output, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);
    assert!(!output
        .iter()
        .any(|line| line.contains("쪽으로 갈 수 있습니다")));
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
}

#[test]
fn look_does_not_prefix_match_raw_mob_name_without_reaction_alias() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player = format!("봐이름접두검사-{suffix}");
    let zone = format!("봐이름접두존-{suffix}");
    let key = format!("{zone}:접두몹");
    let mut data = RawMobData::new();
    data.name = "부분이름몹".into();
    data.reaction_names = vec!["전혀다른별칭".into()];
    data.zone = zone.clone();
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        world
            .mob_cache
            .add_mob_instance(MobInstance::new(key.clone(), zone.clone(), "1", &data));
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", player.as_str());
    let storage = ScriptStorage::default();
    assert_eq!(
        storage
            .execute("봐", &mut body, "부분", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]
    );
    assert!(storage
        .execute("봐", &mut body, "전혀다른", None, None, None)
        .unwrap()
        .0
        .iter()
        .any(|line| line.contains("부분이름몹")));
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.mob_cache.remove_mob(&key);
}

#[test]
fn look_player_uses_reaction_alias_visibility_and_python_raw_prompt() {
    use crate::command::handler::CommandResult;
    use crate::world::{get_world_state, PlayerPosition, RoomObjectRef};
    use std::collections::HashMap;
    use std::sync::Arc;

    let suffix = std::process::id();
    let viewer = format!("봐사람관찰자-{suffix}");
    let target = format!("봐사람대상-{suffix}");
    let zone = format!("봐사람존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&viewer, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "1".into()));
        world.record_test_room_object(&zone, "1", RoomObjectRef::Player(target.clone()));
    }
    let mut target_body = Body::new();
    target_body.set("이름", target.as_str());
    target_body.set("반응이름", "푸른검객 청의검객");
    target_body.set("체력", 321_i64);
    target_body.set("최고체력", 456_i64);
    target_body.set("내공", 12_i64);
    target_body.set("최고내공", 34_i64);
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![build_room_view_player_snapshot(&target_body)],
    )]));
    let descriptions = HashMap::from([(target.clone(), "대상 설명 원문".to_string())]);
    let lookup = Arc::new(move || descriptions.clone());
    let mut body = Body::new();
    body.set("이름", viewer.as_str());
    let result = ScriptStorage::default()
        .execute("봐", &mut body, "푸른", None, Some(lookup), None)
        .unwrap();
    assert_eq!(result.0, vec!["대상 설명 원문"]);
    let actor = format!("\x1b[33m{viewer}\x1b[37m{}", han_iga(&viewer));
    let sends = match result.1 {
        Some(CommandResult::OutputAndSendToUsers(_, sends)) => sends,
        other => panic!("unexpected result: {other:?}"),
    };
    assert_eq!(
        sends,
        vec![(
            target.clone(),
            format!("{RAW_USER_MESSAGE_PREFIX}\r\n{actor} 당신을 살펴봅니다.\r\n\r\n\x1b[0;37;40m[ 321/456, 12/34 ] ")
        )]
    );

    target_body.set("투명상태", 1_i64);
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![build_room_view_player_snapshot(&target_body)],
    )]));
    let hidden_descriptions = HashMap::from([(target.clone(), "보이면안됨".to_string())]);
    let hidden_lookup = Arc::new(move || hidden_descriptions.clone());
    assert_eq!(
        ScriptStorage::default()
            .execute("봐", &mut body, "푸른", None, Some(hidden_lookup), None)
            .unwrap()
            .0,
        vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]
    );
    clear_precomputed_room_view_players();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&viewer);
    world.remove_player_position(&target);
}

#[test]
fn look_guard_syntax_reuses_exact_guard_command_presentation() {
    let mut body = Body::new();
    body.set("이름", "호위보기검사");
    let storage = ScriptStorage::default();
    let direct = storage
        .execute("호위", &mut body, "", None, None, None)
        .unwrap();
    let through_look = storage
        .execute("봐", &mut body, "호위", None, None, None)
        .unwrap();
    assert_eq!(direct, through_look);
    assert_eq!(
        through_look.0,
        vec!["당신은 호위를 거느리지 않고 있습니다."]
    );
}

#[test]
fn look_regular_box_groups_names_in_python_first_seen_order() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("봐일반상자-{suffix}");
    let zone = format!("봐일반상자존-{suffix}");
    let mut box_object = Object::new();
    box_object.set("이름", "보관함");
    box_object.set("인덱스", "시험보관함");
    box_object.set("주인", player.as_str());
    box_object.set("보관수량", 5_i64);
    box_object.set("보관최대수량", 5_i64);
    for (name, index) in [("청옥", "289"), ("백옥", "423"), ("청옥", "289")] {
        let mut item = Object::new();
        item.set("이름", name);
        item.set("인덱스", index);
        box_object.objs.push(Arc::new(Mutex::new(item)));
    }
    assert!(box_commands::prepare_installed_box(
        &mut box_object,
        &player,
        "보관함"
    ));
    box_commands::register_installed_box(&zone, "1", Arc::new(Mutex::new(box_object)));
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    let mut body = Body::new();
    body.set("이름", player.as_str());
    let output = ScriptStorage::default()
        .execute("봐", &mut body, "보관함", None, None, None)
        .unwrap()
        .0;
    assert!(output[3].contains("·\x1b[0;36m청옥 2개"));
    assert!(output[3].find("청옥").unwrap() < output[3].find("백옥").unwrap());
    assert!(output[5].contains("◆ 수량 (3/5)"));
    assert!(!output[5].contains("최대수량"));
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_file(format!("data/box/{player}_보관함.json"));
}

#[test]
fn look_boxes_wrap_long_euc_kr_cells_like_python() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let player = format!("봐장문상자-{suffix}");
    let zone = format!("봐장문상자존-{suffix}");
    let long_name = "가나다라마바사아자차카타파하가나다라마바사";
    for (box_name, detailed) in [("무기고", true), ("보관함", false)] {
        let mut box_object = Object::new();
        box_object.set("이름", box_name);
        box_object.set("인덱스", format!("{player}_{box_name}"));
        box_object.set("주인", player.as_str());
        box_object.set("보관수량", 5_i64);
        box_object.set("보관최대수량", 5_i64);
        for (index, name) in ["짧은검", long_name, "끝검"].into_iter().enumerate() {
            let mut item = Object::new();
            item.set("이름", name);
            item.set("인덱스", format!("장문시험-{index}"));
            box_object.objs.push(Arc::new(Mutex::new(item)));
        }
        assert!(box_commands::prepare_installed_box(
            &mut box_object,
            &player,
            box_name
        ));
        box_commands::register_installed_box(&zone, "1", Arc::new(Mutex::new(box_object)));
        get_world_state()
            .write()
            .unwrap()
            .set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
        let mut body = Body::new();
        body.set("이름", player.as_str());
        let output = ScriptStorage::default()
            .execute("봐", &mut body, box_name, None, None, None)
            .unwrap()
            .0;
        let cells = &output[3];
        assert!(cells.contains("\r\n"));
        if detailed {
            assert!(cells.starts_with(&fill_space_euc_kr(38, "[   1] 짧은검 ")));
            assert!(cells.contains(&format!("\r\n[   2] {long_name} \r\n")));
            assert!(cells.ends_with(&fill_space_euc_kr(38, "[   3] 끝검 ")));
        } else {
            let short = fill_space_euc_kr(22, "\x1b[1;36m·\x1b[0;36m짧은검\x1b[0;37m");
            assert!(cells.starts_with(&short));
            assert!(cells.contains(&format!(
                "\r\n\x1b[1;36m·\x1b[0;36m{long_name}\x1b[0;37m\r\n"
            )));
            assert_eq!(cells.chars().count(), {
                let untrimmed = format!(
                    "{}\r\n{}\r\n{}",
                    short,
                    format!("\x1b[1;36m·\x1b[0;36m{long_name}\x1b[0;37m"),
                    fill_space_euc_kr(22, "\x1b[1;36m·\x1b[0;36m끝검\x1b[0;37m")
                );
                untrimmed.chars().count() - 2
            });
        }
    }
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player);
    let _ = std::fs::remove_file(format!("data/box/{player}_무기고.json"));
    let _ = std::fs::remove_file(format!("data/box/{player}_보관함.json"));
}
