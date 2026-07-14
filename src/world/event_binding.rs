//! Reusable event-script bindings for world entities.
//!
//! This module is deliberately execution-agnostic. It describes which script
//! is bound to a trigger; the caller decides when a trigger fires and which
//! actor/target context is passed to Rhai.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

/// A legacy directive list or a hot-reloadable Rhai script reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventScript {
    Legacy(Vec<String>),
    Rhai(String),
}

/// Trigger name to script binding shared by mobs, rooms, items, and fixtures.
///
/// New data can use an `events`/`이벤트스크립트` object with arbitrary trigger
/// names (`push`, `use`, `enter`, ...). Existing mob-style top-level keys that
/// begin with `이벤트` remain readable for migration compatibility.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventBindings(HashMap<String, EventScript>);

impl EventBindings {
    pub fn from_attributes(attributes: &HashMap<String, JsonValue>) -> Self {
        let mut bindings = Self::default();
        for (key, value) in attributes {
            if key.starts_with("이벤트") && key != "이벤트스크립트" {
                if let Some(script) = parse_script(value) {
                    bindings.insert(key.clone(), script);
                }
            }
        }
        for container in ["events", "이벤트스크립트"] {
            let Some(values) = attributes.get(container).and_then(JsonValue::as_object) else {
                continue;
            };
            for (trigger, value) in values {
                if let Some(script) = parse_script(value) {
                    bindings.insert(trigger.clone(), script);
                }
            }
        }
        bindings
    }

    pub fn from_json_map(attributes: &serde_json::Map<String, JsonValue>) -> Self {
        Self::from_attributes(
            &attributes
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    }
}

fn parse_script(value: &JsonValue) -> Option<EventScript> {
    match value {
        JsonValue::String(path) => Some(EventScript::Rhai(path.clone())),
        JsonValue::Array(lines) => Some(EventScript::Legacy(
            lines
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect(),
        )),
        _ => None,
    }
}

impl Deref for EventBindings {
    type Target = HashMap<String, EventScript>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EventBindings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a EventBindings {
    type Item = (&'a String, &'a EventScript);
    type IntoIter = std::collections::hash_map::Iter<'a, String, EventScript>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_accept_new_nested_and_legacy_event_shapes() {
        let attributes = HashMap::from([
            (
                "events".into(),
                serde_json::json!({"push": "bookshelf_push.rhai", "pull": ["$출력 당긴다"]}),
            ),
            ("이벤트 $대화".into(), JsonValue::String("talk.rhai".into())),
        ]);

        let bindings = EventBindings::from_attributes(&attributes);
        assert_eq!(
            bindings.get("push"),
            Some(&EventScript::Rhai("bookshelf_push.rhai".into()))
        );
        assert!(matches!(bindings.get("pull"), Some(EventScript::Legacy(_))));
        assert!(bindings.contains_key("이벤트 $대화"));
    }
}
