//! JSON persistence for the lending catalogue.
//!
//! Both the Python command scripts and the Rhai efuns use
//! `data/config/book.json`.  Persistence belongs here; user-facing wording
//! remains in the command scripts.

use serde_json::{Map, Value};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub type BookEntry = Value;

static NEXT_BOOK_ID: AtomicU64 = AtomicU64::new(0);

pub fn dict_get<'a>(entry: &'a Value, key: &str) -> Option<&'a Value> {
    entry.as_object()?.get(key)
}

pub fn dict_get_string(entry: &Value, key: &str) -> String {
    match dict_get(entry, key) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        _ => String::new(),
    }
}

pub fn dict_get_bool(entry: &Value, key: &str) -> bool {
    dict_get(entry, key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub fn load(path: impl AsRef<Path>) -> Result<Vec<BookEntry>, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

pub fn save(path: impl AsRef<Path>, entries: &[BookEntry]) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(entries).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

pub fn mark_borrowed(path: impl AsRef<Path>, number: usize, borrower: &str) -> Result<(), String> {
    let path = path.as_ref();
    let mut entries = load(path)?;
    let entry = entries
        .get_mut(number.checked_sub(1).ok_or("invalid catalogue number")?)
        .ok_or("invalid catalogue number")?;
    if !dict_get_bool(entry, "대여가능") {
        return Err("already borrowed".into());
    }
    let object = entry.as_object_mut().ok_or("invalid catalogue entry")?;
    object.insert("대여가능".into(), Value::Bool(false));
    object.insert("대여".into(), Value::String(borrower.into()));
    save(path, &entries)
}

pub fn mark_returned(path: impl AsRef<Path>, item_id: &str) -> Result<(), String> {
    let path = path.as_ref();
    let mut entries = load(path)?;
    let entry = entries
        .iter_mut()
        .find(|entry| dict_get_string(entry, "고유번호") == item_id)
        .ok_or("borrowed entry not found")?;
    let object = entry.as_object_mut().ok_or("invalid catalogue entry")?;
    object.insert("대여가능".into(), Value::Bool(true));
    object.insert("대여".into(), Value::String(String::new()));
    save(path, &entries)
}

fn next_book_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = NEXT_BOOK_ID.fetch_add(1, Ordering::Relaxed);
    format!("book-{millis:x}-{sequence:x}")
}

pub fn register_item(
    path: impl AsRef<Path>,
    key: &str,
    name: &str,
    owner: &str,
    attributes: Map<String, Value>,
) -> Result<(), String> {
    let path = path.as_ref();
    let mut entries = load(path).unwrap_or_default();
    let mut entry = Map::new();
    entry.insert("이름".into(), Value::String(name.into()));
    entry.insert("고유번호".into(), Value::String(next_book_id()));
    entry.insert("등록자".into(), Value::String(owner.into()));
    entry.insert("대여가능".into(), Value::Bool(true));
    entry.insert("인덱스".into(), Value::String(key.into()));
    entry.insert("attr".into(), Value::Object(attributes));
    entries.push(Value::Object(entry));
    save(path, &entries)
}

pub fn remove_entry(
    path: impl AsRef<Path>,
    number: usize,
    owner: &str,
    admin: i64,
    restore: bool,
) -> Result<BookEntry, String> {
    let path = path.as_ref();
    let mut entries = load(path)?;
    let index = number.checked_sub(1).ok_or("invalid catalogue number")?;
    let entry = entries.get(index).ok_or("invalid catalogue number")?;
    if admin < 1000 && dict_get_string(entry, "등록자") != owner {
        return Err("not owner".into());
    }
    if restore && !dict_get_bool(entry, "대여가능") {
        return Err("currently borrowed".into());
    }
    let entry = entries.remove(index);
    save(path, &entries)?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> Value {
        serde_json::json!({
            "이름": "시험검",
            "고유번호": "test-id",
            "등록자": "등록자",
            "대여가능": true,
            "대여": "",
            "인덱스": "시험검",
            "attr": {}
        })
    }

    #[test]
    fn json_mutations_preserve_catalogue_shape() {
        let path = std::env::temp_dir().join(format!("muc-book-{}.json", std::process::id()));
        save(&path, &[sample_entry()]).expect("write json catalogue");
        mark_borrowed(&path, 1, "회귀테스터").expect("borrow");
        mark_returned(&path, "test-id").expect("return");
        let removed = remove_entry(&path, 1, "회귀테스터", 2000, false).expect("delete");
        assert_eq!(dict_get_string(&removed, "인덱스"), "시험검");
        assert!(load(&path).expect("read json catalogue").is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn return_uses_python_unique_item_id_not_shared_template_index() {
        let path =
            std::env::temp_dir().join(format!("muc-book-return-{}.json", std::process::id()));
        let mut first = sample_entry();
        first["고유번호"] = Value::String("first-id".into());
        first["대여가능"] = Value::Bool(false);
        first["대여"] = Value::String("첫대여자".into());
        let mut second = sample_entry();
        second["고유번호"] = Value::String("second-id".into());
        second["대여가능"] = Value::Bool(false);
        second["대여"] = Value::String("둘째대여자".into());
        save(&path, &[first, second]).expect("write catalogue");

        mark_returned(&path, "second-id").expect("return exact item");
        let entries = load(&path).expect("read catalogue");
        assert!(!dict_get_bool(&entries[0], "대여가능"));
        assert!(dict_get_bool(&entries[1], "대여가능"));
        assert_eq!(dict_get_string(&entries[0], "대여"), "첫대여자");
        assert_eq!(dict_get_string(&entries[1], "대여"), "");
        let _ = fs::remove_file(path);
    }
}
