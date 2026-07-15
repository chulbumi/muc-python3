use super::*;

#[test]
fn shop_commands_match_python_valid_quantity_and_item_name_format() {
    use crate::world::{get_world_state, PlayerPosition};

    let player_name = format!("상점회귀-{}", std::process::id());
    let observer_name = format!("상점관찰-{}", std::process::id());
    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("힘", 100_i64);
    body.set("은전", 100_i64);
    let mut observer = Body::new();
    observer.set("이름", observer_name.as_str());
    observer.set("설정상태", "타인전투출력거부 0");
    observer.set("체력", 23_i64);
    observer.set("최고체력", 34_i64);
    observer.set("내공", 5_i64);
    observer.set("최고내공", 6_i64);
    set_cast_room_players(vec![CastRoomPlayerRef::new(&mut observer)]);
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room("낙양성", "6").unwrap();
        world.set_player_position(
            &player_name,
            PlayerPosition::new("낙양성".to_string(), "6".to_string()),
        );
        world.set_player_position(
            &observer_name,
            PlayerPosition::new("낙양성".to_string(), "6".to_string()),
        );
        world.spawn_mobs_for_room("낙양성", "6");
    }

    let storage = ScriptStorage::default();
    let bought = storage
        .execute("구입", &mut body, "수박모자 0개", None, None, None)
        .unwrap();
    assert_eq!(body.object.inv_stack.get("361"), Some(&1));
    assert!(body.object.objs.is_empty());
    assert_eq!(body.get_int("은전"), 91);
    assert_eq!(
        bought.0.join("\r\n"),
        "당신이 \x1b[0;36m수박모자\x1b[37m 1개를 은전 9개에 구입합니다."
    );
    assert!(!bought.0.join("\r\n").contains("수박모자를 1개를"));
    let bought_sends = match bought.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected buy delivery: {other:?}"),
    };
    assert_eq!(bought_sends.len(), 1);
    assert_eq!(bought_sends[0].0, observer_name);
    assert_eq!(
        bought_sends[0].1,
        format!(
            "{}\r\n\x1b[1m{}\x1b[0;37m{} \x1b[0;36m수박모자\x1b[37m 1개를 은전 9개에 구입합니다.\r\n\r\n\x1b[0;37;40m[ 23/34, 5/6 ] ",
            RAW_USER_MESSAGE_PREFIX,
            player_name,
            han_iga(&player_name),
        )
    );

    body.set(
        ALIAS_LIST_ATTR,
        encode_alias_entries(&[
            ("체력약".into(), "수박모자".into()),
            ("체력약개수".into(), "3개".into()),
        ]),
    );
    let auto_bought = storage
        .execute("구입", &mut body, "체력약", None, None, None)
        .unwrap();
    assert_eq!(body.object.inv_stack.get("361"), Some(&3));
    assert_eq!(body.get_int("은전"), 73);
    assert_eq!(
        auto_bought.0.join("\r\n"),
        "당신이 \x1b[0;36m수박모자\x1b[37m 2개를 은전 18개에 구입합니다."
    );
    let enough = storage
        .execute("구입", &mut body, "체력약", None, None, None)
        .unwrap();
    assert_eq!(enough.0, vec!["☞ 구매할 물품이 충분합니다. ^_^"]);

    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room("낙양성", "43").unwrap();
        world.set_player_position(
            &player_name,
            PlayerPosition::new("낙양성".to_string(), "43".to_string()),
        );
        world.set_player_position(
            &observer_name,
            PlayerPosition::new("낙양성".to_string(), "43".to_string()),
        );
        world.spawn_mobs_for_room("낙양성", "43");
    }
    let mut single_colored = Object::new();
    single_colored.set("이름", "자주판매품");
    single_colored.set("안시", "\x1b[1;35m");
    single_colored.set("판매가격", 100_i64);
    body.object
        .objs
        .insert(0, Arc::new(Mutex::new(single_colored)));
    let single_sold = storage
        .execute("판매", &mut body, "자주판매품", None, None, None)
        .unwrap();
    assert_eq!(
        single_sold.0,
        vec!["당신이 \x1b[1;35m자주판매품\x1b[0;37m 1개를 은전 40개에 판매합니다."]
    );
    let sold_sends = match single_sold.1.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected sell delivery: {other:?}"),
    };
    assert_eq!(sold_sends.len(), 1);
    assert_eq!(sold_sends[0].0, observer_name);
    assert_eq!(
        sold_sends[0].1,
        format!(
            "{}\r\n\x1b[1m{}\x1b[0;37m{} \x1b[1;35m자주판매품\x1b[0;37m 1개를 은전 40개에 판매합니다.\r\n\r\n\x1b[0;37;40m[ 23/34, 5/6 ] ",
            RAW_USER_MESSAGE_PREFIX,
            player_name,
            han_iga(&player_name),
        )
    );

    let mut colored = Object::new();
    colored.set("이름", "붉은판매품");
    colored.set("안시", "\x1b[1;31m");
    colored.set("판매가격", 100_i64);
    body.object.objs.insert(0, Arc::new(Mutex::new(colored)));
    let sold = storage
        .execute("판매", &mut body, " 모두 ", None, None, None)
        .unwrap();
    assert!(body.object.objs.is_empty());
    assert_eq!(sold.0.len(), 4);
    assert_eq!(
        sold.0[0],
        "당신이 \x1b[1;31m붉은판매품\x1b[0;37m 1개를 은전 40개에 판매합니다."
    );
    assert_eq!(
        &sold.0[1..],
        &["당신이 \x1b[0;36m수박모자\x1b[37m 1개를 은전 3개에 판매합니다."; 3]
    );

    let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player_name);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&observer_name);
    clear_cast_room_players();
}

