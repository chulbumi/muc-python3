use super::*;
use std::path::PathBuf;

fn repository_config(data_dir: PathBuf) -> ScriptConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ScriptConfig {
        script_dir: root.join("cmds"),
        hot_reload: false,
        extension: ".rhai".into(),
        data_dir,
        lib_dir: root.join("lib"),
    }
}

#[test]
fn help_command_matches_python_index_topic_missing_and_raw_crlf() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "도움말출력회귀");

    let index = storage
        .execute("도움말", &mut body, "", None, None, None)
        .unwrap();
    assert!(!index.0.is_empty());
    assert_ne!(index.0, vec!["☞ 해당 도움말이 없어요. ^^"]);

    // Player.parse_command strips the argument before CmdObj.cmd.
    let whitespace = storage
        .execute("도움말", &mut body, " \t ", None, None, None)
        .unwrap();
    assert_eq!(whitespace.0, index.0);

    let source: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("data/config/help.json").unwrap()).unwrap();
    let expected = source["도움말"]["감정표현"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\r\n");
    let topic = storage
        .execute("도움말", &mut body, " 감정표현 ", None, None, None)
        .unwrap();
    assert_eq!(topic.0, vec![expected]);

    let missing = storage
        .execute("도움말", &mut body, "__없는_도움말__", None, None, None)
        .unwrap();
    assert_eq!(missing.0, vec!["☞ 해당 도움말이 없어요. ^^"]);
}

#[test]
fn help_command_uses_python_style_startup_cache_until_reload() {
    let suffix = std::process::id();
    let data_dir = std::env::temp_dir().join(format!("muc-help-cache-{suffix}"));
    let config_dir = data_dir.join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    let help_path = config_dir.join("help.json");
    std::fs::write(
        &help_path,
        r#"{"도움말":{"도움말":["첫 색인"],"시험항목":["첫 줄","둘째 줄"]}}"#,
    )
    .unwrap();

    let global = crate::data::create_global_data(data_dir.clone());
    let storage = ScriptStorage::with_global_data(repository_config(config_dir), global.clone());
    let mut body = Body::new();
    body.set("이름", "도움말캐시회귀");

    let first = storage
        .execute("도움말", &mut body, "시험항목", None, None, None)
        .unwrap();
    assert_eq!(first.0, vec!["첫 줄\r\n둘째 줄"]);

    std::fs::write(
        &help_path,
        r#"{"도움말":{"도움말":["새 색인"],"시험항목":["갱신된 줄"]}}"#,
    )
    .unwrap();
    let before_reload = storage
        .execute("도움말", &mut body, "시험항목", None, None, None)
        .unwrap();
    assert_eq!(before_reload.0, first.0);

    assert!(global.write().unwrap().reload("help").unwrap());
    let after_reload = storage
        .execute("도움말", &mut body, "시험항목", None, None, None)
        .unwrap();
    assert_eq!(after_reload.0, vec!["갱신된 줄"]);

    let _ = std::fs::remove_dir_all(data_dir);
}
