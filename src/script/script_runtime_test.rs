use super::*;

#[test]
fn room_admin_details_are_built_lazily_from_body_snapshots() {
    clear_precomputed_room_admin_bodies();
    let mut target = Body::new();
    target.set("이름", "대상무사");
    target.set("레벨", 37_i64);
    target.set("체력", 432_i64);
    target.set("은전", 9876_i64);
    target.set("임의속성", "보존값");

    set_precomputed_room_admin_bodies(vec![("대상무사".to_string(), target)]);
    let actor = Body::new();
    assert!(actor.temp().get("_online_room_admin").is_none());

    let values = room_admin_player_values(&actor).expect("lazy room snapshot should be available");
    let value = values
        .iter()
        .find(|value| value.get("name").and_then(serde_json::Value::as_str) == Some("대상무사"))
        .expect("target snapshot");
    assert_eq!(
        value.get("level").and_then(serde_json::Value::as_i64),
        Some(37)
    );
    assert_eq!(
        value.get("hp").and_then(serde_json::Value::as_i64),
        Some(432)
    );
    assert_eq!(
        value.get("silver").and_then(serde_json::Value::as_i64),
        Some(9876)
    );
    assert_eq!(
        value
            .get("raw_attrs")
            .and_then(|attrs| attrs.get("임의속성"))
            .and_then(serde_json::Value::as_str),
        Some("보존값")
    );
    assert_eq!(
        room_admin_player_values(&actor).expect("cached lazy snapshot"),
        values
    );
    clear_precomputed_room_admin_bodies();
}

#[test]
#[ignore = "microbenchmark; run explicitly with --ignored --nocapture"]
fn benchmark_lazy_room_admin_snapshot_against_eager_json() {
    let bodies: Vec<(String, Body)> = (0..40)
        .map(|index| {
            let mut body = Body::new();
            body.set("이름", format!("측정무사{index}"));
            body.set("레벨", 30_i64 + index);
            body.set("체력", 1000_i64 + index);
            body.set("최고체력", 1200_i64 + index);
            for attr in 0..80 {
                body.set(&format!("측정속성{attr}"), index * 100 + attr);
            }
            (format!("측정무사{index}"), body)
        })
        .collect();
    let iterations = 200;

    let eager_started = std::time::Instant::now();
    for _ in 0..iterations {
        let values: Vec<_> = bodies
            .iter()
            .map(|(name, body)| build_room_admin_player_value(name, body))
            .collect();
        std::hint::black_box(serde_json::to_string(&values).unwrap());
    }
    let eager = eager_started.elapsed();

    let lazy_started = std::time::Instant::now();
    for _ in 0..iterations {
        set_precomputed_room_admin_bodies(std::hint::black_box(bodies.clone()));
        clear_precomputed_room_admin_bodies();
    }
    let lazy = lazy_started.elapsed();

    println!(
        "room admin snapshot {iterations}x40: eager={eager:?}, lazy={lazy:?}, speedup={:.1}x",
        eager.as_secs_f64() / lazy.as_secs_f64()
    );
    assert!(lazy < eager, "lazy snapshot should be faster");
}

