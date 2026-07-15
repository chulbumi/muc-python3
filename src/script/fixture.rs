//! Data-only Rhai efuns for room fixtures.
//!
//! These functions intentionally emit no user-visible text. Commands and room
//! scripts decide how a fixture is presented and what an interaction means.

use rhai::{Dynamic, Engine, Map, Scope, AST};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::command::CommandResult;
use crate::player::Body;
use crate::world::{
    get_world_state, EventBindings, EventScript, Fixture, FixtureKind, RoomObjectRef, WorldState,
};

thread_local! {
    /// Rhai AST contains `Rc` without Rhai's sync feature, so fixture event
    /// scripts use the same worker-local caching rule as ordinary commands.
    static FIXTURE_EVENT_AST_CACHE: RefCell<HashMap<PathBuf, (SystemTime, u64, u64, AST)>> =
        RefCell::new(HashMap::new());
}

fn fixture_event_path(zone: &str, configured: &str) -> Option<PathBuf> {
    let relative = Path::new(configured.trim());
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    let filename = if relative.extension().is_some() {
        relative.to_path_buf()
    } else {
        relative.with_extension("rhai")
    };
    Some(Path::new("data/script").join(zone).join(filename))
}

fn cached_fixture_event_ast(path: &Path) -> Result<AST, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("fixture event metadata {}: {error}", path.display()))?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let length = metadata.len();
    #[cfg(unix)]
    let identity = metadata.ino();
    #[cfg(not(unix))]
    let identity = 0;
    if let Some(ast) = FIXTURE_EVENT_AST_CACHE.with(|cache| {
        cache
            .borrow()
            .get(path)
            .filter(|(cached_modified, cached_length, cached_identity, _)| {
                *cached_modified == modified
                    && *cached_length == length
                    && *cached_identity == identity
            })
            .map(|(_, _, _, ast)| ast.clone())
    }) {
        return Ok(ast);
    }

    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("fixture event read {}: {error}", path.display()))?;

    let executable = format!("{source}\nmain(ob, fixture_id, cmdline);");
    let ast = Engine::new()
        .compile(&executable)
        .map_err(|error| format!("fixture event compile {}: {error}", path.display()))?;
    FIXTURE_EVENT_AST_CACHE.with(|cache| {
        cache.borrow_mut().insert(
            path.to_path_buf(),
            (modified, length, identity, ast.clone()),
        );
    });
    Ok(ast)
}

/// Match `[fixture] ... [trigger]` and run the room fixture's hot-reloadable
/// Rhai event. Rust selects data and exposes efuns; all visible game text and
/// state-specific presentation remain in the bound script.
pub(crate) fn try_fixture_event(
    body: &mut Body,
    zone: &str,
    room: &str,
    raw_line: &str,
) -> Option<CommandResult> {
    let words: Vec<&str> = raw_line.split_whitespace().collect();
    if words.len() < 2 {
        return None;
    }
    let query = words[0];
    let trigger = *words.last()?;
    let fixture = {
        let world = get_world_state().read().ok()?;
        world
            .get_room_fixtures(zone, room)
            .into_iter()
            .filter_map(|fixture| {
                let (exact, prefixes) = fixture.match_counts(query);
                (exact || prefixes > 0).then_some((exact, fixture.name().len(), fixture.clone()))
            })
            .max_by_key(|(exact, name_len, _)| (*exact, *name_len))
            .map(|(_, _, fixture)| fixture)?
    };
    let script = fixture
        .events
        .get(trigger)
        .or_else(|| fixture.events.get(&format!("이벤트 ${trigger}")))?;
    let EventScript::Rhai(configured_path) = script else {
        return None;
    };
    let Some(path) = fixture_event_path(zone, configured_path) else {
        return Some(CommandResult::Output(
            "(Fixture 이벤트 경로가 올바르지 않습니다.)".to_string(),
        ));
    };
    let ast = match cached_fixture_event_ast(&path) {
        Ok(ast) => ast,
        Err(error) => return Some(CommandResult::Output(format!("({error})"))),
    };

    let output = Arc::new(Mutex::new(Vec::new()));
    let special = Arc::new(Mutex::new(None));
    let user_sends = Arc::new(Mutex::new(Vec::new()));
    let engine = super::create_engine_with_body_and_output(
        body,
        output.clone(),
        None,
        None,
        special,
        user_sends,
        None,
        Some("fixture_event"),
        None,
    );
    let mut scope = Scope::new();
    let player_data = super::build_ob_from_body(body);
    scope.push("player", player_data.clone());
    scope.push("me", player_data.clone());
    scope.push("ob", player_data);
    scope.push("fixture_id", fixture.id as i64);
    scope.push("cmdline", raw_line.to_string());
    if let Err(error) = engine.run_ast_with_scope(&mut scope, &ast) {
        return Some(CommandResult::Output(format!(
            "(Fixture 이벤트 스크립트 오류: {error})"
        )));
    }
    let output_lines = output.lock().map(|lines| lines.clone()).unwrap_or_default();
    Some(CommandResult::MobEvent {
        output_lines,
        set_position: None,
        broadcast_lines: Vec::new(),
        room_broadcast_lines: Vec::new(),
    })
}

