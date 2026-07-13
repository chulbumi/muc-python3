use super::*;
#[test]
fn socket_command_sorts_host_name_pairs_and_right_aligns_python_host_column() {
    let mut body = Body::new();
    body.set("관리자등급", 999_i64);
    let denied = ScriptStorage::default()
        .execute("소켓", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    let entries = [("10.0.0.2", "나"), ("10.0.0.1", "다"), ("10.0.0.1", "가")]
        .into_iter()
        .map(|(host, name)| {
            let mut map = rhai::Map::new();
            map.insert("host".into(), Dynamic::from(host.to_string()));
            map.insert("이름".into(), Dynamic::from(name.to_string()));
            Dynamic::from(map)
        })
        .collect();
    set_precomputed_all_online(entries);
    body.set("관리자등급", 1000_i64);
    let output = ScriptStorage::default()
        .execute("소켓", &mut body, "", None, None, None)
        .unwrap();
    clear_precomputed_all_online();
    assert_eq!(
        output.0,
        vec!["\r\n        10.0.0.1 : 가, 다\r\n        10.0.0.2 : 나"]
    );

    set_precomputed_all_online(Vec::new());
    let empty = ScriptStorage::default()
        .execute("소켓", &mut body, "", None, None, None)
        .unwrap();
    clear_precomputed_all_online();
    assert_eq!(empty.0, vec![""]);
}
#[test]
fn find_object_command_matches_each_room_object_by_exact_name() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let zone = format!("찾아라시험존-{suffix}");
    let directory = Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("1.json"),
        serde_json::json!({"맵정보":{"이름":"찾아라 실제방","존이름":zone,"설명":[],"출구":[]}})
            .to_string(),
    )
    .unwrap();

    let wanted = format!("동명대상-{suffix}");
    let mut first = Object::new();
    first.set("이름", wanted.as_str());
    let first = Arc::new(Mutex::new(first));
    let mut second = Object::new();
    second.set("이름", wanted.as_str());
    let second = Arc::new(Mutex::new(second));
    let mut alias_only = Object::new();
    alias_only.set("이름", "다른이름");
    alias_only.set("반응이름", wanted.as_str());
    let alias_only = Arc::new(Mutex::new(alias_only));
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        for item in [&first, &second, &alias_only] {
            world.get_room_objs_mut(&zone, "1").push(item.clone());
            world.record_floor_item(&zone, "1", item);
        }
        world.set_player_position(&wanted, PlayerPosition::new(zone.clone(), "1".to_string()));
    }

    let mut admin = Body::new();
    admin.set("관리자등급", 2000_i64);
    let output = ScriptStorage::default()
        .execute("찾아라", &mut admin, &wanted, None, None, None)
        .unwrap()
        .0;
    assert_eq!(output, vec!["찾아라 실제방"; 3]);

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&wanted);
    world.get_room_objs_mut(&zone, "1").clear();
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn where_command_normalizes_exact_name_and_preserves_same_zone_order() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let viewer = format!("어디조회자-{suffix}");
    let target = format!("어디대상-{suffix}");
    let other = format!("어디타존-{suffix}");
    let zone = format!("어디시험존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for (room, name) in [("1", "조회방"), ("2", "대상방")] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보":{"이름":name,"존이름":zone,"설명":[],"출구":[]}})
                .to_string(),
        )
        .unwrap();
    }
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.room_cache.get_room(&zone, "2").unwrap();
        world.set_player_position(&viewer, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let entry = |name: &str, entry_zone: &str, room: &str, active: i64| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name.to_string()));
        map.insert("zone".into(), Dynamic::from(entry_zone.to_string()));
        map.insert("room".into(), Dynamic::from(room.to_string()));
        map.insert("active".into(), Dynamic::from(active));
        Dynamic::from(map)
    };
    let envless = entry("방없는사용자", "", "0", 1);
    let summoned = entry("사용자형소환몹", &zone, "2", 1);
    let inactive = entry("비활성사용자", &zone, "2", 0);
    set_precomputed_all_online(vec![
        entry(&target, &zone, "2", 1),
        entry(&viewer, &zone, "1", 1),
        entry(&other, "다른존", "1", 1),
        envless,
        inactive,
        summoned,
    ]);
    let mut body = Body::new();
    body.set("이름", viewer.as_str());
    let storage = ScriptStorage::default();
    let listed = storage
        .execute("어디", &mut body, "", None, None, None)
        .unwrap();
    let padded = |name: &str| format!("{name}{}", " ".repeat(10 - name.chars().count().min(10)));
    assert_eq!(
        listed.0,
        vec![
            format!("\x1b[1m{}\x1b[0;37m ▷ 대상방", padded(&target)),
            format!("\x1b[1m{}\x1b[0;37m ▷ 조회방", padded(&viewer)),
            format!("\x1b[1m{}\x1b[0;37m ▷ 대상방", padded("비활성사용자")),
            format!("\x1b[1m{}\x1b[0;37m ▷ 대상방", padded("사용자형소환몹")),
        ]
    );
    let spaced = storage
        .execute("어디", &mut body, &format!(" {target}"), None, None, None)
        .unwrap();
    assert_eq!(
        spaced.0,
        vec![format!("\x1b[1m{}\x1b[0;37m ▷ 대상방", padded(&target))]
    );
    let hidden = storage
        .execute("어디", &mut body, "방없는사용자", None, None, None)
        .unwrap();
    assert_eq!(hidden.0, vec!["☞ 활동중인 그런 무림인이 없어요. ^^"]);
    let inactive_named = storage
        .execute("어디", &mut body, "비활성사용자", None, None, None)
        .unwrap();
    assert_eq!(
        inactive_named.0,
        vec!["☞ 활동중인 그런 무림인이 없어요. ^^"],
        "Python only applies the ACTIVE guard to the named lookup branch"
    );
    let summoned_found = storage
        .execute("어디", &mut body, "사용자형소환몹", None, None, None)
        .unwrap();
    assert_eq!(
        summoned_found.0,
        vec![format!(
            "\x1b[1m{}\x1b[0;37m ▷ 대상방",
            padded("사용자형소환몹")
        )]
    );

    clear_precomputed_all_online();
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&viewer);
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn who_command_preserves_python_order_filters_and_exact_line_count() {
    let make =
        |name: &str, nick: &str, tendency: &str, reset: &str, guild: &str, transparent: i64| {
            let mut map = rhai::Map::new();
            map.insert("이름".into(), Dynamic::from(name.to_string()));
            map.insert("무림별호".into(), Dynamic::from(nick.to_string()));
            map.insert("성격".into(), Dynamic::from(tendency.to_string()));
            map.insert("레벨초기화".into(), Dynamic::from(reset.to_string()));
            map.insert("소속".into(), Dynamic::from(guild.to_string()));
            map.insert("투명상태".into(), Dynamic::from(transparent));
            Dynamic::from(map)
        };
    set_precomputed_all_online(vec![
        make("첫째", "", "", "", "청룡", 0),
        make("숨은자", "은자", "선인", "", "청룡", 1),
        make("둘째", "검성", "정파", "", "청룡", 0),
        make("셋째", "혈마", "사파", "1", "백호", 0),
    ]);
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "누구회귀");
    body.set("소속", "청룡");

    let all = storage
        .execute("누구", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(all.0.len(), 6, "Python emits no invented final blank line");
    assert!(all.0[3].find("첫째").unwrap() < all.0[3].find("둘째").unwrap());
    assert!(all.0[3].find("둘째").unwrap() < all.0[3].find("셋째").unwrap());
    assert!(!all.0[3].contains("숨은자"));
    assert!(all.0[3].contains("[\x1b[0;37m무명객\x1b[0;37m]"));
    assert!(all.0[3].contains("[\x1b[1;32m검성\x1b[0;37m]"));
    assert!(all.0[3].contains("<\x1b[0;31m혈마\x1b[0;37m>"));
    assert_eq!(
        all.0[3],
        "  [\x1b[0;37m무명객\x1b[0;37m]     첫째        [\x1b[1;32m검성\x1b[0;37m]       둘째        <\x1b[0;31m혈마\x1b[0;37m>       셋째      ",
        "Python fillSpace uses ANSI-stripped EUC-KR byte widths for all three columns"
    );
    assert_eq!(all.0[5], " ★ 총 3명의 무림인이 활동하고 있습니다.");

    let guild = storage
        .execute("누구", &mut body, "방파", None, None, None)
        .unwrap();
    assert_eq!(guild.0.len(), 6);
    assert!(guild.0[3].contains("첫째"));
    assert!(guild.0[3].contains("둘째"));
    assert!(!guild.0[3].contains("셋째"));
    assert_eq!(
        guild.0[5],
        " ★ 총 2명의 \x1b[1m【\x1b[36m청룡\x1b[37m】\x1b[0;37m파 무림인이 활동하고 있습니다."
    );
    clear_precomputed_all_online();
}

