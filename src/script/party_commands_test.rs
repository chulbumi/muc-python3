use super::*;
use crate::network::social::RelationState;
use crate::script::party::clear_precomputed_party_context;
use rhai::{Array, Dynamic, Map};

fn empty_party_context(body: &Body) -> Map {
    let actor = build_party_person_snapshot("actor".into(), body, RelationState::default(), 1);
    let mut context = Map::new();
    context.insert("self_id".into(), Dynamic::from("actor"));
    context.insert("self".into(), actor);
    context.insert(
        "follow".into(),
        missing_party_person(String::new(), RelationState::default()),
    );
    context.insert("followers".into(), Dynamic::from(Array::new()));
    context.insert(
        "party_leader".into(),
        missing_party_person(String::new(), RelationState::default()),
    );
    context.insert("party_members".into(), Dynamic::from(Array::new()));
    context.insert("room_players".into(), Dynamic::from(Array::new()));
    context.insert("room_objects".into(), Dynamic::from(Array::new()));
    context
}

#[test]
fn party_empty_state_keeps_python_unreachable_error_and_exact_companion_frame() {
    let mut body = Body::new();
    body.set("이름", "홀로걷는이");
    body.set("체력", 100_i64);
    body.set("최고체력", 100_i64);
    body.set("내공", 10_i64);
    body.set("최고내공", 10_i64);
    set_precomputed_party_context(empty_party_context(&body));
    let result = ScriptStorage::default()
        .execute("무리", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        result.0,
        vec![
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            "◁ \x1b[1m홀로걷는이\x1b[0m\x1b[40m\x1b[37m의 동행 ▷",
            "────────────────────────────",
            "동행중",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        ]
    );
    assert!(result.1.is_none());
    clear_precomputed_party_context();
}

#[test]
fn party_chat_checks_real_room_communication_ban_before_party_state() {
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("무리말금지검사-{suffix}");
    let zone = format!("무리말금지존-{suffix}");
    let room_dir = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&room_dir).unwrap();
    std::fs::write(
        room_dir.join("1.json"),
        serde_json::json!({"맵정보": {
            "이름": "통신금지방", "존이름": zone, "설명": [], "출구": [], "몹": [],
            "맵속성": ["모든통신금지"]
        }})
        .to_string(),
    )
    .unwrap();
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.set_player_position(&name, PlayerPosition::new(zone.clone(), "1".into()));
    }
    let mut body = Body::new();
    body.set("이름", name.as_str());
    set_precomputed_party_context(empty_party_context(&body));
    let output = ScriptStorage::default()
        .execute("무리말", &mut body, "전달불가", None, None, None)
        .unwrap();
    assert_eq!(output.0, vec!["☞ 이지역에서는 어떠한 통신도 불가능합니다."]);
    assert!(take_party_requests(&mut body).1.is_empty());
    clear_precomputed_party_context();
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&name);
    let _ = std::fs::remove_dir_all(room_dir);
}

#[test]
fn party_remove_keeps_python_validation_order_for_none_and_nonleader() {
    let mut actor = Body::new();
    actor.set("이름", "제외검사원");
    let empty = empty_party_context(&actor);
    set_precomputed_party_context(empty.clone());
    assert_eq!(
        ScriptStorage::default()
            .execute("무리제외", &mut actor, "", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 사용법: [동행원|무리원] 제외"]
    );
    set_precomputed_party_context(empty);
    assert_eq!(
        ScriptStorage::default()
            .execute("무리제외", &mut actor, "아무개", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 당신이 속한 무리가 없어요. ^^"]
    );

    let actor_person = build_party_person_snapshot(
        "member".into(),
        &actor,
        RelationState {
            follow: None,
            party_leader: Some("leader".into()),
        },
        1,
    );
    let mut leader_body = Body::new();
    leader_body.set("이름", "제외대장");
    let leader_person = build_party_person_snapshot(
        "leader".into(),
        &leader_body,
        RelationState {
            follow: None,
            party_leader: Some("leader".into()),
        },
        1,
    );
    let mut context = empty_party_context(&actor);
    context.insert("self".into(), actor_person);
    context.insert("self_id".into(), Dynamic::from("member"));
    context.insert("party_leader".into(), leader_person);
    set_precomputed_party_context(context);
    let nonleader = ScriptStorage::default()
        .execute("무리제외", &mut actor, "모두", None, None, None)
        .unwrap();
    assert_eq!(nonleader.0, vec!["☞ 당신을 따르는 무리가 없어요. ^^"]);
    assert!(take_party_requests(&mut actor).0.is_none());
    clear_precomputed_party_context();
}