#[test]
fn shop_list_uses_first_python_room_merchant_even_when_buy_only() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

    let suffix = std::process::id();
    let player = format!("품목표회귀-{suffix}");
    let zone = format!("품목표회귀존-{suffix}");
    let seller_key = format!("{zone}:판매상인");
    let buyer_key = format!("{zone}:매입상인");
    let mut seller = RawMobData::new();
    seller.name = "판매상인".into();
    seller.zone = zone.clone();
    seller.items_for_sale = vec![("361".into(), 100)];
    seller.sale_script = vec!["첫째 품목줄".into(), "둘째 품목줄".into()];
    let mut buyer = RawMobData::new();
    buyer.name = "매입상인".into();
    buyer.zone = zone.clone();
    buyer.buy_percent = 40;
    let (seller_id, buyer_id) = {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(seller_key.clone(), seller.clone());
        world
            .mob_cache
            .insert_mob_data(buyer_key.clone(), buyer.clone());
        let seller_instance = MobInstance::new(seller_key.clone(), zone.clone(), "1", &seller);
        let seller_id = seller_instance.instance_id;
        let buyer_instance = MobInstance::new(buyer_key.clone(), zone.clone(), "1", &buyer);
        let buyer_id = buyer_instance.instance_id;
        world.mob_cache.add_mob_instance(seller_instance);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(seller_id));
        world.mob_cache.add_mob_instance(buyer_instance);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(buyer_id));
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
        (seller_id, buyer_id)
    };
    let mut body = Body::new();
    body.set("이름", player.as_str());
    let storage = ScriptStorage::default();

    let blocked = storage
        .execute("품목표", &mut body, "무시되는 입력", None, None, None)
        .unwrap();
    assert_eq!(blocked.0, vec!["☞ 품목을 보여줄 상인이 없어요. ^^"]);
    body.set("은전", 100_i64);
    body.set("힘", 100_i64);
    let blocked_purchase = storage
        .execute("구입", &mut body, "수박모자", None, None, None)
        .unwrap();
    assert_eq!(blocked_purchase.0, vec!["☞ 그런 물건은 팔지 않아요. ^_^"]);
    assert!(body.object.objs.is_empty());

    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        "1",
        RoomObjectRef::Mob(seller_id),
    );
    let shown = storage
        .execute("품목표", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(shown.0, vec!["첫째 품목줄", "둘째 품목줄"]);

    let mut sale_item = Object::new();
    sale_item.set("이름", "판매시도품");
    sale_item.set("판매가격", 100_i64);
    body.object.objs.push(Arc::new(Mutex::new(sale_item)));
    let first_merchant_does_not_buy = storage
        .execute("판매", &mut body, "판매시도품", None, None, None)
        .unwrap();
    assert_eq!(
        first_merchant_does_not_buy.0,
        vec!["☞ 그런 물건을 살 상인이 없어요. ^_^"],
    );
    assert_eq!(body.object.objs.len(), 1);

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, "1") {
        mobs.retain(|mob| mob.instance_id != seller_id && mob.instance_id != buyer_id);
    }
    world.mob_cache.remove_mob(&seller_key);
    world.mob_cache.remove_mob(&buyer_key);
}

