use super::*;
#[test]
fn admin_skill_transfer_allows_self_and_room_reaction_prefix_in_python_order() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin_name = format!("전수관리자-{suffix}");
    let target_name = format!("전수대상-{suffix}");
    let zone = format!("전수시험존-{suffix}");
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            &admin_name,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
        world.set_player_position(
            &target_name,
            PlayerPosition::new(zone.clone(), "1".to_string()),
        );
    }
    let player_view = |name: &str, reactions: &str| {
        let mut map = rhai::Map::new();
        map.insert("이름".into(), Dynamic::from(name.to_string()));
        map.insert("반응이름".into(), Dynamic::from(reactions.to_string()));
        Dynamic::from(map)
    };
    set_precomputed_room_view_players(HashMap::from([(
        format!("{zone}:1"),
        vec![
            player_view(&admin_name, "자기별칭"),
            player_view(&target_name, "제자별칭"),
        ],
    )]));
    let target_body = {
        let mut target = Body::new();
        target.set("이름", target_name.as_str());
        target
    };
    set_precomputed_room_mugong_targets(vec![build_room_mugong_player_snapshot(&target_body)]);

    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();
    let self_transfer = storage
        .execute("무공전수", &mut admin, "자기별 가의", None, None, None)
        .unwrap();
    assert_eq!(self_transfer.0, vec!["☞ 무공이 전수되었습니다."]);
    assert!(admin.skill_list.iter().any(|skill| skill == "가의신공"));

    let target_transfer = storage
        .execute("무공전수", &mut admin, "제자별 강뢰", None, None, None)
        .unwrap();
    assert_eq!(target_transfer.0, vec!["☞ 무공이 전수되었습니다."]);
    assert_eq!(
        take_teach_skill_request(&mut admin),
        Some((target_name.clone(), "강뢰검".to_string()))
    );

    for _ in 0..2 {
        let raw_transfer = storage
            .execute(
                "무공전수2",
                &mut admin,
                "  자기별   임의 무공 이름  ",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(raw_transfer.0, vec!["☞ 무공이 전수되었습니다."]);
    }
    assert_eq!(
        admin
            .skill_list
            .iter()
            .filter(|skill| *skill == "임의 무공 이름")
            .count(),
        2,
        "Python 무공전수2는 원시 skillList.append라 중복과 공백 포함 이름을 허용"
    );

    let mut active = crate::player::ActiveSkill::new("방어 시험".to_string(), 10);
    active.str_bonus = 3;
    admin._str = 3;
    admin.active_skills.push(active);
    admin.sync_active_skills_to_attrs();
    let removed = storage
        .execute(
            "무공제거",
            &mut admin,
            " 자기별 방어 시험 ",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(removed.0, vec!["☞ 무공이 제거되었습니다."]);
    assert!(admin.active_skills.is_empty());
    assert_eq!(
        admin._str, 0,
        "Rust는 Python의 잔류 보너스 오류도 함께 복구"
    );
    assert_eq!(admin.get_string("방어무공시전"), "");

    set_precomputed_room_view_players(HashMap::new());
    set_precomputed_room_mugong_targets(Vec::new());
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin_name);
    world.remove_player_position(&target_name);
    let _ = std::fs::remove_file(format!("data/user/{admin_name}.json"));
}
#[test]
fn test_auto_skill_commands_match_python_state_changes() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("자동무공"));
    assert!(storage.has_script("자동무공삭제"));
    let mut body = Body::new();
    body.set("이름", "자동무공검사");
    body.skill_list.push("강룡십팔장".to_string());
    body.skill_list.push("고영신공".to_string());

    let whitespace = storage
        .execute("자동무공", &mut body, "   ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, vec!["☞ 자동무공 : 없음"]);

    let (output, _) = storage
        .execute("자동무공", &mut body, "강룡", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 자동무공을 지정하였습니다."]);
    assert_eq!(body.get_string("자동무공"), "강룡십팔장");

    let missing = storage
        .execute("자동무공", &mut body, "없는무공", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 그런 무공을 습득한 적이 없습니다."]);
    assert_eq!(body.get_string("자동무공"), "강룡십팔장");

    let noncombat = storage
        .execute("자동무공", &mut body, "고영", None, None, None)
        .unwrap();
    assert_eq!(noncombat.0, vec!["☞ 자동무공을 할 수 없는 무공입니다."]);
    assert_eq!(body.get_string("자동무공"), "강룡십팔장");

    let shown = storage
        .execute("자동무공", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        shown.0,
        vec!["☞ 자동무공 : [\x1b[1;37m강룡십팔장\x1b[0;37m]"]
    );

    let (output, _) = storage
        .execute("자동무공삭제", &mut body, "무시되는 인자", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 자동무공을 삭제하였습니다."]);
    assert_eq!(body.get_string("자동무공"), "");

    let (output, _) = storage
        .execute("자동무공삭제", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 자동무공 : 없음"]);
}
#[test]
fn admin_skill_rank_preserves_unbounded_python_integer() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "성값검사");
    body.set("관리자등급", 2000_i64);
    let (output, _) = storage
        .execute("성올려", &mut body, "성값검사 태극권 999", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(
        body.skill_map["태극권"],
        crate::player::SkillTraining::new(999, 199_999)
    );
    let _ = storage
        .execute("성올려", &mut body, "성값검사 태극권 -7", None, None, None)
        .unwrap();
    assert_eq!(body.skill_map["태극권"].level, -7);
    let invalid = storage
        .execute(
            "성올려",
            &mut body,
            "성값검사 태극권 잘못",
            None,
            None,
            None,
        )
        .unwrap();
    assert!(invalid.0.is_empty());
    assert_eq!(body.skill_map["태극권"].level, -7);
    let extra = storage
        .execute(
            "성올려",
            &mut body,
            "성값검사 태극권 10 추가",
            None,
            None,
            None,
        )
        .unwrap();
    assert!(extra.0.is_empty());
    assert_eq!(body.skill_map["태극권"].level, -7);
    let signed = storage
        .execute("성올려", &mut body, "성값검사 태극권 +8", None, None, None)
        .unwrap();
    assert_eq!(signed.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(body.skill_map["태극권"].level, 8);

    let mut target = Body::new();
    target.set("이름", "성값대상");
    target.set("반응이름", "성대상별칭");
    set_precomputed_room_mugong_targets(vec![build_room_mugong_player_snapshot(&target)]);
    let targeted = storage
        .execute(
            "성올려",
            &mut body,
            "성대상별칭 태극권 15",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(targeted.0, vec!["☞ 값이 설정되었습니다."]);
    assert_eq!(
        take_set_skill_request(&mut body),
        Some(("성값대상".to_string(), "태극권".to_string(), 15))
    );

    let zone = format!("성올려자기충돌존-{}", std::process::id());
    let mut collision = Object::new();
    collision.set("이름", "성값검사");
    let collision = Arc::new(Mutex::new(collision));
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(
            "성값검사",
            crate::world::PlayerPosition::new(zone.clone(), "1".to_string()),
        );
        world.get_room_objs_mut(&zone, "1").push(collision.clone());
        world.record_floor_item(&zone, "1", &collision);
    }
    let self_item_first = storage
        .execute("성올려", &mut body, "성값검사 태극권 77", None, None, None)
        .unwrap();
    assert_eq!(self_item_first.0, vec!["☞ 그런 대상이 없어요!"]);
    assert_eq!(body.skill_map["태극권"].level, 8);
    let mob_key = format!("{zone}:성올려몹");
    let mut mob_data = RawMobData::new();
    mob_data.name = "성올려대상몹".to_string();
    mob_data.reaction_names = vec!["성몹별칭".to_string()];
    let mob = MobInstance::new(mob_key.clone(), zone.clone(), "1", &mob_data);
    let mob_id = mob.instance_id;
    {
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").clear();
        world.mob_cache.insert_mob_data(mob_key.clone(), mob_data);
        world.mob_cache.add_mob_instance(mob);
        world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(mob_id));
    }
    let mob_rank = storage
        .execute("성올려", &mut body, "성몹 태극권 23", None, None, None)
        .unwrap();
    assert_eq!(mob_rank.0, vec!["☞ 값이 설정되었습니다."]);
    let world = get_world_state().read().unwrap();
    let mob = world
        .mob_cache
        .get_all_mobs_in_room(&zone, "1")
        .into_iter()
        .find(|mob| mob.instance_id == mob_id)
        .unwrap();
    assert_eq!(
        mob.skill_map.get("태극권"),
        Some(&crate::player::SkillTraining::new(23, 199_999))
    );
    assert!(mob.learned_skills.contains(&"태극권".to_string()));
    drop(world);
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position("성값검사");
    world.mob_cache.remove_mob(&mob_key);
}

#[test]
fn admin_skill_rank_updates_socketless_summoned_player_like_python_player() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let admin_name = format!("성올려소환관리자-{suffix}");
    let target_name = format!("성올려소환대상-{suffix}");
    let zone = format!("성올려소환존-{suffix}");
    let mut target = Body::new();
    target.set("이름", target_name.as_str());
    target.set("반응이름", "소환성별칭");
    let id = {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&admin_name, PlayerPosition::new(zone.clone(), "1".into()));
        world.add_summoned_user(target, PlayerPosition::new(zone.clone(), "1".into()))
    };
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 2000_i64);
    let result = ScriptStorage::default()
        .execute("성올려", &mut admin, "소환성 태극권 -19", None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["☞ 값이 설정되었습니다."]);
    let world = get_world_state().read().unwrap();
    let training = world
        .summoned_users()
        .iter()
        .find(|user| user.id == id)
        .and_then(|user| user.body.skill_map.get("태극권"))
        .cloned();
    assert_eq!(
        training,
        Some(crate::player::SkillTraining::new(-19, 199_999))
    );
    drop(world);
    let mut world = get_world_state().write().unwrap();
    world.remove_summoned_user_by_id(id);
    world.remove_player_position(&admin_name);
}

