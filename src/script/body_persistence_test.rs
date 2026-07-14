use super::*;
#[test]
fn mugong_json_round_trip_uses_python_arrays_and_rebuilds_runtime_state() {
    let path = std::env::temp_dir().join(format!(
        "muc_mugong_round_trip_{}_{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut body = Body::new();
    body.set("이름", "임시무공검사");
    body.skill_list = vec!["지르기".to_string(), "강룡십팔장".to_string()];
    body.skill_map.insert(
        "지르기".to_string(),
        crate::player::SkillTraining::new(2, 7),
    );
    body.skill_map.insert(
        "강룡십팔장".to_string(),
        crate::player::SkillTraining::new(9, 42),
    );
    body.set("비전이름", "비전검법|비전도법");
    assert!(save_body_to_json(&mut body, path.to_str().unwrap()));

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        json["사용자오브젝트"]["무공이름"],
        serde_json::json!(["지르기", "강룡십팔장"])
    );
    assert_eq!(
        json["사용자오브젝트"]["무공숙련도"],
        serde_json::json!(["지르기 2 7", "강룡십팔장 9 42"])
    );
    assert_eq!(
        json["사용자오브젝트"]["비전이름"],
        serde_json::json!(["비전검법", "비전도법"])
    );

    let mut loaded = Body::new();
    assert!(load_body_from_json(&mut loaded, path.to_str().unwrap()));
    assert_eq!(loaded.skill_list, vec!["지르기", "강룡십팔장"]);
    assert_eq!(
        loaded.skill_map.get("강룡십팔장"),
        Some(&crate::player::SkillTraining::new(9, 42))
    );
    let _ = std::fs::remove_file(path);
}
#[test]
fn save_body_emits_python_numeric_alias_defaults() {
    let path = std::env::temp_dir().join(format!("muc_numeric_alias_{}.json", std::process::id()));
    let mut body = Body::new();
    body.set("이름", "숫자별칭검사");
    body.set("최대체력", 450);
    assert!(save_body_to_json(&mut body, path.to_str().unwrap()));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let user = &json["사용자오브젝트"];
    assert_eq!(user["최고체력"], serde_json::json!(450));
    assert_eq!(user["맷집"], serde_json::json!(0));
    assert_eq!(user["내공"], serde_json::json!(0));
    let _ = std::fs::remove_file(path);
}

#[test]
fn save_body_emits_python_skill_lists_as_arrays_even_when_empty() {
    let path =
        std::env::temp_dir().join(format!("muc_empty_skill_lists_{}.json", std::process::id()));
    let mut body = Body::new();
    body.set("이름", "빈무공배열검사");
    assert!(save_body_to_json_without_timestamp(
        &mut body,
        path.to_str().unwrap()
    ));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let user = &json["사용자오브젝트"];
    assert_eq!(user["무공이름"], serde_json::json!([]));
    assert_eq!(user["무공숙련도"], serde_json::json!([]));
    assert_eq!(user["무공이름수련리스트"], serde_json::json!([]));
    let _ = std::fs::remove_file(path);
}

#[test]
fn item_skill_training_round_trip_keeps_python_array_shape_and_spaced_names() {
    let path = std::env::temp_dir().join(format!(
        "muc_item_skill_round_trip_{}.json",
        std::process::id()
    ));
    let source = serde_json::json!({
        "사용자오브젝트": {
            "이름": "아이템무공왕복",
            "무공이름": [],
            "무공숙련도": [],
            "무공이름수련리스트": ["첫 무기 12", "둘째 7"]
        },
        "아이템": []
    });
    std::fs::write(&path, serde_json::to_string_pretty(&source).unwrap()).unwrap();
    let mut body = Body::new();
    assert!(load_body_from_json(&mut body, path.to_str().unwrap()));
    assert_eq!(body.item_skill_map.get("첫 무기"), Some(&12));
    assert_eq!(body.item_skill_map.get("둘째"), Some(&7));
    assert!(save_body_to_json_without_timestamp(
        &mut body,
        path.to_str().unwrap()
    ));
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        saved["사용자오브젝트"]["무공이름수련리스트"],
        serde_json::json!(["첫 무기 12", "둘째 7"])
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rust_save_is_loadable_and_resaveable_by_the_actual_python_player() {
    let unique = format!(
        "러스트파이썬왕복{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let path = std::path::Path::new("data/user").join(format!("{unique}.json"));
    let mut body = Body::new();
    body.set("이름", unique.clone());
    body.item_skill_order = vec!["첫무기".to_string(), "둘째".to_string()];
    body.item_skill_map.insert("첫무기".to_string(), 12);
    body.item_skill_map.insert("둘째".to_string(), 7);
    body.set("설정상태", "자동습득 1\n전음거부 0");
    body.skill_state_loaded = true;
    assert!(save_body_to_json_without_timestamp(
        &mut body,
        path.to_str().unwrap()
    ));

    let script = r#"
import os
from client import Player
name = os.environ['MUC_ROUND_TRIP_USER']
player = Player()
assert player.load(name)
assert player.itemSkillMap == {'첫무기': 12, '둘째': 7}
assert player.Configs['자동습득'] is True
assert player.Configs['전음거부'] is False
assert player.save(False)
"#;
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .env("MUC_ROUND_TRIP_USER", &unique)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Python Player round trip failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let python_saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        python_saved["사용자오브젝트"]["무공이름수련리스트"],
        serde_json::json!(["첫무기 12", "둘째 7"])
    );
    assert_eq!(
        python_saved["사용자오브젝트"]["설정상태"],
        serde_json::json!(["자동습득 1", "전음거부 0"])
    );
    let mut rust_reloaded = Body::new();
    assert!(load_body_from_json(
        &mut rust_reloaded,
        path.to_str().unwrap()
    ));
    assert_eq!(rust_reloaded.item_skill_map.get("첫무기"), Some(&12));
    assert_eq!(rust_reloaded.item_skill_map.get("둘째"), Some(&7));
    let _ = std::fs::remove_file(path);
}

#[test]
fn python_string_array_attributes_keep_their_json_shape_across_rust_round_trip() {
    let path =
        std::env::temp_dir().join(format!("muc_body_array_shape_{}.json", std::process::id()));
    let source = serde_json::json!({
        "사용자오브젝트": {
            "이름": "배열형태왕복",
            "사용자정의배열": ["첫째", "둘째 값"],
            "빈사용자정의배열": []
        },
        "아이템": []
    });
    std::fs::write(&path, serde_json::to_string_pretty(&source).unwrap()).unwrap();
    let mut body = Body::new();
    assert!(load_body_from_json(&mut body, path.to_str().unwrap()));
    assert_eq!(body.get_string("사용자정의배열"), "첫째|둘째 값");
    assert!(save_body_to_json_without_timestamp(
        &mut body,
        path.to_str().unwrap()
    ));
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        saved["사용자오브젝트"]["사용자정의배열"],
        serde_json::json!(["첫째", "둘째 값"])
    );
    assert_eq!(
        saved["사용자오브젝트"]["빈사용자정의배열"],
        serde_json::json!([])
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn actual_python_save_survives_rust_load_save_and_python_reload() {
    let unique = format!(
        "파이썬러스트왕복{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let path = std::path::Path::new("data/user").join(format!("{unique}.json"));
    let create_script = r#"
import os
from client import Player
from objs.item import getItem
name = os.environ['MUC_ROUND_TRIP_USER']
player = Player()
player.set('이름', name)
player.skillList = ['지르기']
player.skillMap = {'지르기': (2, 7)}
player.itemSkillMap = {'첫무기': 12}
player.set('설정상태', ['자동습득 1', '전음거부 0'])
player.alias = {'봐': '점수'}
player.buildAlias()
item = getItem('1000').deepclone()
player.insert(item)
assert player.save(False)
"#;
    let created = std::process::Command::new("python3")
        .arg("-c")
        .arg(create_script)
        .env("MUC_ROUND_TRIP_USER", &unique)
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "Python fixture save failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&created.stdout),
        String::from_utf8_lossy(&created.stderr)
    );

    let mut body = Body::new();
    assert!(load_body_from_json(&mut body, path.to_str().unwrap()));
    assert_eq!(body.skill_list, vec!["지르기"]);
    assert_eq!(
        body.skill_map.get("지르기"),
        Some(&crate::player::SkillTraining::new(2, 7))
    );
    assert_eq!(body.item_skill_map.get("첫무기"), Some(&12));
    assert_eq!(body.get_string("설정상태"), "자동습득 1|전음거부 0");
    assert_eq!(body.object.objs.len(), 1);
    assert!(save_body_to_json_without_timestamp(
        &mut body,
        path.to_str().unwrap()
    ));

    let verify_script = r#"
import os
from client import Player
name = os.environ['MUC_ROUND_TRIP_USER']
player = Player()
assert player.load(name)
assert player.skillList == ['지르기']
assert player.skillMap == {'지르기': (2, 7)}
assert player.itemSkillMap == {'첫무기': 12}
assert player.Configs['자동습득'] is True
assert player.Configs['전음거부'] is False
assert player.alias == {'봐': '점수'}
assert len(player.objs) == 1 and player.objs[0].index == '1000'
"#;
    let verified = std::process::Command::new("python3")
        .arg("-c")
        .arg(verify_script)
        .env("MUC_ROUND_TRIP_USER", &unique)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "Python reload after Rust save failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&verified.stdout),
        String::from_utf8_lossy(&verified.stderr)
    );
    let _ = std::fs::remove_file(path);
}