#[test]
fn purchase_keeps_python_find_merchant_dead_mob_behavior() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player = format!("사망상인구입-{suffix}");
    let zone = format!("사망상인구입존-{suffix}");
    let key = format!("{zone}:사망상인");
    let mut data = RawMobData::new();
    data.name = "쓰러진상인".into();
    data.zone = zone.clone();
    data.items_for_sale = vec![("361".into(), 100)];
    let mut merchant = MobInstance::new(key.clone(), zone.clone(), "1", &data);
    merchant.kill();
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data);
        world.mob_cache.add_mob_instance(merchant);
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }

    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("힘", 100_i64);
    body.set("은전", 100_i64);
    let result = ScriptStorage::default()
        .execute("구입", &mut body, "수박모자", None, None, None)
        .unwrap();
    assert_eq!(
        result.0,
        vec!["당신이 \x1b[0;36m수박모자\x1b[37m 1개를 은전 9개에 구입합니다."]
    );
    assert_eq!(body.get_int("은전"), 91);
    assert_eq!(body.object.inv_stack.get("361"), Some(&1));
    assert!(body.object.objs.is_empty());

    let _ = std::fs::remove_file(format!("data/user/{player}.json"));
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.mob_cache.remove_mob(&key);
}

#[test]
fn selling_skips_equipped_items_for_order_and_rejects_protected_stack_templates() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player = format!("판매순번-{suffix}");
    let zone = format!("판매순번존-{suffix}");
    let buyer_key = format!("{zone}:매입상인");
    let mut buyer = RawMobData::new();
    buyer.name = "매입상인".into();
    buyer.zone = zone.clone();
    buyer.buy_percent = 40;
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(buyer_key.clone(), buyer.clone());
        world.mob_cache.add_mob_instance(MobInstance::new(
            buyer_key.clone(),
            zone.clone(),
            "1",
            &buyer,
        ));
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let make = |price: i64, worn: bool| {
        let mut item = Object::new();
        item.set("이름", "순번검");
        item.set("반응이름", "순번검 긴순번검");
        item.set("판매가격", price);
        item.set("inUse", i64::from(worn));
        Arc::new(Mutex::new(item))
    };
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.object.objs.push(make(900, true));
    body.object.objs.push(make(100, false));
    body.object.objs.push(make(200, false));
    let storage = ScriptStorage::default();

    let second_unworn = storage
        .execute("판매", &mut body, "2순번검", None, None, None)
        .unwrap();
    assert_eq!(
        second_unworn.0,
        vec!["당신이 \x1b[0;36m순번검\x1b[37m 1개를 은전 80개에 판매합니다."]
    );
    assert_eq!(body.object.objs.len(), 2);
    assert!(body.object.objs[0].lock().unwrap().getBool("inUse"));
    assert_eq!(body.object.objs[1].lock().unwrap().getInt("판매가격"), 100);

    let (changed_hat, _) = super::object_from_item_json("361").unwrap();
    changed_hat.lock().unwrap().set("판매가격", 100_i64);
    body.object.objs.push(changed_hat);
    body.object.inv_stack.insert("361".into(), 2);
    let before_mixed_sale = body.get_int("은전");
    let mixed_sale = storage
        .execute("판매", &mut body, "수박모자 3", None, None, None)
        .unwrap();
    assert_eq!(
        mixed_sale.0,
        vec!["당신이 \x1b[0;36m수박모자\x1b[37m 3개를 은전 46개에 판매합니다."]
    );
    assert_eq!(body.get_int("은전"), before_mixed_sale + 46);
    assert!(!body.object.inv_stack.contains_key("361"));
    assert!(!body
        .object
        .objs
        .iter()
        .any(|item| { item.lock().is_ok_and(|item| item.getName() == "수박모자") }));

    body.object.inv_stack.insert("토령시".into(), 1);
    let protected = storage
        .execute("판매", &mut body, "토령시", None, None, None)
        .unwrap();
    assert_eq!(protected.0, vec!["☞ 그 아이템은 팔 수가 없어요~"]);
    assert_eq!(body.object.inv_stack.get("토령시"), Some(&1));

    body.object.inv_stack.insert("361".into(), 2);
    let all = storage
        .execute("판매", &mut body, "모두", None, None, None)
        .unwrap();
    assert_eq!(
        all.0
            .iter()
            .filter(|line| line.contains("수박모자"))
            .count(),
        2,
        "Python batch sale traverses every individual item"
    );
    assert_eq!(body.object.inv_stack.get("토령시"), Some(&1));
    assert!(!body.object.inv_stack.contains_key("361"));
    assert!(body
        .object
        .objs
        .iter()
        .any(|item| item.lock().is_ok_and(|item| item.getBool("inUse"))));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.mob_cache.remove_mob(&buyer_key);
    let _ = std::fs::remove_file(format!("data/user/{player}.json"));
}