fn dynamic_to_json(value: &Dynamic) -> Option<JsonValue> {
    rhai::serde::from_dynamic(value).ok()
}

fn map_to_attributes(map: &Map) -> Option<HashMap<String, JsonValue>> {
    map.iter()
        .map(|(key, value)| Some((key.to_string(), dynamic_to_json(value)?)))
        .collect()
}

fn event_bindings_to_dynamic(bindings: &EventBindings) -> Dynamic {
    let mut events = Map::new();
    for (trigger, script) in bindings {
        let value = match script {
            EventScript::Legacy(lines) => {
                Dynamic::from_array(lines.iter().cloned().map(Dynamic::from).collect())
            }
            EventScript::Rhai(path) => Dynamic::from(path.clone()),
        };
        events.insert(trigger.as_str().into(), value);
    }
    Dynamic::from(events)
}

pub(super) fn fixture_to_dynamic(fixture: &Fixture) -> Dynamic {
    let mut snapshot = Map::new();
    for (key, value) in &fixture.attributes {
        snapshot.insert(key.as_str().into(), crate::data::json_to_dynamic(value));
    }
    let attributes = JsonValue::Object(
        fixture
            .attributes
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    snapshot.insert(
        "attributes".into(),
        crate::data::json_to_dynamic(&attributes),
    );
    snapshot.insert("id".into(), Dynamic::from(fixture.id as i64));
    if !snapshot.contains_key("name") {
        snapshot.insert("name".into(), Dynamic::from(fixture.name().to_string()));
    }
    if !snapshot.contains_key("종류") {
        snapshot.insert("종류".into(), Dynamic::from(fixture.kind.as_str()));
    }
    if !snapshot.contains_key("설명1") {
        let short = fixture
            .attribute("description1")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        snapshot.insert("설명1".into(), Dynamic::from(short.to_string()));
    }
    if !snapshot.contains_key("설명") {
        let description = fixture
            .attribute("description")
            .map(crate::data::json_to_dynamic)
            .unwrap_or_else(|| Dynamic::from(rhai::Array::new()));
        snapshot.insert("설명".into(), description);
    }
    snapshot.insert("kind".into(), Dynamic::from(fixture.kind.as_str()));
    snapshot.insert("zone".into(), Dynamic::from(fixture.zone.clone()));
    snapshot.insert("room".into(), Dynamic::from(fixture.room.clone()));
    snapshot.insert("events".into(), event_bindings_to_dynamic(&fixture.events));
    Dynamic::from(snapshot)
}

/// Visible short descriptions in Python-compatible room object order. The
/// text itself is fixture data; this function only applies visibility/order.
pub(crate) fn visible_fixture_short_lines(
    world: &WorldState,
    zone: &str,
    room: &str,
) -> Vec<String> {
    let ordered = world.get_room_object_order(zone, room);
    let mut fixture_ids = ordered
        .into_iter()
        .filter_map(|object| match object {
            RoomObjectRef::Fixture(id) => Some(id),
            _ => None,
        })
        .collect::<Vec<_>>();
    if fixture_ids.is_empty() {
        fixture_ids.extend(
            world
                .get_room_fixtures(zone, room)
                .into_iter()
                .map(|fixture| fixture.id),
        );
    }
    fixture_ids
        .into_iter()
        .filter_map(|id| {
            let fixture = world.get_fixture(id)?;
            if fixture.is_hidden() {
                return None;
            }
            fixture
                .attribute("설명1")
                .or_else(|| fixture.attribute("description1"))
                .and_then(JsonValue::as_str)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .or_else(|| (!fixture.name().is_empty()).then(|| fixture.name().to_string()))
        })
        .collect()
}

fn fixture_matches(fixture: &Fixture, query: &str) -> bool {
    let (exact, prefixes) = fixture.match_counts(query);
    exact || prefixes > 0
}

pub(super) fn register_fixture_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    engine.register_fn("fixture_kind_valid", |kind: &str| -> bool {
        FixtureKind::parse(kind).is_some()
    });

    let room_events_body = body_ptr;
    engine.register_fn("room_event_bindings", move |_ob: &mut Map| -> Dynamic {
        let body = unsafe { &*room_events_body };
        let Some((zone, room)) = super::current_body_position(body) else {
            return Dynamic::from(Map::new());
        };
        let Ok(world) = get_world_state().read() else {
            return Dynamic::from(Map::new());
        };
        world
            .room_cache
            .get_room_cached(&zone, &room)
            .and_then(|room| {
                room.read()
                    .ok()
                    .map(|room| event_bindings_to_dynamic(&room.events))
            })
            .unwrap_or_else(|| Dynamic::from(Map::new()))
    });

    engine.register_fn("item_event_bindings", |item_key: &str| -> Dynamic {
        get_world_state()
            .read()
            .ok()
            .and_then(|world| {
                world
                    .item_cache
                    .get_item(item_key)
                    .map(|item| event_bindings_to_dynamic(&item.events))
            })
            .unwrap_or_else(|| Dynamic::from(Map::new()))
    });

    let create_body = body_ptr;
    engine.register_fn(
        "fixture_create",
        move |_ob: &mut Map, kind: &str, attributes: Map| -> i64 {
            let Some(kind) = FixtureKind::parse(kind) else {
                return 0;
            };
            let Some(attributes) = map_to_attributes(&attributes) else {
                return 0;
            };
            let body = unsafe { &*create_body };
            let Some((zone, room)) = super::current_body_position(body) else {
                return 0;
            };
            get_world_state()
                .write()
                .ok()
                .map(|mut world| world.create_fixture(&zone, &room, kind, attributes) as i64)
                .unwrap_or(0)
        },
    );

    engine.register_fn("fixture_get", |id: i64| -> Dynamic {
        let Ok(id) = u64::try_from(id) else {
            return Dynamic::UNIT;
        };
        get_world_state()
            .read()
            .ok()
            .and_then(|world| world.get_fixture(id).map(fixture_to_dynamic))
            .unwrap_or(Dynamic::UNIT)
    });

    let list_body = body_ptr;
    engine.register_fn("fixture_list", move |_ob: &mut Map| -> rhai::Array {
        let body = unsafe { &*list_body };
        let Some((zone, room)) = super::current_body_position(body) else {
            return Vec::new();
        };
        get_world_state()
            .read()
            .ok()
            .map(|world| {
                world
                    .get_room_fixtures(&zone, &room)
                    .into_iter()
                    .map(fixture_to_dynamic)
                    .collect()
            })
            .unwrap_or_default()
    });

    let find_body = body_ptr;
    engine.register_fn(
        "fixture_find",
        move |_ob: &mut Map, query: &str| -> Dynamic {
            let body = unsafe { &*find_body };
            let Some((zone, room)) = super::current_body_position(body) else {
                return Dynamic::UNIT;
            };
            get_world_state()
                .read()
                .ok()
                .and_then(|world| {
                    world
                        .get_room_fixtures(&zone, &room)
                        .into_iter()
                        .find(|fixture| fixture_matches(fixture, query))
                        .map(fixture_to_dynamic)
                })
                .unwrap_or(Dynamic::UNIT)
        },
    );

    engine.register_fn("fixture_get_attr", |id: i64, key: &str| -> Dynamic {
        let Ok(id) = u64::try_from(id) else {
            return Dynamic::UNIT;
        };
        get_world_state()
            .read()
            .ok()
            .and_then(|world| {
                world
                    .get_fixture(id)
                    .and_then(|fixture| fixture.attribute(key))
                    .map(crate::data::json_to_dynamic)
            })
            .unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn(
        "fixture_set_attr",
        |id: i64, key: &str, value: Dynamic| -> bool {
            let Ok(id) = u64::try_from(id) else {
                return false;
            };
            let Some(value) = dynamic_to_json(&value) else {
                return false;
            };
            get_world_state()
                .write()
                .ok()
                .and_then(|mut world| {
                    let fixture = world.get_fixture_mut(id)?;
                    fixture.set_attribute(key, value);
                    Some(true)
                })
                .unwrap_or(false)
        },
    );

    engine.register_fn("fixture_move", |id: i64, zone: &str, room: &str| -> bool {
        let Ok(id) = u64::try_from(id) else {
            return false;
        };
        get_world_state()
            .write()
            .ok()
            .map(|mut world| world.move_fixture(id, zone, room))
            .unwrap_or(false)
    });

    engine.register_fn("fixture_remove", |id: i64| -> bool {
        let Ok(id) = u64::try_from(id) else {
            return false;
        };
        get_world_state()
            .write()
            .ok()
            .and_then(|mut world| world.remove_fixture(id))
            .is_some()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Body;
    use crate::script::{build_room_lines, ScriptStorage};
    use crate::world::{PlayerPosition, RoomObjectRef};
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fixture_event_ast_reloads_after_atomic_runtime_edit() {
        let path = std::env::temp_dir().join(format!(
            "fixture-hot-reload-{}-{}.rhai",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, "fn main(ob, fixture_id, cmdline) { \"A\" }").unwrap();
        let first = cached_fixture_event_ast(&path).unwrap();
        let engine = Engine::new();
        let mut scope = Scope::new();
        scope.push("ob", Map::new());
        scope.push("fixture_id", 1_i64);
        scope.push("cmdline", "밀어");
        assert_eq!(
            engine
                .eval_ast_with_scope::<String>(&mut scope, &first)
                .unwrap(),
            "A"
        );

        let replacement = path.with_extension("replacement");
        std::fs::write(&replacement, "fn main(ob, fixture_id, cmdline) { \"B\" }").unwrap();
        std::fs::rename(replacement, &path).unwrap();
        let second = cached_fixture_event_ast(&path).unwrap();
        assert_eq!(
            engine
                .eval_ast_with_scope::<String>(&mut scope, &second)
                .unwrap(),
            "B"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn fixture_efuns_create_mutate_move_and_remove_without_output() {
        let mut body = Body::new();
        let name = format!("fixture-test-{}", std::process::id());
        body.set("이름", name.clone());
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(&name, PlayerPosition::new("시험존".into(), "1".into()));
        }

        let output = Arc::new(Mutex::new(Vec::new()));
        let special = Arc::new(Mutex::new(None));
        let sends = Arc::new(Mutex::new(Vec::new()));
        let engine = super::super::create_engine_with_body_and_output(
            &mut body,
            output.clone(),
            None,
            None,
            special,
            sends,
            None,
            Some("fixture_test"),
            None,
        );
        let mut scope = rhai::Scope::new();
        scope.push("ob", Map::new());
        let id = engine
            .eval_with_scope::<i64>(
                &mut scope,
                r#"
                    let id = fixture_create(ob, "mechanism", #{
                        name: "시험 레버",
                        hidden: true,
                        deployable: false,
                        state: #{ active: false },
                        events: #{ push: "lever_push.rhai" }
                    });
                    fixture_set_attr(id, "owner", "테스터");
                    id
                "#,
            )
            .unwrap();
        assert!(id > 0);
        assert!(output.lock().unwrap().is_empty());

        {
            let world = get_world_state().read().unwrap();
            let fixture = world.get_fixture(id as u64).unwrap();
            assert_eq!(fixture.kind, FixtureKind::Mechanism);
            assert_eq!(
                fixture.events.get("push"),
                Some(&EventScript::Rhai("lever_push.rhai".into()))
            );
            assert_eq!(
                fixture.attribute("owner"),
                Some(&JsonValue::String("테스터".into()))
            );
            assert_eq!(
                world.get_room_object_order("시험존", "1").first(),
                Some(&RoomObjectRef::Fixture(id as u64))
            );
        }

        assert!(engine
            .eval_with_scope::<bool>(
                &mut scope,
                &format!("fixture_move({id}, \"시험존\", \"2\")")
            )
            .unwrap());
        assert!(engine
            .eval_with_scope::<bool>(&mut scope, &format!("fixture_remove({id})"))
            .unwrap());
        let mut world = get_world_state().write().unwrap();
        assert!(world.get_fixture(id as u64).is_none());
        world.remove_player_position(&name);
    }

    #[test]
    fn mangmang_room_fixture_is_placed_once_and_triggers_rhai_event() {
        let mut body = Body::new();
        let player_name = format!("망망-fixture-event-test-{}", std::process::id());
        body.set("이름", player_name.clone());

        let fixture_id = {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new("사용자맵".into(), "망망".into()),
            );
            world.spawn_mobs_for_room("사용자맵", "망망");
            world.spawn_mobs_for_room("사용자맵", "망망");
            let fixtures = world.get_room_fixtures("사용자맵", "망망");
            let placed: Vec<_> = fixtures
                .into_iter()
                .filter(|fixture| {
                    fixture
                        .attribute("placement_key")
                        .and_then(JsonValue::as_str)
                        == Some("청룡병풍")
                })
                .collect();
            assert_eq!(placed.len(), 1, "re-entering must not duplicate fixtures");
            placed[0].id
        };

        let room_view = build_room_lines(&player_name, &[]).expect("room view");
        let fixture_offset = room_view.find("한쪽 벽에는 푸른 용").unwrap();
        let exit_offset = room_view.find("쪽으로 이동할 수 있습니다").unwrap();
        assert!(fixture_offset < exit_offset);

        let look_output = ScriptStorage::default()
            .execute("봐", &mut body, "병풍", None, None, None)
            .expect("fixture detail look")
            .0;
        assert!(look_output.iter().any(|line| line.contains("◆ 이름 ▷")));
        assert!(look_output
            .iter()
            .any(|line| line.contains("바닥의 희미한 자국")));

        {
            let mut world = get_world_state().write().unwrap();
            world
                .get_fixture_mut(fixture_id)
                .unwrap()
                .set_attribute("hidden", JsonValue::Bool(true));
        }
        assert!(!build_room_lines(&player_name, &[])
            .expect("hidden room view")
            .contains("한쪽 벽에는 푸른 용"));
        let hidden_look = ScriptStorage::default()
            .execute("봐", &mut body, "병풍", None, None, None)
            .expect("hidden fixture look")
            .0;
        assert!(hidden_look.iter().any(|line| line.contains("안광으로는")));
        get_world_state()
            .write()
            .unwrap()
            .get_fixture_mut(fixture_id)
            .unwrap()
            .set_attribute("hidden", JsonValue::Bool(false));

        let CommandResult::MobEvent { output_lines, .. } =
            try_fixture_event(&mut body, "사용자맵", "망망", "병풍 밀어")
                .expect("fixture event should match")
        else {
            panic!("fixture event should return event output");
        };
        assert_eq!(output_lines.len(), 3);
        assert!(output_lines[1].contains("숨겨진 틈"));
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_fixture(fixture_id)
                .and_then(|fixture| fixture.attribute("moved")),
            Some(&JsonValue::Bool(true))
        );

        let CommandResult::MobEvent { output_lines, .. } =
            try_fixture_event(&mut body, "사용자맵", "망망", "청룡병풍 밀어")
                .expect("fixture alias should match")
        else {
            panic!("repeated fixture event should return event output");
        };
        assert_eq!(output_lines.len(), 1);
        assert!(output_lines[0].contains("이미 옮겨진 병풍"));

        let mut world = get_world_state().write().unwrap();
        world.remove_fixture(fixture_id);
        world.remove_player_position(&player_name);
    }
}
