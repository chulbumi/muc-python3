//! Data-only Rhai efuns for room fixtures.
//!
//! These functions intentionally emit no user-visible text. Commands and room
//! scripts decide how a fixture is presented and what an interaction means.

use rhai::{Dynamic, Engine, Map};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::player::Body;
use crate::world::{get_world_state, EventBindings, EventScript, Fixture, FixtureKind};

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

fn fixture_to_dynamic(fixture: &Fixture) -> Dynamic {
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
    snapshot.insert("kind".into(), Dynamic::from(fixture.kind.as_str()));
    snapshot.insert("zone".into(), Dynamic::from(fixture.zone.clone()));
    snapshot.insert("room".into(), Dynamic::from(fixture.room.clone()));
    snapshot.insert("events".into(), event_bindings_to_dynamic(&fixture.events));
    Dynamic::from(snapshot)
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
    use crate::world::{PlayerPosition, RoomObjectRef};
    use std::sync::{Arc, Mutex};

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
}