#[test]
fn find_object_rejects_non_admin_before_scanning_loaded_rooms() {
    let mut body = Body::new();
    body.set("관리자등급", 1999_i64);
    let result = ScriptStorage::default()
        .execute("찾아라", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
}

#[test]
fn item_and_armor_find_use_python_template_order_name_ansi_and_messages() {
    let mut body = Body::new();
    body.set("관리자등급", 1000_i64);
    let storage = ScriptStorage::default();
    let found = storage
        .execute("아이템찾기", &mut body, "간장검", None, None, None)
        .unwrap();
    assert_eq!(
        found.0,
        vec![
            "\x1b[0;36m간장검\x1b[37m : 77-5",
            "\x1b[0;36m간장검\x1b[37m : 77",
        ]
    );
    assert_eq!(
        storage
            .execute("아이템찾기", &mut body, "현철지륜", None, None, None)
            .unwrap()
            .0,
        vec!["\x1b[0;36m현철지륜\x1b[37m : 현철지륜"]
    );
    assert_eq!(
        storage
            .execute(
                "아이템찾기",
                &mut body,
                "절대없는아이템검색어",
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["☞ 찾으시는 아이템이 없습니다."]
    );
    assert_eq!(
        storage
            .execute("아이템찾기", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 운영자 명령: [아이템이름] 아이템찾기"]
    );

    let armor = storage
        .execute("방어구찾기", &mut body, "무시됨", None, None, None)
        .unwrap();
    assert!(armor
        .0
        .iter()
        .any(|line| line == "\x1b[0;36m비단머리띠\x1b[37m : 423"));
    let expected_count = std::fs::read_dir("data/item")
        .unwrap()
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .filter(|json| json["아이템정보"]["종류"] == "방어구")
        .count();
    assert_eq!(armor.0.len(), expected_count);
    assert!(armor.0.iter().all(|line| line.contains(" : ")));

    body.set("관리자등급", 999_i64);
    for command in ["아이템찾기", "방어구찾기"] {
        assert_eq!(
            storage
                .execute(command, &mut body, "무시", None, None, None)
                .unwrap()
                .0,
            vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]
        );
    }
}

#[test]
fn who_gives_item_preserves_python_mob_registration_order() {
    use crate::world::{get_world_state, RawMobData};

    let suffix = std::process::id();
    let item_key = format!("누가주나시험-{suffix}");
    let first_key = format!("순서시험A-{suffix}:첫째");
    let second_key = format!("순서시험B-{suffix}:둘째");
    let merchant_key = format!("순서시험B-{suffix}:상인");
    let duplicate_key = format!("순서시험A-{suffix}:중복");
    let mut first = RawMobData::new();
    first.name = "첫째몹".into();
    first.drop_items.push((item_key.clone(), 1, 100, 1));
    let mut second = RawMobData::new();
    second.name = "둘째몹".into();
    second.use_items.push((item_key.clone(), 1, 1, 1));
    let mut merchant = RawMobData::new();
    merchant.name = "판매만하는몹".into();
    merchant.items_for_sale.push((item_key.clone(), 100));
    let mut duplicate = RawMobData::new();
    duplicate.name = "중복몹".into();
    duplicate.drop_items.push((item_key.clone(), 1, 100, 1));
    duplicate.use_items.push((item_key.clone(), 1, 100, 1));
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(first_key.clone(), first);
        world.mob_cache.insert_mob_data(second_key.clone(), second);
        world
            .mob_cache
            .insert_mob_data(merchant_key.clone(), merchant);
        world
            .mob_cache
            .insert_mob_data(duplicate_key.clone(), duplicate);
    }

    let mut body = Body::new();
    body.set("이름", "누가주나회귀");
    let storage = ScriptStorage::default();
    let denied = storage
        .execute("누가주나", &mut body, &item_key, None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    body.set("관리자등급", 1000_i64);
    let usage = storage
        .execute("누가주나", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 운영자 명령: [아이템인덱스] 누가주나"]);
    let output = storage
        .execute("누가주나", &mut body, &item_key, None, None, None)
        .unwrap();
    assert_eq!(
        output.0,
        vec![
            format!("첫째몹 : {first_key}"),
            format!("중복몹 : {duplicate_key}"),
            format!("중복몹 : {duplicate_key}"),
            format!("둘째몹 : {second_key}")
        ]
    );
    let none = storage
        .execute("누가주나", &mut body, "없는아이템", None, None, None)
        .unwrap();
    assert!(none.0.is_empty());
    let mut world = get_world_state().write().unwrap();
    world.mob_cache.remove_mob(&first_key);
    world.mob_cache.remove_mob(&second_key);
    world.mob_cache.remove_mob(&merchant_key);
    world.mob_cache.remove_mob(&duplicate_key);
}