#[test]
fn guard_purchase_consumes_python_requirement_without_charging_silver() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player_name = format!("호위구매-{suffix}");
    let zone = format!("호위구매존-{suffix}");
    let mob_key = format!("{zone}:상인");
    let mut merchant = RawMobData::new();
    merchant.name = "호위상인".into();
    merchant.zone = zone.clone();
    merchant.items_for_sale = vec![("명견".into(), 100), ("사강시".into(), 100)];
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), merchant.clone());
        world.mob_cache.add_mob_instance(MobInstance::new(
            mob_key.clone(),
            zone.clone(),
            "1",
            &merchant,
        ));
        world.set_player_position(&player_name, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("성격", "정사");
    body.set("은전", 1234_i64);
    let (herb, _) = object_from_item_json("합성1").expect("herb fixture");
    body.object.append(herb);
    let storage = ScriptStorage::default();
    let bought = storage
        .execute("구입", &mut body, "명견", None, None, None)
        .unwrap();
    assert_eq!(
        bought.0,
        vec!["당신이 \x1b[0;36m명견\x1b[37m을 구입합니다."]
    );
    assert_eq!(body.get_int("은전"), 1234);
    assert_eq!(
        body.object.objs.len(),
        1,
        "약초 1개를 소모하고 호위 1개를 추가"
    );
    assert_eq!(
        body.object.objs[0].lock().unwrap().getString("종류"),
        "호위"
    );
    assert_eq!(body.object.objs[0].lock().unwrap().getInt("체력"), 1000);

    body.set("성격", "정파");
    let faction = storage
        .execute("구입", &mut body, "사강시", None, None, None)
        .unwrap();
    assert_eq!(faction.0, vec!["☞ 해당 호위는 사파원만 사용 가능합니다."]);

    let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player_name);
    world.mob_cache.remove_mob(&mob_key);
}

