//! Connection-owned identity controlling several persistent bodies.
//!
//! `Soul` owns formation membership and control state. Character statistics,
//! inventory, combat state, and position remain on each `Body`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_SOUL_BODIES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoulBodyKind {
    Main,
    Alternate,
    Mercenary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulBodyMember {
    pub name: String,
    pub kind: SoulBodyKind,
    #[serde(skip)]
    pub summoned_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Soul {
    pub id: String,
    pub main_body: String,
    pub active_body: String,
    pub members: Vec<SoulBodyMember>,
}

impl Soul {
    pub fn new(id: impl Into<String>, main_body: impl Into<String>) -> Self {
        let id = id.into();
        let main_body = main_body.into();
        Self {
            id,
            active_body: main_body.clone(),
            members: vec![SoulBodyMember {
                name: main_body.clone(),
                kind: SoulBodyKind::Main,
                summoned_id: None,
            }],
            main_body,
        }
    }

    pub fn load_or_new(main_body: &str) -> Self {
        let path = Self::path_for(main_body);
        let mut soul = std::fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str::<Self>(&json).ok())
            .filter(|soul| soul.main_body == main_body)
            .unwrap_or_else(|| Self::new(main_body, main_body));
        soul.normalize();
        // Login authentication is performed against the main character. The
        // roster is restored, but control always starts in that authenticated
        // body; a later switch is explicit.
        soul.active_body = soul.main_body.clone();
        soul
    }

    pub fn save(&self) -> bool {
        let path = Self::path_for(&self.main_body);
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        serde_json::to_string_pretty(self)
            .ok()
            .is_some_and(|json| std::fs::write(path, json).is_ok())
    }

    pub fn path_for(main_body: &str) -> PathBuf {
        Path::new("data/soul").join(format!("{main_body}.json"))
    }

    pub fn active_member(&self) -> Option<&SoulBodyMember> {
        self.members
            .iter()
            .find(|member| member.name == self.active_body)
    }

    pub fn inactive_names(&self) -> Vec<String> {
        self.members
            .iter()
            .filter(|member| member.name != self.active_body)
            .map(|member| member.name.clone())
            .collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.members.iter().any(|member| member.name == name)
    }

    pub fn add_body(&mut self, name: impl Into<String>, kind: SoulBodyKind) -> Result<(), String> {
        let name = name.into();
        if name.is_empty() || self.contains(&name) {
            return Err("already_member".into());
        }
        if self.members.len() >= MAX_SOUL_BODIES {
            return Err("formation_full".into());
        }
        self.members.push(SoulBodyMember {
            name,
            kind,
            summoned_id: None,
        });
        Ok(())
    }

    pub fn set_summoned_id(&mut self, name: &str, id: Option<u64>) {
        if let Some(member) = self.members.iter_mut().find(|member| member.name == name) {
            member.summoned_id = id;
        }
    }

    pub fn switch_active(
        &mut self,
        target: &str,
        old_active_summoned_id: u64,
    ) -> Result<(), String> {
        if target == self.active_body || !self.contains(target) {
            return Err("invalid_target".into());
        }
        let previous = self.active_body.clone();
        self.set_summoned_id(&previous, Some(old_active_summoned_id));
        self.set_summoned_id(target, None);
        self.active_body = target.to_string();
        Ok(())
    }

    fn normalize(&mut self) {
        self.members.retain(|member| !member.name.trim().is_empty());
        let mut seen = std::collections::HashSet::new();
        self.members
            .retain(|member| seen.insert(member.name.clone()));
        if !self
            .members
            .iter()
            .any(|member| member.name == self.main_body)
        {
            self.members.insert(
                0,
                SoulBodyMember {
                    name: self.main_body.clone(),
                    kind: SoulBodyKind::Main,
                    summoned_id: None,
                },
            );
        }
        self.members.truncate(MAX_SOUL_BODIES);
        for member in &mut self.members {
            member.summoned_id = None;
            if member.name == self.main_body {
                member.kind = SoulBodyKind::Main;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Value;
    use crate::player::Body;

    #[test]
    fn supports_direct_and_cyclic_body_switches() {
        let mut soul = Soul::new("account", "일호");
        soul.add_body("이호", SoulBodyKind::Alternate).unwrap();
        soul.add_body("삼호", SoulBodyKind::Mercenary).unwrap();
        soul.switch_active("이호", 11).unwrap();
        assert_eq!(soul.active_body, "이호");
        soul.switch_active("삼호", 12).unwrap();
        assert_eq!(soul.active_body, "삼호");
        soul.switch_active("일호", 13).unwrap();
        assert_eq!(soul.active_body, "일호");
        soul.switch_active("삼호", 14).unwrap();
        assert_eq!(soul.active_body, "삼호");
    }

    #[test]
    fn limits_one_soul_to_main_plus_three_auxiliaries() {
        let mut soul = Soul::new("account", "일호");
        for name in ["이호", "삼호", "사호"] {
            soul.add_body(name, SoulBodyKind::Alternate).unwrap();
        }
        assert_eq!(
            soul.add_body("오호", SoulBodyKind::Alternate),
            Err("formation_full".into())
        );
    }

    #[test]
    fn 전환_rhai_selects_a_character_by_number() {
        let mut body = Body::new();
        body.set("이름", "일호");
        body.temp_mut().insert(
            "_soul_roster".into(),
            Value::String(
                serde_json::json!({
                    "id": "일호", "main": "일호", "active": "일호",
                    "members": [
                        {"number": 1, "name": "일호", "kind": "main", "active": true},
                        {"number": 2, "name": "이호", "kind": "alternate", "active": false}
                    ]
                })
                .to_string(),
            ),
        );
        let storage = crate::script::ScriptStorage::default();
        let (output, _) = storage
            .execute("전환", &mut body, "2", None, None, None)
            .unwrap();
        assert_eq!(
            output,
            vec!["조종 대상을 이호 캐릭터로 전환합니다.".to_string()]
        );
        assert_eq!(
            crate::script::take_soul_switch_request(&mut body).as_deref(),
            Some("이호")
        );
    }
}
