//! Counted item packages (포장).
//!
//! A package is an ordinary, pristine item template with two extra fields in
//! `아이템정보`: `종류: "포장"`, `포장원본` (item index), and `포장수량`.
//! Packages themselves stay in `inv_stack` as one inventory unit.  Unpacking
//! consumes one package and restores the original pristine count.

use rhai::{Dynamic, Engine};
use serde_json::Value as JsonValue;

use crate::player::Body;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageDefinition {
    pub original_key: String,
    pub quantity: i64,
}

fn item_info(key: &str) -> Option<serde_json::Map<String, JsonValue>> {
    let path = format!("data/item/{key}.json");
    let root: JsonValue = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    root.get("아이템정보")?.as_object().cloned()
}

pub(crate) fn package_definition(key: &str) -> Option<PackageDefinition> {
    let info = item_info(key)?;
    let kind = info.get("종류").and_then(JsonValue::as_str).unwrap_or("");
    let original_key = info
        .get("포장원본")
        .or_else(|| info.get("묶음원본"))
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let quantity = info
        .get("포장수량")
        .or_else(|| info.get("묶음수량"))
        .and_then(JsonValue::as_i64)
        .unwrap_or(0);
    if !matches!(kind, "포장" | "묶음" | "꾸러미")
        || original_key.is_empty()
        || original_key == key
        || quantity <= 0
    {
        return None;
    }
    Some(PackageDefinition {
        original_key,
        quantity,
    })
}

/// Only pristine, repeatable consumables/projectiles may be packaged.  This
/// prevents a package definition from duplicating a unique item, equipment,
/// UUID-bearing item, or another stateful object.
pub(crate) fn package_original_is_valid(key: &str) -> bool {
    if !crate::script::is_stackable(key) {
        return false;
    }
    let Some(info) = item_info(key) else {
        return false;
    };
    let kind = info.get("종류").and_then(JsonValue::as_str).unwrap_or("");
    if matches!(
        kind,
        "무기" | "방어구" | "호위" | "포장" | "묶음" | "꾸러미"
    ) {
        return false;
    }
    let forbidden = [
        "고유번호",
        "UUID",
        "uuid",
        "강화",
        "확장",
        "개별",
        "단일아이템",
        "개별인스턴스",
    ];
    if info
        .keys()
        .any(|field| forbidden.iter().any(|x| field.contains(x)))
    {
        return false;
    }
    if let Some(attrs) = info.get("아이템속성") {
        let values = attrs
            .as_array()
            .map(|a| a.iter().filter_map(JsonValue::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        if values
            .iter()
            .any(|value| forbidden.iter().any(|x| value.contains(x)))
        {
            return false;
        }
    }
    true
}

fn package_key_for_query(body: &Body, query: &str) -> Option<String> {
    let mut keys = body
        .object
        .inv_stack
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    keys.sort();
    keys.into_iter().find(|key| {
        package_definition(key).is_some_and(|_| {
            crate::script::object_from_item_json(key).is_some_and(|(item, _)| {
                item.lock().is_ok_and(|item| {
                    item.getName() == query
                        || crate::script::reaction_names(&item.getString("반응이름"))
                            .iter()
                            .any(|alias| alias == query)
                })
            })
        })
    })
}

pub(crate) fn unpack_item(body: &mut Body, query: &str) -> Result<(String, String, i64), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("usage".into());
    }
    let package_key = package_key_for_query(body, query).ok_or_else(|| "not_found".to_string())?;
    let definition =
        package_definition(&package_key).ok_or_else(|| "invalid_package".to_string())?;
    if !package_original_is_valid(&definition.original_key) {
        return Err("invalid_original".into());
    }
    if !crate::script::inventory_compat::remove_pristine_count(&mut body.object, &package_key, 1) {
        return Err("not_found".into());
    }
    if !crate::script::inventory_compat::add_pristine_count(
        &mut body.object,
        &definition.original_key,
        definition.quantity,
    ) {
        // Keep the operation atomic if the original template became invalid
        // during a hot reload.
        *body.object.inv_stack.entry(package_key).or_insert(0) += 1;
        return Err("invalid_original".into());
    }
    let package_name = crate::script::object_from_item_json(&package_key)
        .map(|(_, name)| name)
        .unwrap_or(package_key);
    let original_name = crate::script::object_from_item_json(&definition.original_key)
        .map(|(_, name)| name)
        .unwrap_or(definition.original_key.clone());
    Ok((package_name, original_name, definition.quantity))
}

