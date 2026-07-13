use super::*;

#[test]
fn engraved_teleport_keeps_python_validation_order_and_messages() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();

    let usage = storage
        .execute("이형환위", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 사용법: [대상] 위치이동"]);
    let wrong_target = storage
        .execute("이형환위", &mut body, "비학천룡아님", None, None, None)
        .unwrap();
    assert_eq!(wrong_target.0, vec!["☞ 어디로 이동하시려구요?"]);
    let no_engraving = storage
        .execute("이형환위", &mut body, "비학천룡", None, None, None)
        .unwrap();
    assert_eq!(no_engraving.0, vec!["☞ 각인된 위치가 없습니다."]);

    body.set("위치각인", "콜론없는잘못된방");
    let invalid_room = storage
        .execute("이형환위", &mut body, "비학천룡", None, None, None)
        .unwrap();
    assert_eq!(invalid_room.0, vec!["* 이동 실패!!!"]);
}

#[test]
fn guard_qi_command_distinguishes_python_no_guard_branch() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    let (output, special) = storage
        .execute("내공주입", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 호위를 거느리지 않고 있습니다."]);
    assert!(special.is_none());
}

#[test]
fn guard_list_resolves_room_alias_and_explicit_self_like_python() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let actor = format!("호위조회자-{suffix}");
    let target = format!("호위대상-{suffix}");
    let zone = format!("호위조회존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&actor, PlayerPosition::new(zone.clone(), "1".to_string()));
        world.set_player_position(&target, PlayerPosition::new(zone.clone(), "1".to_string()));
    }
    let snapshot = |name: &str, reactions: &str| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name.to_string()));
        map.insert("반응이름".into(), Dynamic::from(reactions.to_string()));
        Dynamic::from(map)
    };
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![
            snapshot(&actor, "나별칭"),
            snapshot(&target, "대상별칭\r\n긴별칭"),
        ],
    )]));

    let mut body = Body::new();
    body.set("이름", actor.as_str());
    body.temp_mut().insert(
        "_online_room_admin".to_string(),
        Value::String(serde_json::json!([{"name": target, "anger": 0, "guards": []}]).to_string()),
    );
    let storage = ScriptStorage::default();
    let self_named = storage
        .execute("호위", &mut body, &actor, None, None, None)
        .unwrap();
    assert_eq!(self_named.0, vec!["당신은 호위를 거느리지 않고 있습니다."]);
    let alias = storage
        .execute("호위", &mut body, "대상별칭", None, None, None)
        .unwrap();
    assert_eq!(
        alias.0,
        vec![format!(
            "\x1b[1m{target}\x1b[0;37m{} 호위를 거느리지 않고 있습니다.",
            han_eun(&target)
        )]
    );
    let partial = storage
        .execute("호위", &mut body, "대상", None, None, None)
        .unwrap();
    assert_eq!(partial.0, alias.0);

    let mut collision = Object::new();
    collision.set("이름", "호위조회충돌물건");
    collision.set("반응이름", "대상별칭");
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let item_first = storage
        .execute("호위", &mut body, "대상별칭", None, None, None)
        .unwrap();
    assert_eq!(
        item_first.0,
        vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]
    );
    get_world_state().write().unwrap().record_test_room_object(
        &zone,
        "1",
        crate::world::RoomObjectRef::Player(target.clone()),
    );
    let player_first = storage
        .execute("호위", &mut body, "대상별칭", None, None, None)
        .unwrap();
    assert_eq!(player_first.0, alias.0);

    set_precomputed_room_view_players(HashMap::new());
    let mut world = get_world_state().write().unwrap();
    world.get_room_objs_mut(&zone, "1").clear();
    world.remove_player_position(&actor);
    world.remove_player_position(&target);
}

#[test]
fn admin_recovery_uses_python_derived_maximum_vitals() {
    let mut body = Body::new();
    body.set("관리자등급", 1000_i64);
    body.set("최고체력", 1234_i64);
    body.set("최고내공", 567_i64);
    body.set("체력", 10_i64);
    body.set("내공", 20_i64);
    let expected_hp = body.get_max_hp();
    let expected_mp = body.get_max_mp();

    let result = ScriptStorage::default()
        .execute("회복", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["* 회복되었습니다."]);
    assert_eq!(body.get_hp(), expected_hp);
    assert_eq!(body.get_mp(), expected_mp);
}

