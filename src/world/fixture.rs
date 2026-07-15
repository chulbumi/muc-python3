//! Room-bound interactive objects.
//!
//! A fixture has only structural identity and placement in Rust.  Game rules,
//! visible text, and per-kind interaction semantics remain data-driven so Rhai
//! scripts can be hot-reloaded without changing the driver.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::event_binding::EventBindings;

/// Broad fixture classification.  Detailed behavior belongs in attributes and
/// Rhai scripts instead of being hard-coded into these variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureKind {
    Fixture,
    Door,
    Mechanism,
    Trap,
    Container,
    Portal,
    Facility,
}

/// Static fixture declaration embedded in a room JSON document.
///
/// `key` is stable within one room and prevents duplicate placement when the
/// room is entered repeatedly. All game-facing fields remain in `attributes`.
#[derive(Debug, Clone, PartialEq)]
pub struct FixturePlacement {
    pub key: String,
    pub kind: FixtureKind,
    pub attributes: HashMap<String, JsonValue>,
}

impl FixtureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixture => "fixture",
            Self::Door => "door",
            Self::Mechanism => "mechanism",
            Self::Trap => "trap",
            Self::Container => "container",
            Self::Portal => "portal",
            Self::Facility => "facility",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fixture" => Some(Self::Fixture),
            "door" => Some(Self::Door),
            "mechanism" => Some(Self::Mechanism),
            "trap" => Some(Self::Trap),
            "container" => Some(Self::Container),
            "portal" => Some(Self::Portal),
            "facility" => Some(Self::Facility),
            _ => None,
        }
    }
}

/// A fixed or placed room object which can be inspected or interacted with but
/// is not ordinary player inventory.
///
/// Extensible game properties such as `name`, `owner`, `deployable`, `hidden`,
/// `state`, and available actions live in `attributes`.  This keeps the driver
/// independent from rules that have not yet been designed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    pub id: u64,
    pub kind: FixtureKind,
    pub zone: String,
    pub room: String,
    /// Optional scripts for interaction triggers such as push, pull, or use.
    #[serde(default)]
    pub events: EventBindings,
    #[serde(default)]
    pub attributes: HashMap<String, JsonValue>,
}

impl Fixture {
    pub fn new(
        id: u64,
        kind: FixtureKind,
        zone: impl Into<String>,
        room: impl Into<String>,
        attributes: HashMap<String, JsonValue>,
    ) -> Self {
        let events = EventBindings::from_attributes(&attributes);
        Self {
            id,
            kind,
            zone: zone.into(),
            room: room.into(),
            events,
            attributes,
        }
    }

    pub fn name(&self) -> &str {
        self.attributes
            .get("name")
            .or_else(|| self.attributes.get("이름"))
            .and_then(JsonValue::as_str)
            .unwrap_or("")
    }

    pub fn is_hidden(&self) -> bool {
        self.attributes
            .get("hidden")
            .or_else(|| self.attributes.get("숨김"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false)
    }

    pub fn attribute(&self, key: &str) -> Option<&JsonValue> {
        self.attributes.get(key)
    }

    pub fn set_attribute(&mut self, key: impl Into<String>, value: JsonValue) {
        self.attributes.insert(key.into(), value);
    }

    /// Python-style exact/alias-prefix match counts used by the unified room
    /// object selector. Hidden fixtures do not participate until revealed.
    pub fn match_counts(&self, query: &str) -> (bool, usize) {
        if self.is_hidden() {
            return (false, 0);
        }
        let aliases = self
            .attribute("reaction_names")
            .or_else(|| self.attribute("반응이름"));
        let aliases = match aliases {
            Some(JsonValue::Array(values)) => values
                .iter()
                .filter_map(JsonValue::as_str)
                .collect::<Vec<_>>(),
            Some(JsonValue::String(value)) => value
                .split(['\r', '\n'])
                .filter(|alias| !alias.is_empty())
                .collect(),
            _ => Vec::new(),
        };
        let exact = self.name() == query || aliases.iter().any(|alias| *alias == query);
        let prefixes = if exact {
            0
        } else {
            aliases
                .iter()
                .filter(|alias| alias.starts_with(query))
                .count()
        };
        (exact, prefixes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_keeps_extensible_properties_in_attribute_map() {
        let fixture = Fixture::new(
            7,
            FixtureKind::Mechanism,
            "시험존",
            "1",
            HashMap::from([
                ("name".into(), JsonValue::String("숨은 레버".into())),
                ("hidden".into(), JsonValue::Bool(true)),
                ("deployable".into(), JsonValue::Bool(false)),
            ]),
        );

        assert_eq!(fixture.name(), "숨은 레버");
        assert!(fixture.is_hidden());
        assert_eq!(fixture.kind.as_str(), "mechanism");
    }
}