#[test]
fn receive_command_runs_python_guard_funds_daily_limit_and_success_state() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let player_name = format!("수령회귀-{}", std::process::id());
    let zone = format!("수령회귀존-{}", std::process::id());
    let room = "1";
    let mob_key = format!("{zone}:표두");
    let mut guard_data = RawMobData::new();
    guard_data.name = "표두".to_string();
    guard_data.zone = zone.clone();
    guard_data.gold = 50_000;
    guard_data.reaction_names = vec!["표두".to_string(), "무사".to_string()];
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), guard_data.clone());
        let mut dead_guard = MobInstance::new(mob_key.clone(), zone.clone(), room, &guard_data);
        dead_guard.kill();
        world.mob_cache.add_mob_instance(dead_guard);
        world.set_player_position(
            &player_name,
            PlayerPosition::new(zone.clone(), room.to_string()),
        );
    }
    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("레벨", 100_i64);
    body.set("은전", 10_i64);
    body.set("수령액", 0_i64);
    body.set("마지막수령", 0_i64);
    let storage = ScriptStorage::default();

    let dead_missing = storage
        .execute("수령", &mut body, "0", None, None, None)
        .unwrap();
    assert_eq!(dead_missing.0, vec!["☞ 이곳에 표국무사가 없네요."]);

    let item_index = format!("수령표두패-{}", std::process::id());
    let item_path = std::path::Path::new("data/item").join(format!("{item_index}.json"));
    std::fs::write(
        &item_path,
        serde_json::json!({"아이템정보": {
            "인덱스": item_index, "이름": "수령표두패",
            "반응이름": ["표두패"], "은전": 2000
        }})
        .to_string(),
    )
    .unwrap();
    let mut sign = Object::new();
    sign.set("인덱스", item_index.as_str());
    sign.set("이름", "수령표두패");
    sign.set("반응이름", "표두패");
    sign.set("은전", 2_000_i64);
    let sign = Arc::new(Mutex::new(sign));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, room).push(sign.clone());
        world.record_floor_item(&zone, room, &sign);
    }
    let item_receive = storage
        .execute("수령", &mut body, "1000", None, None, None)
        .unwrap();
    assert!(item_receive.0[0].contains("은전 1000개를 표국무사에게 수령합니다."));
    assert_eq!(body.get_int("은전"), 1010);
    assert_eq!(body.get_int("수령액"), 1000);
    assert_eq!(sign.lock().unwrap().getInt("은전"), 1000);
    let saved_sign: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&item_path).unwrap()).unwrap();
    assert_eq!(saved_sign["아이템정보"]["은전"], 1000);
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, room, &sign);
        world
            .get_room_objs_mut(&zone, room)
            .retain(|item| !Arc::ptr_eq(item, &sign));
    }
    body.set("은전", 10_i64);
    body.set("수령액", 0_i64);
    body.set("마지막수령", 0_i64);
    get_world_state()
        .write()
        .unwrap()
        .mob_cache
        .add_mob_instance(MobInstance::new(
            mob_key.clone(),
            zone.clone(),
            room,
            &guard_data,
        ));

    let whitespace = storage
        .execute("수령", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법: [금액] 수령"]);
    let invalid = storage
        .execute("수령", &mut body, "0", None, None, None)
        .unwrap();
    assert_eq!(invalid.0, vec!["☞ 은전 1개 이상 입력 하셔야 해요."]);
    body.set("레벨", 501_i64);
    let high_level = storage
        .execute("수령", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(high_level.0, vec!["☞ 충분한 능력이 있어 보이는데요???"]);
    body.set("레벨", 100_i64);
    let greedy = storage
        .execute("수령", &mut body, "10000001", None, None, None)
        .unwrap();
    assert_eq!(greedy.0, vec!["☞ 너무 욕심이 크군요???"]);
    let short = storage
        .execute("수령", &mut body, "50001", None, None, None)
        .unwrap();
    assert_eq!(short.0, vec!["☞ 기부금이 모잘라요^^;"]);
    body.set("수령액", 1_000_000_000_i64);
    let total_limit = storage
        .execute("수령", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(total_limit.0, vec!["☞ 더이상 수령은 곤란해요^^;"]);
    body.set("수령액", 999_999_999_i64);
    let over_limit = storage
        .execute("수령", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(over_limit.0, vec!["☞ 한도 초과에요!!!"]);
    body.set("수령액", 0_i64);
    body.set("마지막수령", chrono::Utc::now().timestamp());
    let too_soon = storage
        .execute("수령", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(too_soon.0, vec!["☞ 또 오셨어요???"]);
    assert_eq!(body.get_int("은전"), 10);
    assert_eq!(body.get_int("수령액"), 0);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, room)
            .into_iter()
            .find(|mob| mob.alive)
            .unwrap()
            .gold,
        50_000
    );
    body.set("마지막수령", 0_i64);

    let success = storage
        .execute("수령", &mut body, "1000개", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("은전"), 1010);
    assert_eq!(body.get_int("수령액"), 1000);
    assert!(body.get_int("마지막수령") > 0);
    assert!(success
        .0
        .join("\r\n")
        .contains("은전 1000개를 표국무사에게 수령합니다."));
    let guard_gold = get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room(&zone, room)
        .into_iter()
        .find(|mob| mob.alive)
        .unwrap()
        .gold;
    assert_eq!(guard_gold, 49_000);

    let repeated = storage
        .execute("수령", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(repeated.0.join("\r\n"), "☞ 또 오셨어요???");

    let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
    let _ = std::fs::remove_file(item_path);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&player_name);
    get_world_state()
        .write()
        .unwrap()
        .mob_cache
        .remove_mob(&mob_key);
}

#[test]
fn receive_from_self_named_guard_preserves_python_aliased_silver_updates() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("표두자기수령-{suffix}");
    let zone = format!("표두자기수령존-{suffix}");
    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("반응이름", "표두");
    body.set("레벨", 100_i64);
    body.set("은전", 500_i64);
    body.set("수령액", 0_i64);
    body.set("마지막수령", 0_i64);
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&name, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let output = ScriptStorage::default()
        .execute("수령", &mut body, "200", None, None, None)
        .unwrap();
    assert_eq!(
        output.0,
        vec![
            "당신이 은전 200개를 표국무사에게 수령합니다.\r\n현재까지 수령한 기부금 총액은 은전 \x1b[1m200\x1b[0;37m개 입니다."
        ]
    );
    assert_eq!(body.get_int("은전"), 500);
    assert_eq!(body.get_int("수령액"), 200);
    assert!(body.get_int("마지막수령") > 0);
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&name);
}