pub(crate) fn register_package_efun(engine: &mut Engine, body_ptr: *mut Body) {
    engine.register_fn(
        "unpack_item",
        move |_ob: &mut rhai::Map, query: &str| -> rhai::Map {
            let body = unsafe { &mut *body_ptr };
            let mut result = rhai::Map::new();
            match unpack_item(body, query) {
                Ok((package_name, original_name, quantity)) => {
                    result.insert("status".into(), Dynamic::from("ok"));
                    result.insert("package".into(), Dynamic::from(package_name));
                    result.insert("original".into(), Dynamic::from(original_name));
                    result.insert("quantity".into(), Dynamic::from(quantity));
                }
                Err(error) => {
                    result.insert("status".into(), Dynamic::from(error));
                }
            }
            result
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Body;

    #[test]
    fn parses_hanja_package_fields() {
        let root = serde_json::json!({"아이템정보": {
            "종류": "포장", "포장원본": "생수", "포장수량": 100
        }});
        let info = root["아이템정보"].as_object().unwrap().clone();
        assert_eq!(info.get("종류").and_then(JsonValue::as_str), Some("포장"));
        assert_eq!(info.get("포장수량").and_then(JsonValue::as_i64), Some(100));
    }

    #[test]
    fn unpacks_one_package_into_original_count_and_rejects_unique_source() {
        let suffix = format!("package_test_{}", std::process::id());
        let original = format!("{suffix}_water");
        let package = format!("{suffix}_bundle");
        let unique = format!("{suffix}_unique");
        let bad_package = format!("{suffix}_bad_bundle");
        let dir = std::path::Path::new("data/item");
        std::fs::write(
            dir.join(format!("{original}.json")),
            serde_json::json!({"아이템정보": {"이름": "생수", "종류": "먹는것", "무게": 1}})
                .to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{package}.json")),
            serde_json::json!({"아이템정보": {"이름": "생수 100개 포장", "종류": "포장", "포장원본": original, "포장수량": 100}}).to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{unique}.json")),
            serde_json::json!({"아이템정보": {"이름": "기연", "종류": "기타", "아이템속성": ["단일아이템"]}}).to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{bad_package}.json")),
            serde_json::json!({"아이템정보": {"이름": "기연 포장", "종류": "포장", "포장원본": unique, "포장수량": 2}}).to_string(),
        )
        .unwrap();

        let mut body = Body::new();
        body.set("이름", "package-test");
        body.object.inv_stack.insert(package.clone(), 1);
        let result = unpack_item(&mut body, "생수 100개 포장").unwrap();
        assert_eq!(result.2, 100);
        assert_eq!(body.object.inv_stack.get(&original), Some(&100));
        assert!(!body.object.inv_stack.contains_key(&package));

        body.object.inv_stack.insert(bad_package.clone(), 1);
        assert_eq!(
            unpack_item(&mut body, "기연 포장"),
            Err("invalid_original".into())
        );
        assert_eq!(body.object.inv_stack.get(&bad_package), Some(&1));

        for key in [original, package, unique, bad_package] {
            let _ = std::fs::remove_file(dir.join(format!("{key}.json")));
        }
    }

    #[test]
    fn 풀어_script_executes_the_real_command_path() {
        let suffix = format!("package_cmd_test_{}", std::process::id());
        let original = format!("{suffix}_water");
        let package = format!("{suffix}_bundle");
        let dir = std::path::Path::new("data/item");
        std::fs::write(
            dir.join(format!("{original}.json")),
            serde_json::json!({"아이템정보": {"이름": "청수", "종류": "먹는것"}}).to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{package}.json")),
            serde_json::json!({"아이템정보": {"이름": "청수 10개 포장", "종류": "포장", "포장원본": original, "포장수량": 10}}).to_string(),
        )
        .unwrap();
        let mut body = Body::new();
        body.set("이름", "package-command-test");
        body.object.inv_stack.insert(package.clone(), 1);
        let storage = super::super::ScriptStorage::default();
        let (output, _) = storage
            .execute("풀어", &mut body, "청수 10개 포장", None, None, None)
            .unwrap();
        assert!(output.iter().any(|line| line.contains("10개를 꺼냅니다")));
        assert_eq!(body.object.inv_stack.get(&original), Some(&10));
        let _ = std::fs::remove_file(dir.join(format!("{original}.json")));
        let _ = std::fs::remove_file(dir.join(format!("{package}.json")));
        let _ = std::fs::remove_file("data/user/package-command-test.json");
    }
}
