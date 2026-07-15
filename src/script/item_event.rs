//! Hot-reloadable inventory-item events.
//!
//! Rust owns target identity and data mutations. Event rules and all visible
//! text remain in the bound Rhai script.

use rhai::{Dynamic, Engine, Scope, AST};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::command::CommandResult;
use crate::player::Body;
use crate::world::{get_world_state, EventScript};

thread_local! {
    static ITEM_EVENT_AST_CACHE: RefCell<HashMap<PathBuf, (SystemTime, AST)>> =
        RefCell::new(HashMap::new());
}

fn item_event_path(configured: &str) -> Option<PathBuf> {
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
    Some(Path::new("data/script/아이템").join(filename))
}

fn cached_item_event_ast(path: &Path) -> Result<AST, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("item event metadata {}: {error}", path.display()))?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if let Some(ast) = ITEM_EVENT_AST_CACHE.with(|cache| {
        cache
            .borrow()
            .get(path)
            .filter(|(cached_modified, _)| *cached_modified == modified)
            .map(|(_, ast)| ast.clone())
    }) {
        return Ok(ast);
    }
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("item event read {}: {error}", path.display()))?;
    let executable = format!("{source}\nmain(ob, item_id, item_key, cmdline);");
    let ast = Engine::new()
        .compile(&executable)
        .map_err(|error| format!("item event compile {}: {error}", path.display()))?;
    ITEM_EVENT_AST_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(path.to_path_buf(), (modified, ast.clone()));
    });
    Ok(ast)
}

fn item_name_matches(name: &str, reactions: &str, query: &str) -> (bool, bool) {
    let aliases = reactions.split_whitespace().collect::<Vec<_>>();
    let exact = name == query || aliases.contains(&query);
    let prefix = !exact && aliases.iter().any(|alias| alias.starts_with(query));
    (exact, prefix)
}

