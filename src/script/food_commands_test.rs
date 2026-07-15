use super::*;
#[test]
fn medicine_crafting_reads_real_mob_recipe_checks_output_before_consuming_and_formats_python() {
    use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

    let suffix = std::process::id();
    let player = format!("조제검사-{suffix}");
    let zone = format!("조제존-{suffix}");
    let room = "1";
    let mob_key = format!("{zone}:의원");
    let impostor_key = format!("{zone}:가짜조제사");
    let material_key = format!("조제재료-{suffix}");
    let result_key = format!("조제결과-{suffix}");
    let result_path = std::path::Path::new("data/item").join(format!("{result_key}.json"));
    std::fs::write(
        &result_path,
        serde_json::json!({"아이템정보":{"이름":"검사환단","종류":"먹는것","반응이름":["환단"]}})
            .to_string(),
    )
    .unwrap();
    let mut doctor = RawMobData::new();
    doctor.name = "의원장".into();
    doctor.reaction_names = vec!["의원".into()];
    doctor.attributes.insert(
        "조제 검사신약".into(),
        serde_json::json!([format!("{material_key} 2"), format!("+{result_key} 1")]),
    );
    doctor.attributes.insert(
        "조제 실패신약".into(),
        serde_json::json!([format!("{material_key} 2"), "+존재하지않는조제결과 1"]),
    );
    doctor.attributes.insert(
        "조제 중복재료신약".into(),
        serde_json::json!([
            format!("{material_key} 2"),
            format!("{material_key} 2"),
            format!("+{result_key} 1")
        ]),
    );
    let mut impostor = RawMobData::new();
    impostor.name = "약장수".into();
    impostor.reaction_names = vec!["장수".into()];
    impostor.attributes.insert(
        "조제 가짜신약".into(),
        serde_json::json!([format!("{material_key} 1"), format!("+{result_key} 1")]),
    );
    {
        let mut world = get_world_state().write().unwrap();
        world
            .mob_cache
            .insert_mob_data(mob_key.clone(), doctor.clone());
        world.mob_cache.add_mob_instance(MobInstance::new(
            mob_key.clone(),
            zone.clone(),
            room,
            &doctor,
        ));
        world
            .mob_cache
            .insert_mob_data(impostor_key.clone(), impostor.clone());
        world.mob_cache.add_mob_instance(MobInstance::new(
            impostor_key.clone(),
            zone.clone(),
            room,
            &impostor,
        ));
        world.set_player_position(&player, PlayerPosition::new(zone.clone(), room.into()));
    }
    let make_material = || {
        let mut item = Object::new();
        item.set("인덱스", material_key.as_str());
        item.set("이름", "검사약재");
        Arc::new(Mutex::new(item))
    };
    let mut body = Body::new();
    body.set("이름", player.as_str());
    body.object.objs.push(make_material());
    let storage = ScriptStorage::default();

    let non_doctor_recipe = storage
        .execute("조제", &mut body, "가짜신약", None, None, None)
        .unwrap();
    assert_eq!(
        non_doctor_recipe.0,
        vec!["☞ 그러한 것을 조제할 의원이 없어요. ^^"]
    );
    assert_eq!(body.object.objs.len(), 1);

    let insufficient = storage
        .execute("조제", &mut body, "검사신약", None, None, None)
        .unwrap();
    assert_eq!(
        insufficient.0,
        vec!["\x1b[33m의원장\x1b[37m이 말합니다. \"음.. 부족한게 있다네... 재료를 더 구해오게나\""]
    );
    assert_eq!(body.object.objs.len(), 1);

    body.object.objs.push(make_material());
    let repeated_material = storage
        .execute("조제", &mut body, "중복재료신약", None, None, None)
        .unwrap();
    assert_eq!(
        repeated_material.0,
        vec!["\x1b[33m의원장\x1b[37m이 말합니다. \"음.. 부족한게 있다네... 재료를 더 구해오게나\""]
    );
    assert_eq!(
        body.object.objs.len(),
        2,
        "Python consumes temporary index matches across repeated recipe lines"
    );
    let unavailable = storage
        .execute("조제", &mut body, "실패신약", None, None, None)
        .unwrap();
    assert_eq!(
        unavailable.0,
        vec![
            "\x1b[33m의원장\x1b[37m이 말합니다. \"음.. 재료가 다 떨어져서 한동안 조제가 힘들겠어...\""
        ]
    );
    assert_eq!(
        body.object.objs.len(),
        2,
        "Python creates results before consuming materials"
    );

    let mut keepsake = Object::new();
    keepsake.set("인덱스", "조제비재료");
    keepsake.set("이름", "기념패");
    body.object.objs.push(Arc::new(Mutex::new(keepsake)));

    let crafted = storage
        .execute("조제", &mut body, "검사신약", None, None, None)
        .unwrap();
    assert_eq!(
        crafted.0,
        vec![
            "당신이 \x1b[33m의원장\x1b[37m에게 \x1b[36m검사신약\x1b[37m을 만들수 있는 재료들을 건네줍니다.",
            "\x1b[33m의원장\x1b[37m이 재료들을 가지고 심오한 기를 불어 넣으며 작업합니다.",
            "\x1b[33m의원장\x1b[37m이 당신에게 \x1b[0;36m검사환단\x1b[37m을 1개 줍니다.",
        ]
    );
    assert_eq!(body.object.objs.len(), 1);
    assert_eq!(body.object.inv_stack.get(&result_key), Some(&1));
    assert_eq!(
        body.object.objs[0].lock().unwrap().getString("인덱스"),
        "조제비재료"
    );

    let mut world = get_world_state().write().unwrap();
    world.mob_cache.remove_instance(&zone, room, &mob_key);
    world.mob_cache.remove_mob_definition(&mob_key);
    world.mob_cache.remove_instance(&zone, room, &impostor_key);
    world.mob_cache.remove_mob_definition(&impostor_key);
    world.remove_player_position(&player);
    drop(world);
    let _ = std::fs::remove_file(result_path);
}
#[test]
fn eating_requeues_any_healing_food_when_python_continuous_aliases_are_enabled() {
    use crate::command::handler::CommandResult;

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "연속복용검사");
    body.set("체력", 100_i64);
    body.set("최고체력", 10_000_i64);
    body.set(
        ALIAS_LIST_ATTR,
        encode_alias_entries(&[
            ("체력약".into(), "다른약".into()),
            ("체력".into(), "9000이하".into()),
            ("연속복용".into(), "켜기".into()),
        ]),
    );
    let (food, _) = object_from_item_json("1037").expect("food fixture");
    body.object.objs.push(food);
    let (output, special) = storage
        .execute("먹어", &mut body, "탕수육", None, None, None)
        .unwrap();
    assert_eq!(body.get_hp(), 7030);
    assert_eq!(output.len(), 1);
    assert!(matches!(
        special,
        Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
            if own == &output[0]
                && sends == &vec![("연속복용검사".to_string(), "탕수육 먹어".to_string())]
    ));
}
#[test]
fn eating_poison_preserves_python_negative_vitals_without_lower_clamp() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "독음식검사");
    body.set("체력", 5_i64);
    body.set("내공", 3_i64);
    body.set("최고체력", 100_i64);
    body.set("최고내공", 100_i64);
    let mut poison = Object::new();
    poison.set("이름", "독버섯");
    poison.set("반응이름", "독버섯");
    poison.set("종류", "먹는것");
    poison.set("체력", -10_i64);
    poison.set("내공", -8_i64);
    poison.set("사용스크립", "$아이템$을 먹고 고통스러워합니다.");
    body.object.objs.push(Arc::new(Mutex::new(poison)));

    let eaten = storage
        .execute("먹어", &mut body, "독버섯", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("체력"), -5);
    assert_eq!(body.get_int("내공"), -5);
    assert!(body.object.objs.is_empty());
    assert_eq!(
        eaten.0,
        vec!["당신이 \x1b[0;36m독버섯\x1b[37m을 먹고 고통스러워합니다."]
    );
}
#[test]
fn eating_stacked_max_mp_food_matches_python_item_effect_and_one_time_limit() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "스택백사주복용자");
    body.set("체력", 100_i64);
    body.set("최고체력", 500_i64);
    body.set("내공", 50_i64);
    body.set("최고내공", 100_i64);
    body.object.inv_stack.insert("백사주".to_string(), 2);

    let first = storage
        .execute("먹어", &mut body, "백사주", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("최고내공"), 130);
    assert_eq!(body.object.inv_stack.get("백사주"), Some(&1));
    assert!(body.object.objs.is_empty());
    assert_eq!(first.0.len(), 2);
    assert!(first.0[0].contains("마십니다.\r\n뜨거운 기운"));
    assert!(first.0[1].contains("+30"));

    let second = storage
        .execute("먹어", &mut body, "백사주", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("최고내공"), 130);
    assert!(body.object.inv_stack.is_empty());
    assert!(body.object.objs.is_empty());
    assert_eq!(second.0.len(), 1);
    assert!(second.0[0].contains("마십니다.\r\n뜨거운 기운"));
    let _ = std::fs::remove_file("data/user/스택백사주복용자.json");
}