#[test]
fn get_skill_data_uses_global_data_snapshot_instead_of_reading_the_file() {
    // `data/config/skill.json` exists in this repository.  An empty
    // GlobalData must therefore still produce UNIT: otherwise the base
    // engine's file-backed efun was not replaced.
    let global_data = std::sync::Arc::new(std::sync::RwLock::new(crate::data::GlobalData::new(
        std::path::PathBuf::from("missing-data-directory"),
    )));
    let engine = create_engine_with_global_data(global_data);

    let value = engine
        .eval::<rhai::Dynamic>(r#"get_skill_data("가의신공")"#)
        .expect("get_skill_data efun should execute");
    assert!(value.is_unit());
}

#[test]
fn command_engine_get_skill_data_uses_global_data_snapshot() {
    let global_data = std::sync::Arc::new(std::sync::RwLock::new(crate::data::GlobalData::new(
        std::path::PathBuf::from("missing-data-directory"),
    )));
    let mut body = Body::new();
    let engine = create_engine_with_body_and_output(
        &mut body,
        Arc::new(Mutex::new(Vec::new())),
        None,
        None,
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(Vec::new())),
        None,
        None,
        Some(global_data),
    );

    let value = engine
        .eval::<rhai::Dynamic>(r#"get_skill_data("가의신공")"#)
        .expect("get_skill_data efun should execute");
    assert!(value.is_unit());
}

#[test]
fn command_engine_get_murim_config_uses_global_data_snapshot() {
    // The repository's murim.json contains this key. An empty GlobalData must
    // still return UNIT, proving that command execution did not fall through
    // to the synchronous file-backed efun.
    let global_data = std::sync::Arc::new(std::sync::RwLock::new(crate::data::GlobalData::new(
        std::path::PathBuf::from("missing-data-directory"),
    )));
    let mut body = Body::new();
    let engine = create_engine_with_body_and_output(
        &mut body,
        Arc::new(Mutex::new(Vec::new())),
        None,
        None,
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(Vec::new())),
        None,
        None,
        Some(global_data),
    );

    let value = engine
        .eval::<rhai::Dynamic>(r#"get_murim_config("입력초과에러수")"#)
        .expect("get_murim_config efun should execute");
    assert!(value.is_unit());
}

#[test]
fn cached_murim_config_observes_admin_reload_without_rebuilding_the_engine() {
    let root = std::env::temp_dir().join(format!(
        "muc_murim_cache_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config_dir = root.join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    let path = config_dir.join("murim.json");
    std::fs::write(&path, r#"{"메인설정":{"시험값":1}}"#).unwrap();

    let global_data = crate::data::create_global_data(root.clone());
    let engine = create_engine_with_global_data(global_data.clone());
    assert_eq!(
        engine.eval::<i64>(r#"get_murim_config("시험값")"#).unwrap(),
        1
    );

    std::fs::write(&path, r#"{"메인설정":{"시험값":2}}"#).unwrap();
    assert!(global_data.write().unwrap().reload("murim").unwrap());
    assert_eq!(
        engine.eval::<i64>(r#"get_murim_config("시험값")"#).unwrap(),
        2
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "manual performance comparison"]
fn benchmark_cached_murim_config_against_file_backed_efun() {
    let global_data = crate::data::create_global_data(std::path::PathBuf::from("data"));
    let cached_engine = create_engine_with_global_data(global_data);
    let file_engine = create_engine();
    let script = r#"
        let total = 0;
        for n in 0..1_000 {
            total += get_murim_config("입력초과에러수");
        }
        total
    "#;

    let started = std::time::Instant::now();
    let file_value = file_engine.eval::<i64>(script).expect("file-backed lookup");
    let file_elapsed = started.elapsed();
    let started = std::time::Instant::now();
    let cached_value = cached_engine.eval::<i64>(script).expect("cached lookup");
    let cached_elapsed = started.elapsed();

    assert_eq!(cached_value, file_value);
    eprintln!(
        "murim config 1000 lookups: file={file_elapsed:?}, cached={cached_elapsed:?}, speedup={:.1}x",
        file_elapsed.as_secs_f64() / cached_elapsed.as_secs_f64()
    );
}

#[test]
fn item_object_template_cache_returns_independent_clones_and_hot_reloads() {
    let root = std::env::temp_dir().join(format!(
        "muc_item_template_cache_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("시험품.json");
    std::fs::write(&path, r#"{"아이템정보":{"이름":"첫이름","무게":3}}"#).unwrap();

    let (first, first_name) = object_from_item_path(&path, "시험품").unwrap();
    assert_eq!(first_name, "첫이름");
    first.lock().unwrap().set("이름", "변경된인스턴스");
    let (second, second_name) = object_from_item_path(&path, "시험품").unwrap();
    assert_eq!(second_name, "첫이름");
    assert_eq!(second.lock().unwrap().getName(), "첫이름");

    // Different length also makes this deterministic on filesystems whose
    // modification timestamp has coarse granularity.
    std::fs::write(
        &path,
        r#"{"아이템정보":{"이름":"핫리로드된긴이름","무게":7}}"#,
    )
    .unwrap();
    let (reloaded, reloaded_name) = object_from_item_path(&path, "시험품").unwrap();
    assert_eq!(reloaded_name, "핫리로드된긴이름");
    assert_eq!(reloaded.lock().unwrap().getInt("무게"), 7);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "manual performance comparison"]
fn benchmark_cached_item_object_against_file_parse() {
    let path = Path::new("data/item/1037.json");
    let iterations = 1_000;

    let started = std::time::Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(load_item_object_from_path(path, "1037").unwrap());
    }
    let file_elapsed = started.elapsed();

    std::hint::black_box(object_from_item_path(path, "1037").unwrap());
    let started = std::time::Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(object_from_item_path(path, "1037").unwrap());
    }
    let cached_elapsed = started.elapsed();

    eprintln!(
        "item template {iterations} loads: file={file_elapsed:?}, cached={cached_elapsed:?}, speedup={:.1}x",
        file_elapsed.as_secs_f64() / cached_elapsed.as_secs_f64()
    );
}

#[test]
fn every_current_rhai_source_is_syntax_valid() {
    let mut storage = ScriptStorage::default();
    storage
        .load_all_scripts_checked()
        .expect("every cmds/*.rhai source must compile before registration");
    assert_eq!(storage.script_names().len(), 212);
}
#[test]
fn test_script_preserves_self_output_with_targeted_sends() {
    let root = std::env::temp_dir().join(format!(
        "muc_script_combined_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("combined.rhai"),
        r#"fn main(ob, line) {
                send_line(ob, "self");
                send_to_user("other", "target");
            }"#,
    )
    .unwrap();

    let config = ScriptConfig {
        script_dir: root.clone(),
        ..ScriptConfig::default()
    };
    let storage = ScriptStorage::new(config);
    let mut body = Body::new();
    body.set("이름", "self");
    let (output, special) = storage
        .execute("combined", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["self"]);
    assert!(matches!(
        special,
        Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
            if own == "self" && sends == &vec![("other".to_string(), "target".to_string())]
    ));

    let _ = std::fs::remove_dir_all(root);
}
#[test]
fn test_json_debug_command_requires_level_2000() {
    let storage = ScriptStorage::default();
    assert!(storage.has_script("제이슨"));
    let mut body = Body::new();
    body.set("이름", "일반사용자");
    body.set("관리자등급", 0i64);

    let (output, _) = storage
        .execute("제이슨", &mut body, "../user/밍밍", None, None, None)
        .unwrap();
    assert_eq!(output, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    body.set("관리자등급", 2000_i64);
    body.set("한글키", "값😀");
    let (admin_output, special) = storage
        .execute("제이슨", &mut body, "무시", None, None, None)
        .unwrap();
    assert!(
        admin_output.is_empty(),
        "Python writes only to the server console"
    );
    assert!(special.is_none());
    assert_eq!(python_json_ensure_ascii("한😀"), "\\ud55c\\ud83d\\ude00");
}
#[test]
fn test_read_text_file_is_confined_to_public_data() {
    assert!(read_public_text_file("/etc/passwd").is_empty());
    assert!(read_public_text_file("data/config/../user/밍밍.json").is_empty());
    if Path::new("data/text/notice.txt").exists() {
        assert!(!read_public_text_file("data/text/notice.txt").is_empty());
    }
}
#[test]
fn test_has_script() {
    let storage = ScriptStorage::default();
    assert!(!storage.has_script("nonexistent"));
}
#[test]
fn test_script_storage_new() {
    let storage = ScriptStorage::default();
    assert!(storage.config.script_dir.ends_with("cmds"));
}

#[test]
fn cached_ast_reloads_only_after_valid_script_or_library_change() {
    let root = std::env::temp_dir().join(format!(
        "muc_script_ast_cache_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let commands = root.join("cmds");
    let libraries = root.join("lib");
    std::fs::create_dir_all(&commands).unwrap();
    std::fs::create_dir_all(&libraries).unwrap();
    let command_path = commands.join("cached.rhai");
    let library_path = libraries.join("value.rhai");
    std::fs::write(&library_path, r#"fn lib_value() { "one" }"#).unwrap();
    std::fs::write(
        &command_path,
        r#"fn main(ob, line) { send_line(ob, "cmd1-" + lib_value()); }"#,
    )
    .unwrap();

    let config = ScriptConfig {
        script_dir: commands,
        lib_dir: libraries,
        ..ScriptConfig::default()
    };
    let mut storage = ScriptStorage::new(config);
    let mut body = Body::new();
    let run = |storage: &ScriptStorage, body: &mut Body| {
        storage
            .execute("cached", body, "", None, None, None)
            .unwrap()
            .0
    };
    assert_eq!(run(&storage, &mut body), vec!["cmd1-one"]);

    // Execution uses the cached AST, not the retained source string.
    storage.scripts.get_mut("cached").unwrap().source = "invalid source".to_string();
    assert_eq!(run(&storage, &mut body), vec!["cmd1-one"]);

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(
        &command_path,
        r#"fn main(ob, line) { send_line(ob, "cmd2-" + lib_value()); }"#,
    )
    .unwrap();
    assert!(storage.reload_script("cached").unwrap());
    assert_eq!(run(&storage, &mut body), vec!["cmd2-one"]);

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&library_path, r#"fn lib_value() { "two" }"#).unwrap();
    assert!(storage.reload_all().unwrap() >= 1);
    assert_eq!(run(&storage, &mut body), vec!["cmd2-two"]);

    // A bad hot reload never replaces the last valid source/revision/AST.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&command_path, "fn main(").unwrap();
    assert!(storage.reload_script("cached").is_err());
    assert_eq!(run(&storage, &mut body), vec!["cmd2-two"]);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn different_users_do_not_share_a_global_rhai_execution_mutex() {
    let root = std::env::temp_dir().join(format!(
        "muc_script_parallel_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("parallel.rhai"),
        r#"fn main(ob, line) { __test_sleep_ms(500); send_line(ob, line); }"#,
    )
    .unwrap();
    let storage = Arc::new(ScriptStorage::new(ScriptConfig {
        script_dir: root.clone(),
        lib_dir: root.join("lib"),
        ..ScriptConfig::default()
    }));
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let started = std::time::Instant::now();
    let handles = ["first", "second"].map(|line| {
        let storage = storage.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let mut body = Body::new();
            barrier.wait();
            storage
                .execute("parallel", &mut body, line, None, None, None)
                .unwrap()
                .0
        })
    });
    barrier.wait();
    let outputs = handles.map(|handle| handle.join().unwrap());
    let elapsed = started.elapsed();

    assert_eq!(outputs[0], vec!["first"]);
    assert_eq!(outputs[1], vec!["second"]);
    assert!(
        elapsed < std::time::Duration::from_millis(850),
        "two 500ms Rhai commands were serialized: {elapsed:?}"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_script_config_default() {
    let config = ScriptConfig::default();
    assert_eq!(config.script_dir, PathBuf::from("cmds"));
    assert!(config.hot_reload);
    assert_eq!(config.extension, ".rhai");
}
#[test]
fn password_hash_uses_bcrypt_and_reads_legacy_formats() {
    let stored = password_hash("새암호");
    assert!(stored.starts_with("$2"));
    assert_eq!(stored.split('$').nth(2), Some("10"));
    assert!(password_verify(&stored, "새암호"));
    assert!(!password_verify(&stored, "틀린암호"));
    assert!(!password_needs_upgrade(&stored));

    let old_bcrypt = bcrypt::hash("옛암호", bcrypt::DEFAULT_COST).unwrap();
    assert!(password_verify(&old_bcrypt, "옛암호"));
    assert!(password_needs_upgrade(&old_bcrypt));

    let prehashed = bcrypt::hash(password_sha256("전처리암호"), bcrypt::DEFAULT_COST).unwrap();
    let tagged_prehashed = format!("{BCRYPT_SHA256_PREFIX}{prehashed}");
    assert!(password_verify(&tagged_prehashed, "전처리암호"));
    assert!(password_needs_upgrade(&tagged_prehashed));

    assert!(password_verify("평문암호", "평문암호"));

    use sha2::{Digest, Sha512};
    let old_sha512 = format!("{:x}", Sha512::digest("해시암호".as_bytes()));
    assert!(password_verify(&old_sha512, "해시암호"));
    assert!(password_needs_upgrade(&old_sha512));
}
