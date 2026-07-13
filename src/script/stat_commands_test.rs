use super::*;
#[test]
fn raise_stat_matches_python_usage_caps_resource_growth_and_zero_points() {
    let suffix = std::process::id();
    let name = format!("올려경계검사-{suffix}");
    let path = std::path::Path::new("data/user").join(format!("{name}.json"));
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("특성치", 1_i64);
    body.set("최고체력", 0_i64);

    for input in ["", "지능"] {
        let usage = storage
            .execute("올려", &mut body, input, None, None, None)
            .unwrap();
        assert_eq!(
            usage.0,
            vec!["☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 올려"]
        );
    }

    body.set("명중", 100_i64);
    let capped_hit = storage
        .execute("올려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(capped_hit.0, vec!["☞ 더이상 올릴 수 없습니다."]);
    assert_eq!(body.get_int("특성치"), 1);

    body.set("민첩성", 2800_i64);
    let capped_dex = storage
        .execute("올려", &mut body, "민첩성", None, None, None)
        .unwrap();
    assert_eq!(capped_dex.0, vec!["☞ 더이상 올릴 수 없습니다."]);
    assert_eq!(body.get_int("특성치"), 1);

    body.set("최고내공", 20_i64);
    let mp = storage
        .execute("올려", &mut body, "내공", None, None, None)
        .unwrap();
    assert_eq!(mp.0, vec!["☞ [내공] 특성치를 올렸습니다."]);
    assert_eq!(body.get_int("최고내공"), 30);
    assert_eq!(body.get_int("내공특성치"), 1);
    assert_eq!(body.get_int("특성치"), 0);

    let exhausted = storage
        .execute("올려", &mut body, "체력", None, None, None)
        .unwrap();
    assert_eq!(
        exhausted.0,
        vec!["☞ 더이상 올릴 수 있는 여유 특성치가 없습니다."]
    );
    assert_eq!(body.get_int("최고체력"), 0);

    body.set("특성치", 1_i64);
    body.set("최고체력", 500_i64);
    let hp = storage
        .execute("올려", &mut body, "체력", None, None, None)
        .unwrap();
    assert_eq!(hp.0, vec!["☞ [체력] 특성치를 올렸습니다."]);
    assert_eq!(body.get_int("최고체력"), 600);
    assert_eq!(body.get_int("체력특성치"), 1);
    assert!(path.exists(), "Python ob.save() counterpart must persist");
    let _ = std::fs::remove_file(path);
}
#[test]
fn raise_stat_matches_python_normalized_input_negative_points_and_explicit_zero() {
    let suffix = std::process::id();
    let name = format!("올려검사-{suffix}");
    let path = std::path::Path::new("data/user").join(format!("{name}.json"));
    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("특성치", -1_i64);
    body.set("힘", 10_i64);
    body.set("명중", 5_i64);
    body.set("명중특성치", 0_i64);
    let storage = ScriptStorage::default();

    let spaced = storage
        .execute("올려", &mut body, " 힘  ", None, None, None)
        .unwrap();
    assert_eq!(spaced.0, vec!["☞ [힘] 특성치를 올렸습니다."]);
    assert_eq!(body.get_int("특성치"), -2);
    assert_eq!(body.get_int("힘"), 11);

    let strength = storage
        .execute("올려", &mut body, "힘", None, None, None)
        .unwrap();
    assert_eq!(strength.0, vec!["☞ [힘] 특성치를 올렸습니다."]);
    assert_eq!(body.get_int("특성치"), -3);
    assert_eq!(body.get_int("힘"), 12);
    assert_eq!(body.get_int("힘특성치"), 2);

    let hit = storage
        .execute("올려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(hit.0, vec!["☞ [명중] 특성치를 올렸습니다."]);
    assert_eq!(body.get_int("명중특성치"), 1);
    assert_eq!(body.get_int("명중"), 6);
    assert_eq!(body.get_int("특성치"), -4);

    let _ = std::fs::remove_file(path);
}

#[test]
fn lower_trait_command_distinguishes_missing_from_explicit_zero_like_python() {
    let suffix = std::process::id();
    let name = format!("특성내려-{suffix}");
    let path = format!("data/user/{name}.json");
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", name.as_str());
    body.set("힘", 5_i64);

    let usage = storage
        .execute("내려", &mut body, "잘못", None, None, None)
        .unwrap();
    assert_eq!(
        usage.0,
        vec!["☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려"]
    );
    body.set("힘특성치", 2_i64);
    let spaced = storage
        .execute("내려", &mut body, " 힘 ", None, None, None)
        .unwrap();
    assert_eq!(spaced.0, vec!["☞ [힘] 특성치를 내렸습니다."]);
    assert_eq!(body.get_int("힘특성치"), 1);
    assert_eq!(body.get_int("힘"), 4);
    body.set("힘", 5_i64);
    body.set("특성치", 0_i64);
    body.object.attr.remove("힘특성치");
    let ordinary_missing = storage
        .execute("내려", &mut body, "힘", None, None, None)
        .unwrap();
    assert_eq!(ordinary_missing.0, vec!["☞ [힘] 더이상 내릴 수 없습니다."]);
    assert_eq!(body.get_int("힘"), 5);

    body.set("힘특성치", 2_i64);
    let strength = storage
        .execute("내려", &mut body, "힘", None, None, None)
        .unwrap();
    assert_eq!(strength.0, vec!["☞ [힘] 특성치를 내렸습니다."]);
    assert_eq!(body.get_int("힘특성치"), 1);
    assert_eq!(body.get_int("힘"), 4);
    assert_eq!(body.get_int("특성치"), 1);
    assert!(body.get_int("마지막저장시간") > 0);
    assert!(std::path::Path::new(&path).exists());

    body.set("명중", 5_i64);
    body.set("명중특성치", 0_i64);
    let explicit_zero = storage
        .execute("내려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(explicit_zero.0, vec!["☞ [명중] 더이상 내릴 수 없습니다."]);
    assert_eq!(body.get_int("명중"), 5);
    body.object.attr.remove("명중특성치");
    let legacy_fallback = storage
        .execute("내려", &mut body, "명중", None, None, None)
        .unwrap();
    assert_eq!(legacy_fallback.0, vec!["☞ [명중] 특성치를 내렸습니다."]);
    assert_eq!(body.get_int("명중특성치"), 4);
    assert_eq!(body.get_int("명중"), 4);
    assert_eq!(body.get_int("특성치"), 2);

    body.set("내공특성치", 1_i64);
    body.set("최고내공", 50_i64);
    let mp = storage
        .execute("내려", &mut body, "내공", None, None, None)
        .unwrap();
    assert_eq!(mp.0, vec!["☞ [내공] 특성치를 내렸습니다."]);
    assert_eq!(body.get_int("최고내공"), 40);
    body.set("체력특성치", 1_i64);
    body.set("최고체력", 500_i64);
    let hp = storage
        .execute("내려", &mut body, "체력", None, None, None)
        .unwrap();
    assert_eq!(hp.0, vec!["☞ [체력] 특성치를 내렸습니다."]);
    assert_eq!(body.get_int("최고체력"), 400);
    assert_eq!(body.get_int("특성치"), 4);

    for stat in ["민첩성", "맷집", "회피", "필살", "운"] {
        body.set(stat, 10_i64);
        body.set(&format!("{stat}특성치"), 1_i64);
        let lowered = storage
            .execute("내려", &mut body, stat, None, None, None)
            .unwrap();
        assert_eq!(lowered.0, vec![format!("☞ [{stat}] 특성치를 내렸습니다.")]);
        assert_eq!(body.get_int(stat), 9, "{stat} base value");
        assert_eq!(body.get_int(&format!("{stat}특성치")), 0);
    }
    assert_eq!(body.get_int("특성치"), 9);

    let mut loaded = Body::new();
    assert!(load_body_from_json(&mut loaded, &path));
    assert_eq!(loaded.get_int("특성치"), 9);
    assert_eq!(loaded.get_int("최고내공"), 40);
    assert_eq!(loaded.get_int("최고체력"), 400);
    for stat in ["민첩성", "맷집", "회피", "필살", "운"] {
        assert_eq!(loaded.get_int(stat), 9, "persisted {stat}");
    }

    let _ = std::fs::remove_file(path);
}