#[test]
fn numbered_eating_counts_stateful_food_before_pristine_quantity() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "혼합백사주복용자");
    body.set("체력", 10_i64);
    body.set("최고체력", 500_i64);
    body.set("내공", 10_i64);
    body.set("최고내공", 100_i64);
    let (modified, _) = object_from_item_json("백사주").unwrap();
    modified.lock().unwrap().set("체력", 7_i64);
    body.object.objs.push(modified.clone());
    body.object.inv_stack.insert("백사주".into(), 1);

    storage
        .execute("먹어", &mut body, "2백사주", None, None, None)
        .unwrap();
    assert!(!body.object.inv_stack.contains_key("백사주"));
    assert_eq!(body.object.objs.len(), 1);
    assert!(Arc::ptr_eq(&body.object.objs[0], &modified));
    assert_eq!(body.get_int("체력"), 10);

    storage
        .execute("먹어", &mut body, "백사주", None, None, None)
        .unwrap();
    assert!(body.object.objs.is_empty());
    assert_eq!(body.get_int("체력"), 17);
    let _ = std::fs::remove_file("data/user/혼합백사주복용자.json");
}
#[test]
fn eating_list_script_preserves_python_crlf_and_max_mp_room_ansi() {
    use crate::command::handler::CommandResult;

    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "백사주복용자");
    body.set("체력", 100_i64);
    body.set("최고체력", 500_i64);
    body.set("내공", 50_i64);
    body.set("최고내공", 100_i64);
    let (drink, _) = object_from_item_json("백사주").expect("list-script drink fixture");
    assert_eq!(
        drink.lock().unwrap().getString("사용스크립"),
        "$아이템$을 마십니다.\n뜨거운 기운이 기경팔맥으로 뻗어나갑니다"
    );
    body.object.objs.push(drink);
    let mut observer = Body::new();
    observer.set("이름", "복용목격자");
    observer.set("설정상태", "타인전투출력거부 0");
    observer.set("체력", 21_i64);
    observer.set("최고체력", 31_i64);
    observer.set("내공", 4_i64);
    observer.set("최고내공", 5_i64);
    let mut rejecting = Body::new();
    rejecting.set("이름", "복용출력거부자");
    rejecting.set("설정상태", "타인전투출력거부 1");
    set_cast_room_players(vec![
        CastRoomPlayerRef::new(&mut observer),
        CastRoomPlayerRef::new(&mut rejecting),
    ]);

    let (output, special) = storage
        .execute("먹어", &mut body, "백사주", None, None, None)
        .unwrap();
    clear_cast_room_players();
    assert_eq!(body.get_int("최고내공"), 130);
    assert!(body.object.objs.is_empty());
    assert_eq!(
        output,
        vec![
            "당신이 \x1b[0;36m백사주\x1b[37m을 마십니다.\r\n뜨거운 기운이 기경팔맥으로 뻗어나갑니다",
            "\r\n\x1b[1m당신의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돕니다.\x1b[0;37m (\x1b[1;36m+30\x1b[0;37m)",
        ]
    );
    let sends = match special.unwrap() {
        CommandResult::OutputAndSendToUsers(_, sends) => sends,
        other => panic!("unexpected eating room delivery: {other:?}"),
    };
    assert_eq!(sends.len(), 1);
    assert_eq!(sends[0].0, "복용목격자");
    assert_eq!(
        sends[0].1,
        format!(
            "{}\r\n\x1b[1m백사주복용자\x1b[0;37m가 \x1b[0;36m백사주\x1b[37m을 마십니다.\r\n뜨거운 기운이 기경팔맥으로 뻗어나갑니다\r\n\r\n\x1b[1m\x1b[1m백사주복용자\x1b[0;37m의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돕니다.\x1b[0;37m (\x1b[1;36m+30\x1b[0;37m)\r\n\r\n\x1b[0;37;40m[ 21/31, 4/5 ] ",
            RAW_USER_MESSAGE_PREFIX
        )
    );
    let _ = std::fs::remove_file("data/user/백사주복용자.json");
}
#[test]
fn eating_uses_item_script_and_clamps_vitals_before_removal() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "복용검사");
    body.set("체력", 100_i64);
    body.set("최고체력", 500_i64);
    let (food, _) = object_from_item_json("1037").expect("food fixture");
    body.object.objs.push(food);
    let (output, _) = storage
        .execute("먹어", &mut body, "탕수육", None, None, None)
        .unwrap();
    assert_eq!(body.get_hp(), 500);
    assert!(body.object.objs.is_empty());
    assert_eq!(
        output,
        vec!["당신이 \x1b[0m\x1b[36m\x1b[40m탕수육\x1b[0m\x1b[37m\x1b[40m을 맛있게 먹습니다."]
    );
}