#[test]
fn guard_qi_command_heals_template_hp_and_reports_total_spend() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("힘", 100_i64);
    body.set("내공", 500_i64);
    let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
    {
        let mut guard = guard.lock().unwrap();
        guard.set("체력", 1000_i64);
    }
    body.object.objs.push(guard);

    let (output, _) = storage
        .execute("내공주입", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("내공"), 490);
    assert_eq!(body.object.objs[0].lock().unwrap().getInt("체력"), 1224);
    assert!(output[0].contains("사강시에게 내가진기를 주입하여 체력을 회복 시킵니다."));
    assert!(output[0].contains("+224"));
    assert_eq!(
        output[1],
        "당신이 소모된 진기를 다스립니다. (\x1b[1;32m-10\x1b[0;37m)"
    );
}

#[test]
fn guard_qi_command_matches_python_full_shortage_and_partial_inventory_order() {
    let storage = ScriptStorage::default();
    let make_guard = |hp: i64| {
        let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
        guard.lock().unwrap().set("체력", hp);
        guard
    };

    let mut full = Body::new();
    full.set("힘", 100_i64);
    full.set("내공", 100_i64);
    full.object.objs.push(make_guard(2240));
    let full_result = storage
        .execute("내공주입", &mut full, "무시", None, None, None)
        .unwrap();
    assert_eq!(full_result.0, vec!["☞ 회복할 호위가 없습니다."]);
    assert_eq!(full.get_int("내공"), 100);

    let mut shortage = Body::new();
    shortage.set("힘", 100_i64);
    shortage.set("내공", 9_i64);
    shortage.object.objs.push(make_guard(1000));
    let shortage_result = storage
        .execute("내공주입", &mut shortage, "", None, None, None)
        .unwrap();
    assert_eq!(
        shortage_result.0,
        vec!["☞ 내가진기를 주입할 내공이 부족합니다."]
    );
    assert_eq!(shortage.get_int("내공"), 9);
    assert_eq!(shortage.object.objs[0].lock().unwrap().getInt("체력"), 1000);

    let mut partial = Body::new();
    partial.set("힘", 100_i64);
    partial.set("내공", 15_i64);
    partial.object.objs.push(make_guard(1000));
    partial.object.objs.push(make_guard(1200));
    let partial_result = storage
        .execute(
            "내공주입",
            &mut partial,
            "입력은 사용하지 않음",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(partial.get_int("내공"), 5);
    assert_eq!(partial.object.objs[0].lock().unwrap().getInt("체력"), 1224);
    assert_eq!(partial.object.objs[1].lock().unwrap().getInt("체력"), 1200);
    assert_eq!(partial_result.0.len(), 2);
    assert_eq!(
            partial_result.0[0],
            "당신이 사강시에게 내가진기를 주입하여 체력을 회복 시킵니다. (\x1b[1;36m+224\x1b[0;37m)\r\n"
        );
    assert_eq!(
        partial_result.0[1],
        "당신이 소모된 진기를 다스립니다. (\x1b[1;32m-10\x1b[0;37m)"
    );

    let mut varied = Body::new();
    varied.set("힘", 100_i64);
    varied.set("내공", 100_i64);
    let varied_first = make_guard(1000);
    varied_first.lock().unwrap().set("내공감소", 10_i64);
    let varied_second = make_guard(1000);
    varied_second.lock().unwrap().set("내공감소", 20_i64);
    varied.object.objs.push(varied_first);
    varied.object.objs.push(varied_second);
    let varied_result = storage
        .execute("내공주입", &mut varied, "", None, None, None)
        .unwrap();
    assert_eq!(varied.get_int("내공"), 70, "actual costs are 10 + 20");
    assert_eq!(
        varied_result.0[1], "당신이 소모된 진기를 다스립니다. (\x1b[1;32m-40\x1b[0;37m)",
        "Python displays the last loop cost multiplied by healed count"
    );

    let mut shortage_after_one = Body::new();
    shortage_after_one.set("힘", 100_i64);
    shortage_after_one.set("내공", 25_i64);
    let affordable = make_guard(1000);
    affordable.lock().unwrap().set("내공감소", 10_i64);
    let unaffordable = make_guard(1000);
    unaffordable.lock().unwrap().set("내공감소", 30_i64);
    shortage_after_one.object.objs.push(affordable);
    shortage_after_one.object.objs.push(unaffordable);
    let shortage_after_one_result = storage
        .execute("내공주입", &mut shortage_after_one, "", None, None, None)
        .unwrap();
    assert_eq!(shortage_after_one.get_int("내공"), 15);
    assert_eq!(
        shortage_after_one_result.0[1],
        "당신이 소모된 진기를 다스립니다. (\x1b[1;32m-30\x1b[0;37m)",
        "Python leaves mp set to the candidate that caused break"
    );

    let mut negative = Body::new();
    negative.set("힘", 100_i64);
    negative.set("내공", 20_i64);
    let odd_guard = make_guard(1000);
    odd_guard.lock().unwrap().set("내공감소", -10_i64);
    negative.object.objs.push(odd_guard);
    let odd = storage
        .execute("내공주입", &mut negative, "", None, None, None)
        .unwrap();
    assert_eq!(negative.get_int("내공"), 30);
    assert_eq!(
        odd.0[1],
        "당신이 소모된 진기를 다스립니다. (\x1b[1;32m--10\x1b[0;37m)"
    );
}

#[test]
fn guard_view_reads_the_same_inventory_objects_as_guard_combat() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("분노", 37_i64);
    let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
    guard.lock().unwrap().set("체력", 700_i64);
    body.object.objs.push(guard);

    let (output, _) = storage
        .execute("호위", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output, vec![concat!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n",
            "\x1b[1;44m☞ 당신의 호위 : 사강시, 호위수 : 1, 분노 : 37                        \x1b[0;40m\r\n",
            "────────────────────────────\r\n",
            "죽은지 사흘이내의 시체를 이용해서 만든 초급적인 강시\r\n",
            "────────────────────────────\r\n",
            "\x1b[1;32m·\x1b[0;36m 1.사강시\x1b[0;37m ː \x1b[33m━━━━━\x1b[37m━━━━━\x1b[37m (50)\r\n",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        )]);
}

