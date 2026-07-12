//! Python `cmds/업데이트.py`가 호출하던 런타임 데이터 재로딩 로직.
//!
//! 사용자에게 보이는 문구와 분기 순서는 `cmds/업데이트.rhai`에만
//! 둔다. 이 모듈은 Rhai efun이 요청한 설정 캐시 교체와, 현재
//! 명령을 실행하는 동안에는 write lock을 잡을 수 없는 명령
//! 스크립트 재로드 요청만 다룬다.

use crate::data::SharedGlobalData;
use crate::object::Value;
use crate::player::Body;
use crate::script::ScriptStorage;
use rhai::Engine;
use serde_json::Value as JsonValue;
use std::io::{Error as IoError, ErrorKind};

pub(crate) const UPDATE_COMMAND_REQUEST: &str = "_update_command_reload_request";

fn default_config(file: &str) -> Result<JsonValue, String> {
    let path = format!("data/config/{file}.json");
    let source = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&source).map_err(|error| error.to_string())
}

fn reload_global_config(
    global_data: Option<&SharedGlobalData>,
    file: &str,
) -> Result<JsonValue, String> {
    let Some(global_data) = global_data else {
        // Some unit-test/tool engines do not own GlobalData. The Python load()
        // call still reads and parses the file at command time, so preserve
        // that observable failure point even when there is no cache to swap.
        return default_config(file);
    };

    let mut data = global_data.write().map_err(|error| error.to_string())?;
    let loaded = data.reload(file).map_err(|error| error.to_string())?;
    if !loaded {
        return Err(IoError::new(ErrorKind::NotFound, format!("{file}.json")).to_string());
    }
    data.get_clone(file)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, format!("{file}.json")).to_string())
}

fn require_section(root: &JsonValue, section: &str) -> Result<(), String> {
    if root.get(section).is_some() {
        Ok(())
    } else {
        Err(format!("missing field `{section}`"))
    }
}

fn reload_runtime_data(
    body: &mut Body,
    global_data: Option<&SharedGlobalData>,
    target: &str,
) -> Result<(), String> {
    match target {
        // ScriptStorage is read-locked while a Rhai command is executing.
        // Record the request and let create_script_command apply it immediately
        // after execution, before returning the success text to the client.
        "명령어" => {
            body.temp_mut().insert(
                UPDATE_COMMAND_REQUEST.to_string(),
                Value::String(target.to_string()),
            );
            Ok(())
        }
        "무림별호" => {
            let root = reload_global_config(global_data, "nickname")?;
            require_section(&root, "무림별호")?;
            if crate::world::nickname::nickname_reload() {
                Ok(())
            } else {
                Err("nickname.json".to_string())
            }
        }
        "도움말" => {
            let root = reload_global_config(global_data, "help")?;
            require_section(&root, "도움말")
        }
        "무공" => {
            let root = reload_global_config(global_data, "skill")?;
            if !root.is_object() {
                return Err("skill.json".to_string());
            }
            crate::world::skill::reload_skill_cache()?;
            crate::data::reload_skill_defense_head_cache()?;
            Ok(())
        }
        "표현" => {
            crate::emotion::reload_emotion_map()?;
            let root = reload_global_config(global_data, "emotion")?;
            require_section(&root, "감정표현")
        }
        "도우미" => {
            let root = reload_global_config(global_data, "doumi")?;
            require_section(&root, "도우미메인설정")
        }
        "메인설정" => {
            let root = reload_global_config(global_data, "murim")?;
            require_section(&root, "메인설정")
        }
        "스크립트" => {
            let root = reload_global_config(global_data, "script")?;
            require_section(&root, "메인설정")
        }
        // `cmds/업데이트.py` has no final else branch. Rhai never calls
        // the efun for an unknown word, but keeping this a no-op matches it.
        _ => Ok(()),
    }
}

/// Register the data-only efun used by `cmds/업데이트.rhai`.
pub(crate) fn register_update_efun(
    engine: &mut Engine,
    body_ptr: *mut Body,
    global_data: Option<SharedGlobalData>,
) {
    engine.register_fn(
        "reload_runtime_data",
        move |_ob: &mut rhai::Map, target: &str| -> bool {
            let body = unsafe { &mut *body_ptr };
            match reload_runtime_data(body, global_data.as_ref(), target) {
                Ok(()) => true,
                Err(error) => {
                    tracing::error!(target, %error, "Python-compatible runtime reload failed");
                    false
                }
            }
        },
    );
}

/// Apply the deferred `init_commands()` counterpart after the executing Rhai
/// script has released ScriptStorage's read lock.
#[allow(dead_code)]
pub(crate) fn apply_command_reload(storage: &mut ScriptStorage) -> Result<(), String> {
    // Python does not clear Player.cmdList first: it recompiles every command
    // file and overwrites/adds entries only after that file compiles.
    storage
        .load_all_scripts_checked()
        .map_err(|error| error.to_string())
}