/// Match `[inventory item] ... [trigger]` and execute its Rhai binding.
pub(crate) fn try_item_event(
    body: &mut Body,
    _zone: &str,
    raw_line: &str,
) -> Option<CommandResult> {
    let words = raw_line.split_whitespace().collect::<Vec<_>>();
    if words.len() < 2 {
        return None;
    }
    let query = words[0];
    let trigger = *words.last()?;
    let mut candidates = body
        .object
        .objs
        .iter()
        .filter_map(|item| {
            let value = item.lock().ok()?;
            if value.getBool("inUse") {
                return None;
            }
            let (exact, prefix) =
                item_name_matches(&value.getName(), &value.getString("반응이름"), query);
            (exact || prefix).then_some((
                exact,
                Arc::as_ptr(item) as usize as i64,
                value.getString("인덱스"),
            ))
        })
        .collect::<Vec<_>>();
    let mut stack_keys = body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
    stack_keys.sort();
    for key in stack_keys {
        if body.object.inv_stack.get(&key).copied().unwrap_or(0) <= 0 {
            continue;
        }
        let Some((item, _)) = super::object_from_item_json(&key) else {
            continue;
        };
        let Ok(item) = item.lock() else { continue };
        let (exact, prefix) =
            item_name_matches(&item.getName(), &item.getString("반응이름"), query);
        if exact || prefix {
            candidates.push((exact, -1_i64, key));
        }
    }
    candidates.sort_by_key(|(exact, _, _)| std::cmp::Reverse(*exact));
    let (_, item_id, item_key) = candidates.first()?.clone();
    if item_key.is_empty() {
        return None;
    }

    let item_data = {
        let mut world = get_world_state().write().ok()?;
        if world.item_cache.get_item(&item_key).is_none()
            && world.item_cache.load_item(&item_key).is_err()
        {
            return None;
        }
        world.item_cache.get_item(&item_key).cloned()?
    };
    let script = item_data
        .events
        .get(trigger)
        .or_else(|| item_data.events.get(&format!("이벤트 ${trigger}")))?;
    let EventScript::Rhai(configured_path) = script else {
        return None;
    };
    let Some(path) = item_event_path(configured_path) else {
        return Some(CommandResult::Output(
            "(아이템 이벤트 경로가 올바르지 않습니다.)".to_string(),
        ));
    };
    let ast = match cached_item_event_ast(&path) {
        Ok(ast) => ast,
        Err(error) => return Some(CommandResult::Output(format!("({error})"))),
    };

    let output = Arc::new(Mutex::new(Vec::new()));
    let special = Arc::new(Mutex::new(None));
    let user_sends = Arc::new(Mutex::new(Vec::new()));
    let mut engine = super::create_engine_with_body_and_output(
        body,
        output.clone(),
        None,
        None,
        special,
        user_sends,
        None,
        Some("item_event"),
        None,
    );
    let body_ptr = body as *mut Body;
    let selected_stack_key = item_key.clone();
    engine.register_fn("item_event_consume", move |selected_id: i64| -> bool {
        let body = unsafe { &mut *body_ptr };
        if selected_id == -1 {
            return super::inventory_compat::remove_pristine_count(
                &mut body.object,
                &selected_stack_key,
                1,
            );
        }
        let Ok(selected_id) = usize::try_from(selected_id) else {
            return false;
        };
        let Some(index) = body
            .object
            .objs
            .iter()
            .position(|item| Arc::as_ptr(item) as usize == selected_id)
        else {
            return false;
        };
        body.object.objs.remove(index);
        true
    });
    let grant_body = body_ptr;
    engine.register_fn(
        "item_event_grant",
        move |key: &str, count: i64| -> rhai::Array {
            let body = unsafe { &mut *grant_body };
            let mut names = Vec::new();
            for _ in 0..count.clamp(0, 20) {
                let Some((item, name)) = super::object_from_item_json(key) else {
                    break;
                };
                if !super::inventory_compat::store_acquired_object(&mut body.object, item, true) {
                    break;
                }
                names.push(Dynamic::from(name));
            }
            names
        },
    );
    let damage_body = body_ptr;
    engine.register_fn("item_event_damage", move |requested: i64| -> i64 {
        let body = unsafe { &mut *damage_body };
        let current = body.get_hp();
        let applied = requested.max(0).min(current.saturating_sub(1));
        body.set("체력", current.saturating_sub(applied));
        applied
    });

    let mut scope = Scope::new();
    let player_data = super::build_ob_from_body(body);
    scope.push("player", player_data.clone());
    scope.push("me", player_data.clone());
    scope.push("ob", player_data);
    scope.push("item_id", item_id);
    scope.push("item_key", item_key);
    scope.push("cmdline", raw_line.to_string());
    if let Err(error) = engine.run_ast_with_scope(&mut scope, &ast) {
        return Some(CommandResult::Output(format!(
            "(아이템 이벤트 스크립트 오류: {error})"
        )));
    }
    let path = format!("data/user/{}.json", body.get_name());
    let _ = super::save_body_to_json(body, &path);
    let output_lines = output.lock().map(|lines| lines.clone()).unwrap_or_default();
    Some(CommandResult::MobEvent {
        output_lines,
        set_position: None,
        broadcast_lines: Vec::new(),
        room_broadcast_lines: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_dragon_box_event_consumes_grants_multiple_items_and_damages() {
        let mut body = Body::new();
        let player_name = format!("item-event-test-{}", std::process::id());
        let save_path = format!("data/user/{player_name}.json");
        let _ = std::fs::remove_file(&save_path);
        body.set("이름", player_name);
        body.set("체력", 100_i64);
        body.set("최고체력", 100_i64);
        body.object.inv_stack.insert("흑룡봉인함".to_string(), 1);

        let CommandResult::MobEvent { output_lines, .. } =
            try_item_event(&mut body, "사용자맵", "상자 열어").expect("item event")
        else {
            panic!("item event should return event output");
        };
        assert_eq!(body.get_hp(), 63);
        assert!(output_lines.iter().any(|line| line.contains("천뢰진")));
        assert!(output_lines.iter().any(|line| line.contains("기혈 37")));

        assert!(!body.object.inv_stack.contains_key("흑룡봉인함"));
        assert_eq!(body.object.inv_stack.get("강철조각"), Some(&2));
        assert_eq!(body.object.inv_stack.get("피독단"), Some(&1));
        assert!(body.object.objs.is_empty());
        assert!(try_item_event(&mut body, "사용자맵", "상자 열어").is_none());
        let _ = std::fs::remove_file(save_path);
    }
}