#[test]
fn guard_view_preserves_list_description_crlf_and_uses_last_guard_like_python() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("분노", 88_i64);
    let (first, _) = object_from_item_json("사강시").expect("first guard fixture");
    let (last, _) = object_from_item_json("귀혼강시").expect("list description guard");
    first.lock().unwrap().set("체력", 700_i64);
    last.lock().unwrap().set("체력", 850_i64);
    assert_eq!(
            last.lock().unwrap().getString("설명2"),
            "인간의 심지를 말살시켜 하나의 가공할 살상병기로 만들어\n내는 극악무도한 사술을 이용해서 만들어내는 강시로써 자\n아기능이 상실되어 무차별적인 공격과 살생을 일삼는다"
        );
    body.object.objs.push(first);
    body.object.objs.push(last);

    let (output, _) = storage
        .execute("호위", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output.len(), 1);
    assert!(output[0].contains("☞ 당신의 호위 : 귀혼강시, 호위수 : 2, 분노 : 88"));
    assert!(output[0].contains(
            "인간의 심지를 말살시켜 하나의 가공할 살상병기로 만들어\r\n내는 극악무도한 사술을 이용해서 만들어내는 강시로써 자\r\n아기능이 상실되어 무차별적인 공격과 살생을 일삼는다\r\n"
        ));
    let first_line = output[0].find(" 1.사강시").unwrap();
    let second_line = output[0].find(" 2.귀혼강시").unwrap();
    assert!(first_line < second_line, "Python preserves inventory order");
}

#[test]
fn teleport_rejects_a_non_dragon_first_guard_like_python() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("위치각인", "낙양성:42");
    let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
    body.object.objs.push(guard);
    let (output, _) = storage
        .execute("이형환위", &mut body, "비학천룡", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 비학천룡이 없습니다."]);
    let (output, _) = storage
        .execute("위치각인", &mut body, "비학천룡", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 비학천룡이 없습니다."]);
}