#[test]
fn insurance_query_and_deposit_match_python_agent_order_prefix_and_ansi() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player = format!("보험회귀-{suffix}");
    let zone = format!("보험회귀존-{suffix}");
    let key = format!("{zone}:표두");
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.set("레벨", 2_i64);
    body.set("은전", 30_i64);
    body.set("보험료", 5_i64);
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&player, PlayerPosition::new(zone.clone(), "1".to_string()));
    let storage = ScriptStorage::default();

    let no_agent = storage
        .execute("입금", &mut body, "0", None, None, None)
        .unwrap();
    assert_eq!(no_agent.0, vec!["☞ 이곳에 표국무사가 없네요."]);
    let no_query = storage
        .execute("조회", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(no_query.0, vec!["☞ 이곳에 표국무사가 없네요."]);

    // 조회/입금도 Python에서는 is_mob 검사를 하지 않는다. 표두에
    // 반응하는 일반 방 아이템만 있어도 보험 창구가 존재하는 것으로
    // 처리된다.
    let mut sign = Object::new();
    sign.set("이름", "표두안내패");
    sign.set("반응이름", "표두안내패\r\n표두대리");
    let sign = Arc::new(Mutex::new(sign));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(sign.clone());
        world.record_floor_item(&zone, "1", &sign);
    }
    let item_agent = storage
        .execute("조회", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(item_agent.0.len(), 1);
    assert!(item_agent.0[0].starts_with("당신의 보험료 총액은"));
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, "1", &sign);
        world
            .get_room_objs_mut(&zone, "1")
            .retain(|item| !Arc::ptr_eq(item, &sign));
    }

    let mut data = RawMobData::new();
    data.name = "보험담당자".to_string();
    data.zone = zone.clone();
    data.reaction_names = vec!["표두대리인".to_string(), "표국무사".to_string()];
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        world
            .mob_cache
            .add_mob_instance(MobInstance::new(key.clone(), zone.clone(), "1", &data));
    }
    {
        let mut world = get_world_state().write().unwrap();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room_mut(&zone, "1")
            .unwrap()
            .first_mut()
            .unwrap();
        mob.alive = false;
        mob.act = 2;
    }
    let dead_agent = storage
        .execute("조회", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(dead_agent.0, vec!["☞ 이곳에 표국무사가 없네요."]);
    {
        let mut world = get_world_state().write().unwrap();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room_mut(&zone, "1")
            .unwrap()
            .first_mut()
            .unwrap();
        mob.alive = true;
        mob.act = 0;
    }
    let unit = get_murim_config_int("보험료단가");
    let threshold = 2 * unit;
    let trip = threshold * get_murim_config_int("보험출장률") / 100;
    let queried = storage
        .execute("조회", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        queried.0,
        vec![format!(
            "당신의 보험료 총액은 은전 \x1b[1m5\x1b[0;37m개이며\r\n보험 혜택은 \x1b[1m{}\x1b[0m\x1b[40m\x1b[37m번 받으실 수 있습니다.\r\n보험혜택이 적용되는 금액은 은전 \x1b[1m{threshold}\x1b[0;37m개 이상이며\r\n한번의 출장 처리시엔 은전 \x1b[1m{trip}\x1b[0;37m개가 소요됩니다.",
            if threshold > 0 { 5 / threshold } else { 0 }
        )]
    );

    let invalid = storage
        .execute("입금", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(invalid.0, vec!["☞ 사용법: [금액] 입금"]);
    let prefixed = storage
        .execute("입금", &mut body, " 10개 ", None, None, None)
        .unwrap();
    let premium = 15;
    assert_eq!(body.get_int("은전"), 20);
    assert_eq!(body.get_int("보험료"), premium);
    assert_eq!(
        prefixed.0,
        vec![format!(
            "당신이 은전 10개를 표국무사에게 입금합니다.\r\n\r\n당신의 보험료 총액은 은전 \x1b[1m{premium}\x1b[0;37m개이며\r\n보험 혜택은 \x1b[1m{}\x1b[0m\x1b[40m\x1b[37m번 받으실 수 있습니다.",
            if threshold > 0 {
                premium / threshold
            } else {
                0
            }
        )]
    );

    let underscored = storage
        .execute("입금", &mut body, "1_0", None, None, None)
        .unwrap();
    assert!(underscored.0[0].starts_with("당신이 은전 10개를"));
    assert_eq!(body.get_int("은전"), 10);
    assert_eq!(body.get_int("보험료"), 25);

    let clamped = storage
        .execute("입금", &mut body, "999", None, None, None)
        .unwrap();
    assert!(clamped.0[0].starts_with("당신이 은전 10개를"));
    assert_eq!(body.get_int("은전"), 0);
    assert_eq!(body.get_int("보험료"), 35);

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player);
    world.mob_cache.remove_instance(&zone, "1", &key);
}