#[test]
fn vision_and_mastery_commands_match_python_selection_state_and_exact_table() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "비전숙련회귀");
    body.set("비전이름", "비전검법|비전도법");
    for (weapon, value) in [(1, 7), (2, 80), (3, 900), (4, 10_000), (5, 0)] {
        body.set(&format!("{weapon} 숙련도"), value as i64);
    }

    let none = storage
        .execute("비전", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(none.0, vec!["☞ 비전 : 없음"]);
    let unknown = storage
        .execute("비전", &mut body, "없는비전", None, None, None)
        .unwrap();
    assert_eq!(unknown.0, vec!["☞ 당신은 그런 비전을 배운적이 없습니다."]);
    let prefix_only = storage
        .execute("비전", &mut body, "비전도", None, None, None)
        .unwrap();
    assert_eq!(
        prefix_only.0,
        vec!["☞ 당신은 그런 비전을 배운적이 없습니다."]
    );
    let selected = storage
        .execute("비전", &mut body, "  비전도법  ", None, None, None)
        .unwrap();
    assert_eq!(selected.0, vec!["☞ 비전을 지정하였습니다."]);
    assert_eq!(body.get_string("비전설정"), "비전도법");
    let current = storage
        .execute("비전", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(current.0, vec!["☞ 비전 : [\x1b[1;37m비전도법\x1b[0;37m]"]);
    let deleted = storage
        .execute("비전삭제", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(deleted.0, vec!["☞ 지정된 비전을 삭제합니다."]);
    assert_eq!(body.get_string("비전설정"), "");
    let deleted_again = storage
        .execute("비전삭제", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(deleted_again.0, vec!["☞ 지정된 비전이 없습니다."]);

    let mastery = storage
        .execute("숙련도", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(
        mastery.0,
        vec![
            "\x1b[1m ★ 당신의 무기 숙련도 ★\x1b[0m\x1b[40m\x1b[37m",
            "┏─────┬─────┓",
            "│◁  검  ▷│\x1b[1m         7\x1b[0m\x1b[40m\x1b[37m│",
            "├─────┼─────┤",
            "│◁  도  ▷│\x1b[1m        80\x1b[0m\x1b[40m\x1b[37m│",
            "├─────┼─────┤",
            "│◁  창  ▷│\x1b[1m       900\x1b[0m\x1b[40m\x1b[37m│",
            "├─────┼─────┤",
            "│◁ 기타 ▷│\x1b[1m     10000\x1b[0m\x1b[40m\x1b[37m│",
            "├─────┼─────┤",
            "│◁ 맨손 ▷│\x1b[1m         0\x1b[0m\x1b[40m\x1b[37m│",
            "┗─────┴─────┛",
        ]
    );
}

#[test]
fn mugong_status_non_admin_ignores_target_argument_and_shows_active_effect() {
    let mut body = Body::new();
    body.set("이름", "무공상태회귀");
    body.set("관리자등급", 0_i64);
    body.skill_map.insert(
        "고영신공".to_string(),
        crate::player::SkillTraining::new(1, 0),
    );
    body.active_skills
        .push(crate::player::ActiveSkill::new("고영신공".to_string(), 150));

    let shown = ScriptStorage::default()
        .execute(
            "무공상태",
            &mut body,
            "  존재하지않는대상  ",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(shown.0[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    assert!(shown.0[1].contains("무공상태회귀"));
    assert_eq!(shown.0[2], "────────┬──────┬──────────────");
    assert_eq!(shown.0.len(), 5);
    assert!(shown.0[3].contains("고영신공"));
    assert!(shown.0[3].contains("전투체력상승"));
    assert!(shown.0[3].contains("  150ː\x1b[33m━━━━━\x1b[37m━━━━━\x1b[37m"));
    assert_eq!(shown.0[4], "━━━━━━━━┷━━━━━━┷━━━━━━━━━━━━━━");
}

#[test]
fn mugong_admin_keeps_python_exact_and_prefix_ordinals_separate() {
    let mut viewer = Body::new();
    viewer.set("이름", "무공순번관리자");
    viewer.set("관리자등급", 1000_i64);

    let mut item = Object::new();
    item.set("이름", "무공충돌물건");
    item.set("반응이름", "혼합");
    let mut target = Body::new();
    target.set("이름", "무공조회대상");
    target.set("반응이름", "혼합접두별칭");
    set_precomputed_room_mugong_targets(vec![
        build_room_mugong_item_snapshot(&item),
        build_room_mugong_player_snapshot(&target),
    ]);

    let storage = ScriptStorage::default();
    let no_second_combined_match = storage
        .execute("무공", &mut viewer, "2혼합", None, None, None)
        .unwrap();
    assert_eq!(
        no_second_combined_match.0,
        vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]
    );

    let prefix_player = storage
        .execute("무공", &mut viewer, "혼합접", None, None, None)
        .unwrap();
    assert!(prefix_player.0[1].contains("무공조회대상의 무공"));

    set_precomputed_room_mugong_targets(Vec::new());
}

#[test]
fn mugong_status_admin_displays_runtime_mob_defense_effect() {
    let mut viewer = Body::new();
    viewer.set("이름", "몹상태관리자");
    viewer.set("관리자등급", 1000_i64);
    let mut data = RawMobData::new();
    data.name = "방어수련몹".to_string();
    data.reaction_names = vec!["방어수련".to_string()];
    let mut instance = MobInstance::new(
        "시험:방어수련몹".to_string(),
        "시험".to_string(),
        "1",
        &data,
    );
    instance.skills.push("고영신공".to_string());
    instance.skill_effects.push(crate::world::MobSkillEffect {
        name: "고영신공".to_string(),
        anti_type: "방어".to_string(),
        expires_at: chrono::Utc::now().timestamp() + 150,
        str_bonus: 0,
        dex_bonus: 0,
        arm_bonus: 0,
        mp_bonus: 0,
        max_mp_bonus: 0,
        hp_bonus: 0,
        max_hp_bonus: 0,
    });
    set_precomputed_room_mugong_targets(vec![build_room_mugong_mob_snapshot(&instance, &data)]);
    let shown = ScriptStorage::default()
        .execute("무공상태", &mut viewer, "방어수련", None, None, None)
        .unwrap();
    assert!(shown.0[1].contains("방어수련몹"));
    assert_eq!(shown.0[2], "────────┬──────┬──────────────");
    assert!(shown.0[3].contains("고영신공"));
    assert!(shown.0[3].contains("전투체력상승"));
    assert_eq!(shown.0[4], "━━━━━━━━┷━━━━━━┷━━━━━━━━━━━━━━");
}

#[test]
fn mugong_remove_admin_removes_mob_effect_and_rolls_back_modifiers() {
    let suffix = std::process::id();
    let viewer_name = format!("몹무공제거관리자-{suffix}");
    let zone = format!("몹무공제거존-{suffix}");
    let room = "1";
    let key = format!("{zone}:방어몹");
    let mut data = RawMobData::new();
    data.name = "제거방어몹".to_string();
    data.reaction_names = vec!["제거방어".to_string()];
    data.zone = zone.clone();
    let mut instance = MobInstance::new(key.clone(), zone.clone(), room, &data);
    instance.skills.push("고영신공".to_string());
    instance.str_modifier = 3;
    instance.arm_modifier = 7;
    instance.skill_effects.push(crate::world::MobSkillEffect {
        name: "고영신공".to_string(),
        anti_type: "방어".to_string(),
        expires_at: chrono::Utc::now().timestamp() + 150,
        str_bonus: 3,
        dex_bonus: 0,
        arm_bonus: 7,
        mp_bonus: 0,
        max_mp_bonus: 0,
        hp_bonus: 0,
        max_hp_bonus: 0,
    });
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        world.mob_cache.add_mob_instance(instance.clone());
        world.set_player_position(
            &viewer_name,
            PlayerPosition::new(zone.clone(), room.to_string()),
        );
    }
    let mut viewer = Body::new();
    viewer.set("이름", viewer_name.as_str());
    viewer.set("관리자등급", 1000_i64);
    let mut collision = Object::new();
    collision.set("이름", "제거충돌물건");
    collision.set("반응이름", "제거방어");
    set_precomputed_room_mugong_targets(vec![
        build_room_mugong_item_snapshot(&collision),
        build_room_mugong_mob_snapshot(&instance, &data),
    ]);
    let item_first = ScriptStorage::default()
        .execute(
            "무공제거",
            &mut viewer,
            "제거방어 고영신공",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(item_first.0, vec!["☞ 그런 대상이 없어요!"]);
    assert_eq!(
        get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, room)[0]
            .skill_effects
            .len(),
        1
    );
    set_precomputed_room_mugong_targets(vec![
        build_room_mugong_mob_snapshot(&instance, &data),
        build_room_mugong_item_snapshot(&collision),
    ]);
    let removed = ScriptStorage::default()
        .execute(
            "무공제거",
            &mut viewer,
            "제거방어 고영신공",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(removed.0, vec!["☞ 무공이 제거되었습니다."]);
    let missing = ScriptStorage::default()
        .execute(
            "무공제거",
            &mut viewer,
            "제거방어 고영신공",
            None,
            None,
            None,
        )
        .unwrap();
    assert!(
        missing.0.is_empty(),
        "Python is silent when no active effect matches"
    );
    let world = get_world_state().read().unwrap();
    let mob = world
        .mob_cache
        .get_all_mobs_in_room(&zone, room)
        .into_iter()
        .find(|mob| mob.mob_key == key)
        .unwrap();
    assert!(mob.skill_effects.is_empty());
    assert!(!mob.skills.iter().any(|name| name == "고영신공"));
    assert_eq!(mob.str_modifier, 0);
    assert_eq!(mob.arm_modifier, 0);
    drop(world);
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&viewer_name);
    world.mob_cache.remove_instance(&zone, room, &key);
}

#[test]
fn mugong_transfer_admin_teaches_runtime_mob_and_rejects_duplicate() {
    let suffix = std::process::id();
    let admin_name = format!("몹전수관리자-{suffix}");
    let zone = format!("몹전수존-{suffix}");
    let room = "1";
    let key = format!("{zone}:전수몹");
    let mut data = RawMobData::new();
    data.name = "전수대상몹".to_string();
    data.reaction_names = vec!["전수대상".to_string()];
    data.zone = zone.clone();
    let instance = MobInstance::new(key.clone(), zone.clone(), room, &data);
    {
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        world.mob_cache.add_mob_instance(instance.clone());
        world.set_player_position(
            &admin_name,
            PlayerPosition::new(zone.clone(), room.to_string()),
        );
    }
    set_precomputed_room_mugong_targets(vec![build_room_mugong_mob_snapshot(&instance, &data)]);
    let mut admin = Body::new();
    admin.set("이름", admin_name.as_str());
    admin.set("관리자등급", 2000_i64);
    let storage = ScriptStorage::default();
    let mut collision = Object::new();
    collision.set("이름", "전수충돌물건");
    collision.set("반응이름", "전수대상");
    set_precomputed_room_mugong_targets(vec![
        build_room_mugong_item_snapshot(&collision),
        build_room_mugong_mob_snapshot(&instance, &data),
    ]);
    let item_first = storage
        .execute(
            "무공전수",
            &mut admin,
            "전수대상 강룡십팔장",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(item_first.0, vec!["☞ 그런 대상이 없어요!"]);
    assert!(get_world_state()
        .read()
        .unwrap()
        .mob_cache
        .get_all_mobs_in_room(&zone, room)[0]
        .learned_skills
        .is_empty());
    set_precomputed_room_mugong_targets(vec![
        build_room_mugong_mob_snapshot(&instance, &data),
        build_room_mugong_item_snapshot(&collision),
    ]);
    let taught = storage
        .execute(
            "무공전수",
            &mut admin,
            "전수대상 강룡십팔장",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(taught.0, vec!["☞ 무공이 전수되었습니다."]);
    let duplicate = storage
        .execute(
            "무공전수",
            &mut admin,
            "전수대상 강룡십팔장",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(duplicate.0, vec!["☞ 이미 무공을 익히셨는걸요"]);
    let world = get_world_state().read().unwrap();
    let mob = world
        .mob_cache
        .get_all_mobs_in_room(&zone, room)
        .into_iter()
        .find(|mob| mob.mob_key == key)
        .unwrap();
    assert_eq!(mob.learned_skills, vec!["강룡십팔장"]);
    let learned_snapshot = build_room_mugong_mob_snapshot(mob, &data);
    drop(world);
    set_precomputed_room_mugong_targets(vec![learned_snapshot]);
    let listed = storage
        .execute("무공", &mut admin, "전수대상", None, None, None)
        .unwrap();
    assert!(listed.0[1].contains("◁ 전수대상몹의 무공 ▷"));
    assert!(listed.0[3].contains("강룡십팔장(1성)"));
    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&admin_name);
    world.mob_cache.remove_instance(&zone, room, &key);
}

#[test]
fn skill_list_command_sends_one_python_buffer_with_five_column_crlfs() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "무공목록회귀");
    body.set("관리자등급", 999_i64);
    let denied = storage
        .execute("무공리스트", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 1000_i64);
    let listed = storage
        .execute("무공리스트", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(
        listed.0.len(),
        1,
        "Python calls sendLine once for the whole buffer"
    );
    let text = &listed.0[0];
    let skill_count = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string("data/config/skill.json").unwrap(),
    )
    .unwrap()
    .as_object()
    .unwrap()
    .len();
    assert_eq!(text.matches(',').count(), skill_count);
    assert_eq!(text.matches("\r\n").count(), skill_count / 5);
    assert!(text.starts_with("          가의신공,"));
    let first_row = text.split("\r\n").next().unwrap();
    let columns = first_row.split(',').collect::<Vec<_>>();
    assert_eq!(
        columns.len(),
        6,
        "five entries plus the trailing empty field"
    );
    assert!(columns[..5]
        .iter()
        .all(|column| column.chars().count() == 14));
    assert_eq!(
        first_row,
        "          가의신공,           강뢰검,         강룡십팔장,           격공장,          격공진력,"
    );
    assert_eq!(text.ends_with("\r\n"), skill_count % 5 == 0);
}

#[test]
fn score_uses_live_inventory_weight_guild_title_and_has_no_extra_blank_line() {
    let suffix = std::process::id();
    let guild = format!("점수방파-{suffix}");
    let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
    crate::world::guild::guild_set(&guild, "장로명칭", "호법장로");

    let mut body = Body::new();
    body.set("이름", "점수회귀");
    body.set("레벨", 12_i64);
    body.set("나이", 34_i64);
    body.set("체력", 300_i64);
    body.set("최고체력", 300_i64);
    body.set("최대체력", 300_i64);
    body.set("내공", 20_i64);
    body.set("최고내공", 20_i64);
    body.set("최대내공", 20_i64);
    body.set("힘", 10_i64);
    body.set("맷집", 11_i64);
    body.set("민첩성", 12_i64);
    body.set("명중", 13_i64);
    body.set("회피", 14_i64);
    body.set("필살", 15_i64);
    body.set("운", 16_i64);
    body._str = 3;
    body._arm = 2;
    body._dex = 4;
    body._hit = 5;
    body._miss = 6;
    body._critical = 7;
    body._critical_chance = 8;
    body._mp = 50;
    body.set("소속", guild.as_str());
    body.set("직위", "장로");
    body.set("방파별호", "푸른별");
    body.set("소지품무게", 999_i64);
    body.set("특성치", 2_i64);
    let mut item = Object::new();
    item.set("이름", "무거운돌");
    item.set("무게", 7_i64);
    body.object.objs.push(Arc::new(Mutex::new(item)));

    let score = ScriptStorage::default()
        .execute("점수", &mut body, "무시", None, None, None)
        .unwrap();
    assert!(score
        .0
        .contains(&"│ [레  벨]        [    12] │ [나  이]              34 │".to_string()));
    assert!(
        score
            .0
            .iter()
            .any(|line| line.contains("[소지품]           7/130")),
        "{:?}",
        score.0
    );
    assert!(score
        .0
        .iter()
        .any(|line| line.starts_with("│ [  힘  ]      0 +     13 │")));
    assert!(score
        .0
        .iter()
        .any(|line| line.starts_with("│ [맷  집]      0 +     13 │")));
    assert!(score
        .0
        .iter()
        .any(|line| line.starts_with("│ [민  첩]              16 │")));
    assert!(score
        .0
        .iter()
        .any(|line| line == "│ [命  中]              18 │ [回  避]              20 │"));
    assert!(score
        .0
        .iter()
        .any(|line| line == "│ [必  殺]              22 │ [  運  ]              24 │"));
    assert!(score
        .0
        .iter()
        .any(|line| line.contains("[내  공]           30/20")));
    assert!(score
        .0
        .iter()
        .any(|line| line.contains("[직  위]        호법장로")));
    assert!(score.0.iter().any(|line| {
        line == &format!(
            "★ 당신은 \x1b[1m【{guild}】\x1b[0m 문파의 \x1b[1m호법장로(푸른별)\x1b[0m 입니다."
        )
    }));
    assert_eq!(
        score.0.last().unwrap(),
        "★ 당신은 2개의 여유 특성치를 보유하고 있습니다."
    );
    assert!(!score.0.iter().any(String::is_empty));

    crate::world::guild::guild_remove(&guild);
    std::fs::write("data/config/guild.json", guild_file_before).unwrap();
}

#[test]
fn score_age_column_uses_python_six_character_numeric_field() {
    let mut body = Body::new();
    body.set("이름", "큰나이점수");
    body.set("나이", 123_456_i64);
    body.set("최고체력", 1_i64);
    body.set("최대체력", 1_i64);
    body.set("최고내공", 1_i64);
    body.set("최대내공", 1_i64);

    let output = ScriptStorage::default()
        .execute("점수", &mut body, "", None, None, None)
        .unwrap()
        .0;

    assert!(output.iter().any(|line| {
        line == "│ [레  벨]        [     0] │ [나  이]          123456 │"
    }));
}

#[test]
fn score_normalizes_empty_gold_and_reports_negative_remaining_stats_like_python() {
    let mut body = Body::new();
    body.set("이름", "점수음수회귀");
    body.set("금전", "");
    body.set("특성치", -2_i64);
    body.set("최고체력", 1_i64);
    body.set("최대체력", 1_i64);
    body.set("최고내공", 1_i64);
    body.set("최대내공", 1_i64);

    let output = ScriptStorage::default()
        .execute("점수", &mut body, "", None, None, None)
        .unwrap()
        .0;
    assert_eq!(
        output.last().map(String::as_str),
        Some("★ 당신은 -2개의 여유 특성치를 보유하고 있습니다.")
    );
    assert_eq!(body.object.attr.get("금전"), Some(&Value::Int(0)));
    assert!(!output.iter().any(|line| line.contains("[금  전]")));
}

#[test]
fn score_hp_description_uses_live_highest_hp_not_maximum_cap_or_saved_file() {
    let mut body = Body::new();
    body.set("이름", format!("점수체력기준-{}", std::process::id()));
    body.set("체력", 75_i64);
    body.set("최고체력", 100_i64);
    body.set("최대체력", 50_i64);
    body.set("최고내공", 1_i64);
    body.set("최대내공", 1_i64);
    let output = ScriptStorage::default()
        .execute("점수", &mut body, "", None, None, None)
        .unwrap()
        .0;
    assert!(
        output.iter().any(|line| {
            line == "★ 당신의 이곳 저곳에 깊은 상처를 입었습니다."
        }),
        "{output:?}"
    );
    assert!(!output
        .iter()
        .any(|line| line == "★ 당신은 아주 활력이 넘칩니다."));
}

#[test]
fn test_mugong_self_output_matches_python_categories_width_and_visions() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "검객");
    body.skill_list = vec![
        "강룡십팔장".to_string(),
        "지르기".to_string(),
        "철포삼".to_string(),
    ];
    body.skill_map.insert(
        "강룡십팔장".to_string(),
        crate::player::SkillTraining::new(9, 42),
    );
    body.skill_map.insert(
        "지르기".to_string(),
        crate::player::SkillTraining::new(2, 5),
    );
    body.set("비전수련", "강룡십팔장비전 17");
    body.set("비전이름", "비전검법|비전도법|비전창법");

    let (output, special) = storage
        .execute("무공", &mut body, "", None, None, None)
        .unwrap();

    assert!(special.is_none());
    assert_eq!(output.len(), 8);
    assert_eq!(output[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    assert_eq!(
            output[1],
            "\x1b[0m\x1b[47m\x1b[30m◁ 당신의 무공 ▷                                                             \x1b[0m\x1b[40m\x1b[37m"
        );
    assert_eq!(output[2], "───────────────────────────────────────");
    assert_eq!(
        output[3],
        concat!(
            "\x1b[1m\x1b[40m\x1b[32m▷ 초급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            " ◇ 지르기(2성)          \r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 중급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 상급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 고급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 특급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            " ◇ 강룡십팔장(9성)      \r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 절정무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 회복무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 방어무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            " ◇ 철포삼(1성)          \r\n",
            "\x1b[1m\x1b[40m\x1b[32m▷ 기타무공\x1b[0m\x1b[40m\x1b[37m"
        )
    );
    assert_eq!(output[4], "───────────────────────────────────────");
    assert_eq!(
        output[5],
        "\x1b[1m\x1b[40m\x1b[32m▷ 비전\x1b[0m\x1b[40m\x1b[37m"
    );
    assert_eq!(
        output[6],
        concat!(
            "\x1b[1m\x1b[33m강룡십팔장비전 17\x1b[0m\x1b[40m\x1b[37m(수련중)\r\n",
            " ◇ 비전검법              ◇ 비전도법              ◇ 비전창법             "
        )
    );
    assert_eq!(output[7], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[test]
fn test_mugong_skill_cells_use_python_three_column_wrap() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "검객");
    body.skill_list = ["지르기", "비각", "원앙퇴", "쌍비각"]
        .into_iter()
        .map(str::to_string)
        .collect();

    let (output, _) = storage
        .execute("무공", &mut body, "", None, None, None)
        .unwrap();

    assert!(output[3].starts_with(concat!(
        "\x1b[1m\x1b[40m\x1b[32m▷ 초급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
        " ◇ 지르기(1성)           ◇ 비각(1성)             ◇ 원앙퇴(1성)          \r\n",
        " ◇ 쌍비각(1성)          "
    )));
}

#[test]
fn test_mugong_admin_uses_same_room_snapshot_and_regular_line_is_ignored() {
    let storage = ScriptStorage::default();
    let mut viewer = Body::new();
    viewer.set("이름", "관리자");
    viewer.set("관리자등급", 1000i64);

    let mut target = Body::new();
    target.set("이름", "대상");
    target.set("반응이름", "검객 대상자");
    target.skill_list.push("지르기".to_string());
    set_precomputed_room_mugong_targets(vec![build_room_mugong_player_snapshot(&target)]);

    let (output, _) = storage
        .execute("무공", &mut viewer, "대상", None, None, None)
        .unwrap();
    assert!(output[1].contains("◁ 대상의 무공 ▷"));

    viewer.set("관리자등급", 999i64);
    let (output, _) = storage
        .execute("무공", &mut viewer, "대상", None, None, None)
        .unwrap();
    clear_precomputed_all_online();
    assert!(output[1].contains("◁ 당신의 무공 ▷"));
}

#[test]
fn test_mugong_admin_can_view_python_mob_target_shape() {
    let storage = ScriptStorage::default();
    let mut viewer = Body::new();
    viewer.set("이름", "관리자");
    viewer.set("관리자등급", 1000i64);

    let mut data = RawMobData::new();
    data.name = "수련인".to_string();
    data.reaction_names = vec!["수련".to_string()];
    data.skills = vec![("지르기".to_string(), 100, 30)];
    let instance = MobInstance::new("시험:수련인".to_string(), "시험".to_string(), "1", &data);
    set_precomputed_room_mugong_targets(vec![build_room_mugong_mob_snapshot(&instance, &data)]);

    let (output, _) = storage
        .execute("무공", &mut viewer, "수련", None, None, None)
        .unwrap();
    clear_precomputed_all_online();

    assert!(output[1].contains("◁ 수련인의 무공 ▷"));
    // Python Mob.skillList는 무공 튜플 목록이고 skillMap은 비어 있으므로
    // 카테고리 머리말은 출력되지만 플레이어식 `지르기(1성)` 셀은 없다.
    assert!(output[3].contains("▷ 초급무공"));
    assert!(!output[3].contains("지르기(1성)"));

    let mut second_data = RawMobData::new();
    second_data.name = "수련인둘".to_string();
    let second = MobInstance::new(
        "시험:수련인둘".to_string(),
        "시험".to_string(),
        "1",
        &second_data,
    );
    set_precomputed_room_mugong_targets(vec![
        build_room_mugong_mob_snapshot(&instance, &data),
        build_room_mugong_mob_snapshot(&second, &second_data),
    ]);
    let (output, _) = storage
        .execute("무공", &mut viewer, "1", None, None, None)
        .unwrap();
    clear_precomputed_all_online();
    assert!(output[1].contains("◁ 수련인의 무공 ▷"));
}

#[test]
fn test_mugong_admin_rejects_item_and_uses_unified_order_collision() {
    let storage = ScriptStorage::default();
    let mut viewer = Body::new();
    viewer.set("이름", "관리자");
    viewer.set("관리자등급", 1000i64);

    let mut item = Object::new();
    item.set("이름", "옥패");
    set_precomputed_room_mugong_targets(vec![build_room_mugong_item_snapshot(&item)]);
    let (output, _) = storage
        .execute("무공", &mut viewer, "옥패", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);

    let mut player = Body::new();
    player.set("이름", "옥패");
    set_precomputed_room_mugong_targets(vec![
        build_room_mugong_player_snapshot(&player),
        build_room_mugong_item_snapshot(&item),
    ]);
    let (output, _) = storage
        .execute("무공", &mut viewer, "옥패", None, None, None)
        .unwrap();
    clear_precomputed_all_online();
    assert!(output[1].contains("◁ 옥패의 무공 ▷"));
}
