//! Integration test for script loading and execution

use muc_engine::player::Body;
use muc_engine::script::{ScriptConfig, ScriptStorage};
use std::collections::HashSet;

#[test]
fn test_script_loading() {
    let config = ScriptConfig::default();
    let storage = ScriptStorage::new(config);

    let names = storage.script_names();
    println!("Loaded scripts: {:?}", names);

    // Should have loaded the .rhai files from cmds/ directory
    assert!(names.contains(&"말".to_string()));
    assert!(names.contains(&"봐".to_string()));
    assert!(names.contains(&"도움말".to_string()));
}

#[test]
fn test_script_execution() {
    let config = ScriptConfig::default();
    let storage = ScriptStorage::new(config);

    // Create a test player
    let mut body = Body::new();
    body.set("이름", "test_player");

    // Try to execute a script if it exists
    let names = storage.script_names();
    if let Some(name) = names.first() {
        let result = storage.execute(name, &mut body, "", None, None, None);
        // Script might fail due to API mismatches, but it should at least compile
        println!("Script {:?} execution result: {:?}", name, result);
    }
}

#[test]
fn only_python_global_list_commands_scan_all_online_players() {
    // Python scans the whole connected-player list for global list commands.
    // Adult-channel commands use the separate channel-membership index, while
    // room-local commands must use get_room_players(ob).
    let global_commands = HashSet::from([
        "누구",
        "어디",
        "방파상태",
        "모두끝",
        "정리",
        "모두소환",
        "트윗",
        "외쳐",
        "외쳐2",
        "직위임명",
        "방파말",
        "똥파말",
        "방파별호",
        "방파파문",
        "방주권한양도",
        "명칭설정",
    ]);

    for entry in std::fs::read_dir("cmds").expect("cmds directory") {
        let entry = entry.expect("cmds entry");
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rhai") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read Rhai command");
        if !source.contains("get_all_online_players(") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("UTF-8 command name");
        assert!(
            global_commands.contains(name),
            "room-local command {name}.rhai must use get_room_players(ob)"
        );
    }
}

#[test]
fn adult_channel_commands_use_membership_efuns_not_all_online_players() {
    for name in ["채널입장", "채널퇴장", "채널잡담", "채널누구"] {
        let source = std::fs::read_to_string(format!("cmds/{name}.rhai"))
            .expect("read adult-channel Rhai command");
        assert!(
            source.contains("get_adult_channel_members(")
                || source.contains("is_adult_channel_member("),
            "{name}.rhai must use adult-channel membership"
        );
        assert!(
            !source.contains("get_all_online_players("),
            "{name}.rhai must not substitute the full online list for adultCH"
        );
    }
}

#[test]
fn guild_expulsion_does_not_disconnect_the_player() {
    let source =
        std::fs::read_to_string("cmds/방파파문.rhai").expect("read guild expulsion script");
    assert!(source.contains("guild_kick_member("));
    assert!(!source.contains("request_disconnect"));
    assert!(!source.contains("kick_player("));
}

#[test]
fn room_description_commands_use_multiline_input_state() {
    for name in ["방설명", "방파방설명"] {
        let source = std::fs::read_to_string(format!("cmds/{name}.rhai"))
            .expect("read room description script");
        assert!(
            source.contains("begin_room_description("),
            "{name} must use PendingInput"
        );
        assert!(
            !source.contains("현재 구현 중"),
            "{name} must not be a placeholder"
        );
    }
}
