use super::*;

#[test]
fn raise_only_allocates_hit_evasion_critical_and_luck() {
    let suffix = std::process::id();
    let name = format!("올려재배분-{suffix}");
    let path = format!("data/user/{name}.json");
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", name);
    body.set("특성치", 4_i64);

    for denied in ["", "힘", "민첩성", "맷집", "내공", "체력", "지능"] {
        let result = storage
            .execute("올려", &mut body, denied, None, None, None)
            .unwrap();
        assert_eq!(result.0, vec!["☞ 사용법: [명중|회피|필살|운] 올려"]);
    }
    for stat in ["명중", "회피", "필살", "운"] {
        let result = storage
            .execute("올려", &mut body, stat, None, None, None)
            .unwrap();
        assert_eq!(result.0, vec![format!("☞ [{stat}] 특성치를 올렸습니다.")]);
        assert_eq!(body.get_int(stat), 1);
        assert_eq!(body.get_int(&format!("{stat}특성치")), 1);
    }
    assert_eq!(body.get_int("특성치"), 0);
    let exhausted = storage
        .execute("올려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(
        exhausted.0,
        vec!["☞ 더이상 올릴 수 있는 여유 특성치가 없습니다."]
    );

    body.set("특성치", 1_i64);
    body.set("명중", 100_i64);
    let capped = storage
        .execute("올려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(capped.0, vec!["☞ 더이상 올릴 수 없습니다."]);
    assert_eq!(body.get_int("특성치"), 1);
    assert!(std::path::Path::new(&path).exists());
    let _ = std::fs::remove_file(path);
}

#[test]
fn lower_only_refunds_points_allocated_to_the_four_reassignable_stats() {
    let suffix = std::process::id();
    let name = format!("내려재배분-{suffix}");
    let path = format!("data/user/{name}.json");
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", name);
    body.set("특성치", 0_i64);

    for denied in ["", "힘", "민첩성", "맷집", "내공", "체력", "지능"] {
        let result = storage
            .execute("내려", &mut body, denied, None, None, None)
            .unwrap();
        assert_eq!(result.0, vec!["☞ 사용법: [명중|회피|필살|운] 내려"]);
    }

    for stat in ["명중", "회피", "필살", "운"] {
        body.set(stat, 10_i64);
        body.set(&format!("{stat}특성치"), 1_i64);
        let result = storage
            .execute("내려", &mut body, stat, None, None, None)
            .unwrap();
        assert_eq!(result.0, vec![format!("☞ [{stat}] 특성치를 내렸습니다.")]);
        assert_eq!(body.get_int(stat), 9);
        assert_eq!(body.get_int(&format!("{stat}특성치")), 0);
    }
    assert_eq!(body.get_int("특성치"), 4);

    let none_left = storage
        .execute("내려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(none_left.0, vec!["☞ [명중] 더이상 내릴 수 없습니다."]);

    let mut loaded = Body::new();
    assert!(load_body_from_json(&mut loaded, &path));
    assert_eq!(loaded.get_int("특성치"), 4);
    for stat in ["명중", "회피", "필살", "운"] {
        assert_eq!(loaded.get_int(stat), 9);
        assert_eq!(loaded.get_int(&format!("{stat}특성치")), 0);
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn legacy_four_stat_value_becomes_the_initial_reassignable_baseline() {
    let suffix = std::process::id();
    let name = format!("특성레거시-{suffix}");
    let path = format!("data/user/{name}.json");
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", name);
    body.set("명중", 5_i64);
    body.set("특성치", 0_i64);
    body.object.attr.remove("명중특성치");

    storage
        .execute("내려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(body.get_int("명중"), 4);
    assert_eq!(body.get_int("명중특성치"), 4);
    assert_eq!(body.get_int("특성치"), 1);
    let _ = std::fs::remove_file(path);
}
