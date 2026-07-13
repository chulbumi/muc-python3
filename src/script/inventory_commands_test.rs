use super::*;
#[test]
fn test_inventory_keeps_python_hidden_only_and_target_failure_behavior() {
    let storage = ScriptStorage::default();
    let mut viewer = Body::new();
    viewer.set("이름", "관리자");
    viewer.set("관리자등급", 1000i64);

    let mut target = Body::new();
    target.set("이름", "대상");
    target
        .object
        .objs
        .push(inventory_test_item("비밀패", false, true));
    set_precomputed_room_inventories(vec![build_room_player_inventory_snapshot(&target)]);
    let (output, _) = storage
        .execute("소지품", &mut viewer, "대상", None, None, None)
        .unwrap();
    assert!(output.contains(&"\x1b[36m☞ 아무것도 없습니다.\x1b[37m".to_string()));
    assert!(!output.iter().any(|line| line.contains("비밀패")));

    let (output, _) = storage
        .execute("소지품", &mut viewer, "없는사람", None, None, None)
        .unwrap();
    clear_precomputed_all_online();
    assert_eq!(output, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);
}
#[test]
fn inventory_views_stop_at_matching_floor_item_before_player() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let viewer_name = format!("소지충돌관리자-{suffix}");
    let target_name = format!("소지충돌대상-{suffix}");
    let zone = format!("소지충돌존-{suffix}");
    let room = "1";
    let mut viewer = Body::new();
    viewer.set("이름", viewer_name.as_str());
    viewer.set("관리자등급", 1000_i64);
    let mut target = Body::new();
    target.set("이름", target_name.as_str());
    target.set("반응이름", "공통소지별칭");
    target.set("은전", 77_i64);
    set_precomputed_room_inventories(vec![build_room_player_inventory_snapshot(&target)]);

    let mut item = Object::new();
    item.set("이름", "소지충돌패");
    item.set("반응이름", "공통소지별칭");
    let item = Arc::new(Mutex::new(item));
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&viewer_name, PlayerPosition::new(zone.clone(), room.into()));
        world.set_player_position(&target_name, PlayerPosition::new(zone.clone(), room.into()));
        world.get_room_objs_mut(&zone, room).push(item.clone());
        world.record_floor_item(&zone, room, &item);
    }

    let storage = ScriptStorage::default();
    for command in ["소소", "소지품"] {
        let output = storage
            .execute(command, &mut viewer, "공통소지별칭", None, None, None)
            .unwrap();
        assert_eq!(
            output.0,
            vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"],
            "{command}"
        );
    }

    set_precomputed_room_inventories(Vec::new());
    let mut world = get_world_state().write().unwrap();
    world.remove_floor_item_record(&zone, room, &item);
    world.get_room_objs_mut(&zone, room).clear();
    world.remove_player_position(&viewer_name);
    world.remove_player_position(&target_name);
}
#[test]
fn compact_inventory_non_admin_ignores_target_and_normalizes_empty_viewer_gold() {
    let mut viewer = Body::new();
    viewer.set("이름", "소소일반인");
    viewer.set("관리자등급", 0_i64);
    viewer.set("은전", 12_i64);
    viewer.set("금전", "");
    viewer
        .object
        .objs
        .push(inventory_test_item("본인약초", false, false));

    let inventory = ScriptStorage::default()
        .execute("소지품", &mut viewer, "없는다른사람", None, None, None)
        .unwrap()
        .0;
    assert!(inventory.contains(&"\x1b[36m본인약초\x1b[37m".to_string()));
    assert_eq!(viewer.object.attr.get("금전"), Some(&Value::Int(0)));
    viewer.set("금전", "");

    let output = ScriptStorage::default()
        .execute("소소", &mut viewer, "없는다른사람", None, None, None)
        .unwrap()
        .0;
    assert!(output.contains(&"\x1b[36m본인약초\x1b[37m".to_string()));
    assert!(output
        .iter()
        .any(|line| line.contains("은전 :                   12 개")));
    assert!(!output.iter().any(|line| line.contains("▶ 금전")));
    assert_eq!(viewer.object.attr.get("금전"), Some(&Value::Int(0)));
}
#[test]
fn compact_inventory_uses_admin_target_but_python_viewer_gold() {
    let storage = ScriptStorage::default();
    let mut viewer = Body::new();
    viewer.set("이름", "소소관리자");
    viewer.set("관리자등급", 1000_i64);
    viewer.set("금전", 33_i64);
    let mut target = Body::new();
    target.set("이름", "소소대상");
    target.set("은전", 7_i64);
    target.set("금전", 99_i64);
    target
        .object
        .objs
        .push(inventory_test_item("약초", false, false));
    target
        .object
        .objs
        .push(inventory_test_item("약초", false, false));
    set_precomputed_room_inventories(vec![build_room_player_inventory_snapshot(&target)]);

    let output = storage
        .execute("소소", &mut viewer, "소소대상", None, None, None)
        .unwrap()
        .0;
    clear_precomputed_all_online();
    assert!(output.contains(&"\x1b[36m약초 \x1b[36m2개\x1b[37m".to_string()));
    assert!(output
        .iter()
        .any(|line| line.contains("은전 :                    7 개")));
    assert!(output
        .iter()
        .any(|line| line.contains("금전 :                   33 개")));
    assert!(!output
        .iter()
        .any(|line| line.contains("금전 :                   99 개")));
    assert_eq!(output.last().unwrap(), "─────────────────");
}
#[test]
fn test_inventory_admin_views_same_room_target_like_python() {
    let storage = ScriptStorage::default();
    let mut viewer = Body::new();
    viewer.set("이름", "관리자");
    viewer.set("관리자등급", 1000i64);

    let mut target = Body::new();
    target.set("이름", "대상");
    target.set("은전", 7i64);
    target.set("금전", 9i64);
    target
        .object
        .objs
        .push(inventory_test_item("약초", false, false));
    target
        .object
        .objs
        .push(inventory_test_item("비밀패", false, true));
    target
        .object
        .objs
        .push(inventory_test_item("철검", true, false));
    target
        .object
        .objs
        .push(inventory_test_item("약초", false, false));
    set_precomputed_room_inventories(vec![build_room_player_inventory_snapshot(&target)]);

    let (output, special) = storage
        .execute("소지품", &mut viewer, "대상", None, None, None)
        .unwrap();
    clear_precomputed_all_online();

    assert!(special.is_none());
    assert_eq!(
        output,
        vec![
            "━━━━━━━━━━━━━━━━━".to_string(),
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m  ◁     소     지     품     ▷  \x1b[0m\x1b[37m\x1b[40m"
                .to_string(),
            "─────────────────".to_string(),
            "\x1b[36m약초 \x1b[36m2개\x1b[37m".to_string(),
            "\x1b[36m비밀패\x1b[37m".to_string(),
            "─────────────────".to_string(),
            format!(
                "\x1b[0m\x1b[47m\x1b[30m▶ 은전 : {:>20} 개 \x1b[0m\x1b[37m\x1b[40m",
                7
            ),
            format!(
                "\x1b[0m\x1b[43m\x1b[30m▶ 금전 : {:>20} 개 \x1b[0m\x1b[37m\x1b[40m",
                9
            ),
            "─────────────────\x1b[0;37m".to_string(),
        ]
    );
}
fn inventory_test_item(name: &str, in_use: bool, hidden: bool) -> Arc<Mutex<Object>> {
    let item = Arc::new(Mutex::new(Object::new()));
    {
        let mut item = item.lock().unwrap();
        item.set("이름", name);
        item.set("inUse", i64::from(in_use));
        if hidden {
            item.set("아이템속성", "출력안함");
        }
    }
    item
}