#[test]
fn donation_command_requires_guard_then_clamps_to_carried_silver_like_python() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let player_name = format!("기부회귀-{}", std::process::id());
    let zone = format!("기부회귀존-{}", std::process::id());
    let mut body = Body::new();
    body.set("이름", player_name.as_str());
    body.set("은전", 50_i64);
    {
        get_world_state().write().unwrap().set_player_position(
            &player_name,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
    }
    let storage = ScriptStorage::default();
    let no_guard_before_amount = storage
        .execute("기부", &mut body, "0", None, None, None)
        .unwrap();
    assert_eq!(
        no_guard_before_amount.0,
        vec!["☞ 이곳에 표국무사가 없네요."]
    );

    body.set("반응이름", "표두");
    let self_donation = storage
        .execute("기부", &mut body, "10", None, None, None)
        .unwrap();
    assert_eq!(
        body.get_int("은전"),
        50,
        "same Python object subtracts then adds"
    );
    assert_eq!(
        self_donation.0,
        vec![
            "당신이 은전 10개를 표국무사에게 기탁합니다.\r\n현재까지 모여진 기부금 총액은 은전 \x1b[1m50\x1b[0;37m개 입니다."
        ]
    );
    body.set("반응이름", "");

    let receiver_name = format!("기부표두사용자-{}", std::process::id());
    let mut receiver = Body::new();
    receiver.set("이름", receiver_name.as_str());
    receiver.set("반응이름", "표두");
    receiver.set("은전", 5_i64);
    get_world_state().write().unwrap().set_player_position(
        &receiver_name,
        PlayerPosition::new(zone.clone(), "1".to_string()),
    );
    let mut receiver_view = rhai::Map::new();
    receiver_view.insert("이름".into(), Dynamic::from(receiver_name.clone()));
    receiver_view.insert("반응이름".into(), Dynamic::from("표두"));
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![Dynamic::from(receiver_view)],
    )]));
    set_cast_room_players(vec![CastRoomPlayerRef::new(&mut receiver)]);
    let player_donation = storage
        .execute("기부", &mut body, "10", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("은전"), 40);
    assert_eq!(receiver.get_int("은전"), 15);
    assert_eq!(
        player_donation.0,
        vec![
            "당신이 은전 10개를 표국무사에게 기탁합니다.\r\n현재까지 모여진 기부금 총액은 은전 \x1b[1m15\x1b[0;37m개 입니다."
        ]
    );
    let player_receipt = storage
        .execute("수령", &mut body, "5", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("은전"), 45);
    assert_eq!(body.get_int("수령액"), 5);
    assert_eq!(receiver.get_int("은전"), 10);
    assert_eq!(
        player_receipt.0,
        vec![
            "당신이 은전 5개를 표국무사에게 수령합니다.\r\n현재까지 수령한 기부금 총액은 은전 \x1b[1m5\x1b[0;37m개 입니다."
        ]
    );
    clear_cast_room_players();
    set_precomputed_room_view_players(HashMap::new());
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&receiver_name);
    body.set("은전", 50_i64);
    body.set("수령액", 0_i64);
    body.set("마지막수령", 0_i64);

    let item_index = format!("기부표두패-{}", std::process::id());
    let item_path = std::path::Path::new("data/item").join(format!("{item_index}.json"));
    std::fs::write(
        &item_path,
        serde_json::json!({"아이템정보": {
            "인덱스": item_index, "이름": "기부표두패",
            "반응이름": ["표두패"], "은전": 7
        }})
        .to_string(),
    )
    .unwrap();
    let mut sign = Object::new();
    sign.set("인덱스", item_index.as_str());
    sign.set("이름", "기부표두패");
    sign.set("반응이름", "표두패");
    sign.set("은전", 7_i64);
    let sign = Arc::new(Mutex::new(sign));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(sign.clone());
        world.record_floor_item(&zone, "1", &sign);
    }
    let item_donation = storage
        .execute("기부", &mut body, "10", None, None, None)
        .unwrap();
    assert_eq!(
        item_donation.0,
        vec![
            "당신이 은전 10개를 표국무사에게 기탁합니다.\r\n현재까지 모여진 기부금 총액은 은전 \x1b[1m17\x1b[0;37m개 입니다."
        ]
    );
    assert_eq!(body.get_int("은전"), 40);
    assert_eq!(sign.lock().unwrap().getInt("은전"), 17);
    let saved_sign: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&item_path).unwrap()).unwrap();
    assert_eq!(saved_sign["아이템정보"]["은전"], 17);
    {
        let mut world = get_world_state().write().unwrap();
        world.remove_floor_item_record(&zone, "1", &sign);
        world
            .get_room_objs_mut(&zone, "1")
            .retain(|item| !Arc::ptr_eq(item, &sign));
    }
    body.set("은전", 50_i64);

    let mob_key = format!("{zone}:표두");
    let mob_dir = std::path::Path::new("data/mob").join(&zone);
    let mob_path = mob_dir.join("표두.json");
    std::fs::create_dir_all(&mob_dir).unwrap();
    std::fs::write(
        &mob_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "몹정보": {"이름": "표두", "은전": 100}
        }))
        .unwrap(),
    )
    .unwrap();
    let mut guard_data = RawMobData::new();
    guard_data.name = "표두".into();
    guard_data.zone = zone.clone();
    guard_data.gold = 100;
    guard_data.reaction_names = vec!["표두".into(), "표국무사".into()];
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), guard_data.clone());
        let mut dead_guard = MobInstance::new(mob_key.clone(), zone.clone(), "1", &guard_data);
        dead_guard.kill();
        world.mob_cache.add_mob_instance(dead_guard);
    }
    let dead_is_missing = storage
        .execute("기부", &mut body, "1", None, None, None)
        .unwrap();
    assert_eq!(dead_is_missing.0, vec!["☞ 이곳에 표국무사가 없네요."]);
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.add_mob_instance(MobInstance::new(
            mob_key.clone(),
            zone.clone(),
            "1",
            &guard_data,
        ));
    }
    let whitespace = storage
        .execute("기부", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 사용법: [금액] 기부"]);
    let invalid = storage
        .execute("기부", &mut body, "0", None, None, None)
        .unwrap();
    assert_eq!(invalid.0, vec!["☞ 은전 1개 이상 입금 하셔야 해요."]);

    let numeric_prefix = storage
        .execute("기부", &mut body, "10개", None, None, None)
        .unwrap();
    assert_eq!(
        numeric_prefix.0,
        vec![
            "당신이 은전 10개를 표국무사에게 기탁합니다.\r\n현재까지 모여진 기부금 총액은 은전 \x1b[1m110\x1b[0;37m개 입니다."
        ]
    );
    assert_eq!(body.get_int("은전"), 40);

    let donated = storage
        .execute("기부", &mut body, "100", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("은전"), 0);
    assert_eq!(
        donated.0,
        vec![
            "당신이 은전 40개를 표국무사에게 기탁합니다.\r\n현재까지 모여진 기부금 총액은 은전 \x1b[1m150\x1b[0;37m개 입니다."
        ]
    );
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, "1")
            .into_iter()
            .find(|mob| mob.alive)
            .unwrap()
            .gold,
        150
    );
    let saved_guard: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mob_path).unwrap()).unwrap();
    assert_eq!(saved_guard["몹정보"]["은전"], 150);

    let zero_clamped = storage
        .execute("기부", &mut body, "1", None, None, None)
        .unwrap();
    assert!(zero_clamped.0[0].starts_with("당신이 은전 0개를"));

    let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
    let _ = std::fs::remove_file(item_path);
    let _ = std::fs::remove_dir_all(mob_dir);
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&player_name);
    world.mob_cache.remove_mob(&mob_key);
}
