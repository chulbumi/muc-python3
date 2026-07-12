//! 무림별호 전역 레지스트리.
//!
//! Python `objs/nickname.py`의 `NICKNAME.attr`와 같은 역할을 하며,
//! `data/config/nickname.json`의 `{ "무림별호": { 별호: 사용자이름 } }`
//! 구조를 그대로 읽고 쓴다.

use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

const DEFAULT_PATH: &str = "data/config/nickname.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NicknameError {
    AlreadyExists,
    SaveFailed,
}

/// 별호와 소유자 이름은 모두 문자열 속성으로 관리한다.
#[derive(Debug)]
pub struct NicknameRegistry {
    path: PathBuf,
    pub attr: HashMap<String, String>,
}

impl Default for NicknameRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NicknameRegistry {
    pub fn new() -> Self {
        Self::with_path(DEFAULT_PATH)
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            attr: HashMap::new(),
        }
    }

    pub fn load(&mut self) -> bool {
        let Ok(source) = std::fs::read_to_string(&self.path) else {
            return false;
        };
        let Ok(root) = serde_json::from_str::<JsonValue>(&source) else {
            return false;
        };
        let Some(entries) = root.get("무림별호").and_then(JsonValue::as_object) else {
            return false;
        };

        self.attr = entries
            .iter()
            .filter_map(|(nickname, owner)| {
                owner
                    .as_str()
                    .map(|owner| (nickname.clone(), owner.to_string()))
            })
            .collect();
        true
    }

    pub fn save(&self) -> bool {
        let Some(parent) = self.path.parent() else {
            return false;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }

        let entries: Map<String, JsonValue> = self
            .attr
            .iter()
            .map(|(nickname, owner)| (nickname.clone(), JsonValue::String(owner.clone())))
            .collect();
        let root = serde_json::json!({ "무림별호": entries });
        let Ok(source) = serde_json::to_string_pretty(&root) else {
            return false;
        };

        // 같은 디렉토리에 먼저 기록한 뒤 교체해 중간 상태의 JSON 노출을 피한다.
        let temp_path = temporary_path(&self.path);
        if std::fs::write(&temp_path, source).is_err() {
            return false;
        }
        if std::fs::rename(&temp_path, &self.path).is_err() {
            let _ = std::fs::remove_file(&temp_path);
            return false;
        }
        true
    }

    pub fn contains(&self, nickname: &str) -> bool {
        self.attr.contains_key(nickname)
    }

    pub fn owner(&self, nickname: &str) -> String {
        self.attr.get(nickname).cloned().unwrap_or_default()
    }

    /// 중복 확인과 등록을 하나의 write lock 안에서 수행한다.
    pub fn reserve(&mut self, nickname: &str, owner: &str) -> Result<(), NicknameError> {
        if self.attr.contains_key(nickname) {
            return Err(NicknameError::AlreadyExists);
        }

        self.attr.insert(nickname.to_string(), owner.to_string());
        if self.save() {
            Ok(())
        } else {
            // Python은 `NICKNAME[line] = owner` 후 `save()`의 False 반환을
            // 무시하므로, 파일 저장이 실패해도 런타임 레지스트리에는
            // 등록값이 남는다.
            Err(NicknameError::SaveFailed)
        }
    }

    /// 소유자가 일치할 때만 예약을 해제한다. 후속 작업 실패 시 롤백용이다.
    pub fn release(&mut self, nickname: &str, owner: &str) -> bool {
        if self.attr.get(nickname).map(String::as_str) != Some(owner) {
            return false;
        }

        let previous = self.attr.remove(nickname);
        if self.save() {
            true
        } else {
            if let Some(previous) = previous {
                self.attr.insert(nickname.to_string(), previous);
            }
            false
        }
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("nickname.json");
    path.with_file_name(format!(".{file_name}.tmp"))
}

static NICKNAMES: OnceLock<RwLock<NicknameRegistry>> = OnceLock::new();

fn global_registry() -> &'static RwLock<NicknameRegistry> {
    NICKNAMES.get_or_init(|| {
        let mut registry = NicknameRegistry::new();
        let _ = registry.load();
        RwLock::new(registry)
    })
}

pub fn nickname_exists(nickname: &str) -> bool {
    global_registry().read().unwrap().contains(nickname)
}

pub fn nickname_owner(nickname: &str) -> String {
    global_registry().read().unwrap().owner(nickname)
}

/// Rhai용 결과 코드: 성공 `""`, 중복 `"exists"`, 저장 실패 `"save_failed"`.
pub fn nickname_reserve(nickname: &str, owner: &str) -> String {
    match global_registry().write().unwrap().reserve(nickname, owner) {
        Ok(()) => String::new(),
        Err(NicknameError::AlreadyExists) => "exists".to_string(),
        Err(NicknameError::SaveFailed) => "save_failed".to_string(),
    }
}

pub fn nickname_release(nickname: &str, owner: &str) -> bool {
    global_registry().write().unwrap().release(nickname, owner)
}

pub fn nickname_save() -> bool {
    global_registry().read().unwrap().save()
}

/// `NICKNAME.load()` counterpart used by the hot-reloadable update command.
pub fn nickname_reload() -> bool {
    global_registry().write().unwrap().load()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "muc_nickname_{}_{}_{}",
                label,
                std::process::id(),
                nonce
            ))
            .join("nickname.json")
    }

    #[test]
    fn load_reserve_release_round_trip() {
        let path = test_path("round_trip");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"무림별호":{"검성":"기존사용자"}}"#).unwrap();

        let mut registry = NicknameRegistry::with_path(&path);
        assert!(registry.load());
        assert_eq!(registry.owner("검성"), "기존사용자");
        assert_eq!(
            registry.reserve("검성", "다른사용자"),
            Err(NicknameError::AlreadyExists)
        );

        assert_eq!(registry.reserve("천마", "새사용자"), Ok(()));
        let mut reloaded = NicknameRegistry::with_path(&path);
        assert!(reloaded.load());
        assert_eq!(reloaded.owner("천마"), "새사용자");

        assert!(!registry.release("천마", "다른사용자"));
        assert!(registry.release("천마", "새사용자"));
        assert!(!registry.contains("천마"));

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
