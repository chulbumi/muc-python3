//! Rhai scripting engine for MUD server
//!
//! Provides hot-reloadable scripting support using Rhai.
//! Scripts are stored in cmds/ directory and automatically reloaded on change.

use rhai::{Engine, Scope, Dynamic};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::network::Broadcaster;
use crate::player::Body;
use crate::object::{Object, Value};
use crate::scheduler::CallOutScheduler;
use std::time::Duration;
use crate::data::{GlobalData, SharedGlobalData};
use crate::command::parser::CommandParser;
use crate::command::CommandResult;
use crate::player::{get_hp_bar_string, get_item_level_display, ITEM_EQUIP_LEVELS};
use crate::world::{
    get_world_state, Direction, format_exits_long, format_room_header, MobInstance, PlayerPosition,
    RawMobData, WorldState,
};

/// Script engine configuration
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Directory containing .rhai scripts
    pub script_dir: PathBuf,
    /// Enable hot-reloading
    pub hot_reload: bool,
    /// Script file extension
    pub extension: String,
    /// Data directory for JSON config files
    pub data_dir: PathBuf,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            script_dir: PathBuf::from("cmds"),
            hot_reload: true,
            extension: ".rhai".to_string(),
            data_dir: PathBuf::from("data/config"),
        }
    }
}

// 스크립트용: handle_game_command에서 미리 채워 둔 전 접속자 목록. get_all_online_players()가 참조.
thread_local! {
    static PRE_COMPUTED_ALL_ONLINE: RefCell<Option<rhai::Array>> = RefCell::new(None);
}

/// handle_game_command에서 호출. 전 접속자(이름, 무림별호, 성격, 레벨초기화, 소속) 배열 세팅.
pub fn set_precomputed_all_online(a: rhai::Array) {
    PRE_COMPUTED_ALL_ONLINE.with(|c| *c.borrow_mut() = Some(a));
}

/// 스크립트 get_all_online_players()에서 호출.
pub fn get_precomputed_all_online() -> rhai::Array {
    PRE_COMPUTED_ALL_ONLINE.with(|c| c.borrow().clone()).unwrap_or_default()
}

/// PreComputedOtherDescsGuard Drop에서 호출.
pub fn clear_precomputed_all_online() {
    PRE_COMPUTED_ALL_ONLINE.with(|c| *c.borrow_mut() = None);
}

/// Stored script with metadata
struct StoredScript {
    /// Source code of the script
    source: String,
    /// Last modification time
    modified: std::time::SystemTime,
    /// Script name
    name: String,
}

/// [36m, [37m 등 ESC 없는 축약 ANSI를 \x1b[36m 형태로 확장.
/// 이미 \x1b[...]m 인 경우는 플레이스홀더로 보호 후 복원하여 이중 치환 방지.
fn expand_abbreviated_ansi(s: &str) -> String {
    let mut r = s.to_string();
    let protected: Vec<(String, String)> = vec![
        ("\x1b[36m".into(), "\u{E000}".into()),
        ("\x1b[37m".into(), "\u{E001}".into()),
        ("\x1b[33m".into(), "\u{E002}".into()),
        ("\x1b[0;37m".into(), "\u{E003}".into()),
        ("\x1b[1;32m".into(), "\u{E004}".into()),
    ];
    for (full, place) in &protected {
        r = r.replace(full, place);
    }
    r = r.replace("[;37m", "\x1b[0;37m"); // [0;37m 오타(0 누락) 보정
    r = r.replace("[36m", "\x1b[36m");
    r = r.replace("[37m", "\x1b[37m");
    r = r.replace("[33m", "\x1b[33m");
    r = r.replace("[0;37m", "\x1b[0;37m");
    r = r.replace("[1;32m", "\x1b[1;32m");
    for (full, place) in &protected {
        r = r.replace(place, full);
    }
    r
}

/// ANSI color code mapping for Rhai scripts
fn ansi_convert(msg: &str, conv: bool) -> String {
    let mut buf = msg.to_string();

    if conv {
        buf = buf.replace("{밝}", "\x1b[1m");
        buf = buf.replace("{어}", "\x1b[0m");
        buf = buf.replace("{검}", "\x1b[30m");
        buf = buf.replace("{빨}", "\x1b[31m");
        buf = buf.replace("{초}", "\x1b[32m");
        buf = buf.replace("{노}", "\x1b[33m");
        buf = buf.replace("{파}", "\x1b[34m");
        buf = buf.replace("{자}", "\x1b[35m");
        buf = buf.replace("{하}", "\x1b[36m");
        buf = buf.replace("{흰}", "\x1b[37m");
        buf = buf.replace("{배검}", "\x1b[40m");
        buf = buf.replace("{배빨}", "\x1b[41m");
        buf = buf.replace("{배초}", "\x1b[42m");
        buf = buf.replace("{배노}", "\x1b[43m");
        buf = buf.replace("{배파}", "\x1b[44m");
        buf = buf.replace("{배자}", "\x1b[45m");
        buf = buf.replace("{배하}", "\x1b[46m");
        buf = buf.replace("{배흰}", "\x1b[47m");
    } else {
        buf = buf.replace("{밝}", "");
        buf = buf.replace("{어}", "");
        buf = buf.replace("{검}", "");
        buf = buf.replace("{빨}", "");
        buf = buf.replace("{초}", "");
        buf = buf.replace("{노}", "");
        buf = buf.replace("{파}", "");
        buf = buf.replace("{자}", "");
        buf = buf.replace("{하}", "");
        buf = buf.replace("{흰}", "");
        buf = buf.replace("{배검}", "");
        buf = buf.replace("{배빨}", "");
        buf = buf.replace("{배초}", "");
        buf = buf.replace("{배노}", "");
        buf = buf.replace("{배파}", "");
        buf = buf.replace("{배자}", "");
        buf = buf.replace("{배하}", "");
        buf = buf.replace("{배흰}", "");
    }

    buf
}

/// Korean particle helper (이/가)
fn han_iga(name: &str) -> String {
    use crate::hangul::han_iga;
    han_iga(name).to_string()
}

/// Korean particle helper (을/를) - 목적어 조사
fn han_eul(name: &str) -> String {
    use crate::hangul::han_obj;
    han_obj(name).to_string()
}

/// Korean particle helper (은/는) - placeholder, uses han_iga for now
fn han_eun(name: &str) -> String {
    // TODO: implement proper 은/는 particle
    name.to_string()
}

/// Korean particle helper (와/과)
fn han_wa(name: &str) -> String {
    use crate::hangul::han_wa;
    han_wa(name).to_string()
}

// ---------------------------------------------------------------------------
// 비밀번호 SHA-512 해시
// ---------------------------------------------------------------------------

/// 평문을 SHA-512 해시한 16진수 문자열(128자)로 반환. 저장용.
pub fn password_hash(plain: &str) -> String {
    use sha2::{Sha512, Digest};
    let mut h = Sha512::new();
    h.update(plain.as_bytes());
    format!("{:x}", h.finalize())
}

/// 저장된 값(해시 또는 레거시 평문)과 평문 입력이 일치하는지 검사.
/// - 저장이 128자 16진수면 SHA-512(plain)==stored
/// - 아니면 레거시: stored==plain
pub fn password_verify(stored: &str, plain: &str) -> bool {
    let is_sha512_hex = stored.len() == 128 && stored.chars().all(|c| c.is_ascii_hexdigit());
    if is_sha512_hex {
        password_hash(plain) == stored
    } else {
        stored == plain
    }
}

/// data/user/{name}.json 에서 사용자오브젝트.암호 값을 읽어 반환. 로그인 검증용.
pub fn load_user_password_hash(name: &str) -> Option<String> {
    let path = format!("data/user/{}.json", name);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let uso = json.get("사용자오브젝트")?.as_object()?;
    let s = uso.get("암호")?.as_str()?;
    Some(s.to_string())
}

/// Value -> serde_json::Value (저장용)
fn value_to_serde_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Number(serde_json::Number::from(0))),
        Value::String(s) => serde_json::Value::String(s.clone()),
    }
}

/// serde_json::Value -> Value (로드용)
fn serde_json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::String(String::new()),
        serde_json::Value::Bool(b) => Value::Int(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Int(0)
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            let s = arr
                .iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            Value::String(s)
        }
        serde_json::Value::Object(_) => Value::String(serde_json::to_string(v).unwrap_or_default()),
    }
}

/// Body를 data/user/{이름}.json 에 저장. 소지품(objs, inv_stack) 포함.
/// 저장 직전에 마지막저장시간을 갱신한다.
pub fn save_body_to_json(body: &mut Body, path: &str) -> bool {
    if let Err(_) = std::fs::create_dir_all(Path::new(path).parent().unwrap_or(Path::new("."))) {
        return false;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    body.object.attr.insert("마지막저장시간".to_string(), Value::Int(now));

    let mut uso = serde_json::Map::new();
    for (k, v) in &body.object.attr {
        uso.insert(k.clone(), value_to_serde_json(v));
    }

    let mut items = vec![];
    for obj in &body.object.objs {
        if let Ok(o) = obj.lock() {
            let mut rec = serde_json::Map::new();
            let idx = o.getString("인덱스");
            if idx.is_empty() {
                continue;
            }
            rec.insert("인덱스".to_string(), serde_json::Value::String(idx.clone()));
            rec.insert("이름".to_string(), serde_json::Value::String(o.getName()));
            let rn = o.getString("반응이름");
            if !rn.is_empty() {
                let arr: Vec<serde_json::Value> = rn.split_whitespace().map(|s| serde_json::Value::String(s.to_string())).collect();
                rec.insert("반응이름".to_string(), serde_json::Value::Array(arr));
            }
            for key in &["공격력", "방어력", "기량", "옵션", "아이템속성", "확장 이름", "체력", "고유번호"] {
                let v = o.get(key);
                if !matches!(v, Value::String(ref s) if s.is_empty()) {
                    rec.insert((*key).to_string(), value_to_serde_json(&v));
                }
            }
            if o.getBool("inUse") {
                rec.insert("상태".to_string(), value_to_serde_json(&o.get("계층")));
            }
            items.push(serde_json::Value::Object(rec));
        }
    }

    let mut root = serde_json::Map::new();
    root.insert("사용자오브젝트".to_string(), serde_json::Value::Object(uso));
    root.insert("아이템".to_string(), serde_json::Value::Array(items));
    let stack_map: serde_json::Map<String, serde_json::Value> = body
        .object
        .inv_stack
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::Number(serde_json::Number::from(*v))))
        .collect();
    root.insert("소지품_수량".to_string(), serde_json::Value::Object(stack_map));

    let j = serde_json::Value::Object(root);
    match std::fs::write(path, serde_json::to_string_pretty(&j).unwrap_or_default()) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// data/user/{이름}.json 에서 Body 복원. attr, objs, inv_stack.
/// 파일 없거나 실패 시 false. 성공 시 true.
pub fn load_body_from_json(body: &mut Body, path: &str) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return false,
    };
    let root = match json.as_object() {
        Some(o) => o,
        None => return false,
    };

    if let Some(uso) = root.get("사용자오브젝트").and_then(|v| v.as_object()) {
        body.object.attr.clear();
        for (k, v) in uso {
            body.object.attr.insert(k.clone(), serde_json_to_value(v));
        }
    }

    body.object.inv_stack.clear();
    if let Some(st) = root.get("소지품_수량").and_then(|v| v.as_object()) {
        for (k, v) in st {
            if let Some(n) = v.as_i64() {
                if n > 0 {
                    body.object.inv_stack.insert(k.clone(), n);
                }
            }
        }
    }

    body.object.objs.clear();
    if let Some(arr) = root.get("아이템").and_then(|v| v.as_array()) {
        for it in arr {
            let ob = match it.as_object() {
                Some(o) => o,
                None => continue,
            };
            let idx = match ob.get("인덱스").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let (arc, _) = match object_from_item_json(&idx) {
                Some(t) => t,
                None => continue,
            };
            if let Ok(mut o) = arc.lock() {
                for (k, v) in ob {
                    if k == "인덱스" {
                        continue;
                    }
                    if k == "상태" {
                        o.set("inUse", 1i64);
                        let layer = serde_json_to_value(v);
                        let s = match &layer {
                            Value::String(x) => x.clone(),
                            Value::Int(i) => i.to_string(),
                            Value::Float(f) => f.to_string(),
                        };
                        o.set("계층", s);
                        continue;
                    }
                    if k == "반응이름" {
                        let val = serde_json_to_value(v);
                        let s = match &val {
                            Value::String(x) => x.clone(),
                            Value::Int(i) => i.to_string(),
                            Value::Float(f) => f.to_string(),
                        };
                        o.set(k, s);
                        continue;
                    }
                    let val = serde_json_to_value(v);
                    match val {
                        Value::Int(i) => o.set(k, i),
                        Value::Float(f) => o.set(k, f),
                        Value::String(s) => o.set(k, s),
                    }
                }
            }
            body.object.objs.push(arc);
        }
    }

    true
}

/// Create an Object from data/item/{key}.json 아이템정보.
/// Returns None if file missing or invalid; else Some((object, 아이템정보.이름 or key)).
fn object_from_item_json(key: &str) -> Option<(Arc<Mutex<Object>>, String)> {
    let path = format!("data/item/{}.json", key);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let info = json.get("아이템정보")?.as_object()?;
    let display_name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or(key)
        .to_string();
    let mut obj = Object::new();
    obj.set("인덱스", key); // item JSON 파일명(확장자 제외). 저장/로드·스택 식별용.
    for (k, v) in info {
        match v {
            serde_json::Value::Null => {}
            serde_json::Value::Bool(b) => {
                obj.set(k, if *b { 1i64 } else { 0i64 });
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    obj.set(k, i);
                } else if let Some(f) = n.as_f64() {
                    obj.set(k, f as i64);
                }
            }
            serde_json::Value::String(s) => {
                obj.set(k, s.as_str());
            }
            serde_json::Value::Array(arr) => {
                let s = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                obj.set(k, s);
            }
            serde_json::Value::Object(_) => {}
        }
    }
    Some((Arc::new(Mutex::new(obj)), display_name))
}

/// item JSON에서 이름, 반응이름, 판매가격(또는 값), 무게 반환. 구입 가격 계산용.
fn get_item_info(key: &str) -> Option<(String, String, i64, i64)> {
    let path = format!("data/item/{}.json", key);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let info = json.get("아이템정보")?.as_object()?;
    let name = info.get("이름").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let rn = info
        .get("반응이름")
        .map(|v| {
            if let Some(s) = v.as_str() {
                s.to_string()
            } else if let Some(arr) = v.as_array() {
                arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" ")
            } else {
                String::new()
            }
        })
        .unwrap_or_default();
    let price = info
        .get("판매가격")
        .or_else(|| info.get("값"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let weight = info.get("무게").and_then(|v| v.as_i64()).unwrap_or(0);
    Some((name, rn, price, weight))
}

/// 아이템 설명1. data/item/{key}.json. 방 바닥 스택 표시용.
fn get_item_desc1(key: &str) -> String {
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return String::new(),
    };
    let info = match json.get("아이템정보").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return String::new(),
    };
    info.get("설명1")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// 아이템 인덱스(스택)가 누적 가능한지. 무기/방어구·개별인스턴스 아니면 true.
fn is_stackable(key: &str) -> bool {
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return false,
    };
    let info = match json.get("아이템정보").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return false,
    };
    let kind = info.get("종류").and_then(|v| v.as_str()).unwrap_or("기타");
    if kind == "무기" || kind == "방어구" {
        return false;
    }
    let attrs = info.get("아이템속성");
    if let Some(serde_json::Value::Array(arr)) = attrs {
        for v in arr {
            if v.as_str() == Some("개별인스턴스") {
                return false;
            }
        }
    } else if let Some(serde_json::Value::String(s)) = attrs {
        if s.contains("개별인스턴스") {
            return false;
        }
    }
    true
}

/// 이름 또는 반응이름으로 아이템 인덱스(키) 찾기. data/item/*.json 검색.
fn find_item_key_by_name(name: &str) -> Option<String> {
    if name.is_empty() {
        return None;
    }
    let dir = std::path::Path::new("data/item");
    let read_dir = match std::fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return None,
    };
    for e in read_dir.flatten() {
        let p = e.path();
        if p.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let key = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        if let Some((iname, rn, _, _)) = get_item_info(&key) {
            if iname == name {
                return Some(key);
            }
            if !rn.is_empty() && rn.split_whitespace().any(|s| s == name) {
                return Some(key);
            }
        }
    }
    None
}

/// Global reference to the current object being accessed
/// This is set by the driver before calling script functions
static mut CURRENT_OBJECT: Option<Object> = None;

/// Set the current object context (called by driver)
pub fn set_current_object(obj: Object) {
    unsafe {
        CURRENT_OBJECT = Some(obj);
    }
}

/// Get the current object context
pub fn get_current_object() -> Option<Object> {
    unsafe {
        CURRENT_OBJECT.clone()
    }
}

/// Create a new Rhai engine with all API functions registered
pub fn create_engine() -> Engine {
    let mut engine = Engine::new();

    // ============================================================
    // UTILITY FUNCTIONS
    // ============================================================

    engine.register_fn("random", |min: i64, max: i64| -> i64 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    });

    engine.register_fn("abs", |n: i64| -> i64 { n.abs() });

    // String utilities
    engine.register_fn("contains", |s: &str, pattern: &str| -> bool {
        s.contains(pattern)
    });
    engine.register_fn("starts_with", |s: &str, pattern: &str| -> bool {
        s.starts_with(pattern)
    });
    engine.register_fn("ends_with", |s: &str, pattern: &str| -> bool {
        s.ends_with(pattern)
    });
    engine.register_fn("trim", |s: &str| -> String {
        s.trim().to_string()
    });
    engine.register_fn("length", |s: &str| -> i64 {
        s.chars().count() as i64
    });

    // Array utilities
    engine.register_fn("len", |arr: &mut rhai::Array| -> i64 {
        arr.len() as i64
    });

    // ============================================================
    // ANSI COLOR CONVERSION
    // ============================================================

    engine.register_fn("ansi", |msg: &str, conv: bool| -> String {
        ansi_convert(msg, conv)
    });

    // ============================================================
    // KOREAN PARTICLE HELPERS
    // ============================================================

    engine.register_fn("han_iga", |name: &str| -> String {
        han_iga(name)
    });
    engine.register_fn("han_eul", |name: &str| -> String {
        han_eul(name)
    });
    engine.register_fn("han_eun", |name: &str| -> String {
        han_eun(name)
    });
    engine.register_fn("han_wa", |name: &str| -> String {
        han_wa(name)
    });

    // 이름 ANSI(노랑), 문자열 치환, 정수→문자. format_room_objs.rhai 등에서 사용.
    engine.register_fn("name_ansi", |s: &str| -> String {
        format!("\x1b[33m{}\x1b[37m", s)
    });
    engine.register_fn("replace_str", |s: &str, from: &str, to: &str| -> String {
        s.replace(from, to)
    });
    engine.register_fn("int_to_str", |i: i64| -> String {
        i.to_string()
    });

    // ============================================================
    // OUTPUT FUNCTIONS
    // ============================================================

    engine.register_fn("print", |s: &str| {
        println!("[SCRIPT] {}", s);
    });
    engine.register_fn("debug", |s: &str| {
        debug!("[SCRIPT] {}", s);
    });

    // Player action functions
    // Note: These now need access to the scope's _output variable
    // For now, we'll use a simpler approach - just print and return
    engine.register_fn("send_line", |player_data: &mut rhai::Map, msg: &str| {
        println!("[SEND_LINE] {}", msg);
        // Store in player_data for now - scripts can use get_attr/set_attr
        let output = player_data.get_mut("_output");
        if let Some(arr) = output {
            if let Some(mut vec) = arr.clone().try_cast::<rhai::Array>() {
                let msg_string = msg.to_string();
                let msg_dynamic = rhai::Dynamic::from(msg_string);
                vec.push(msg_dynamic);
                player_data.insert("_output".into(), rhai::Dynamic::from(vec));
            }
        }
    });

    engine.register_fn("send_room", |player_data: &mut rhai::Map, msg: &str| {
        println!("[SEND_ROOM] {}", msg);
        let output = player_data.get_mut("_output");
        if let Some(arr) = output {
            if let Some(mut vec) = arr.clone().try_cast::<rhai::Array>() {
                let msg_string = msg.to_string();
                let msg_dynamic = rhai::Dynamic::from(msg_string);
                vec.push(msg_dynamic);
                player_data.insert("_output".into(), rhai::Dynamic::from(vec));
            }
        }
    });

    // ============================================================
    // ATTRIBUTE ACCESS (Player/Object data)
    // ============================================================

    engine.register_fn("get_attr", |player_data: &mut rhai::Map, key: &str| -> Dynamic {
        player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("set_attr", |player_data: &mut rhai::Map, key: &str, value: Dynamic| {
        player_data.insert(key.to_string().into(), value);
    });

    engine.register_fn("get_int", |player_data: &mut rhai::Map, key: &str| -> i64 {
        player_data.get(key)
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    engine.register_fn("get_string", |player_data: &mut rhai::Map, key: &str| -> String {
        player_data.get(key)
            .and_then(|v| {
                if v.is_string() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    // ============================================================
    // STRING MANIPULATION HELPERS
    // ============================================================

    engine.register_fn("fill_space", |width: i64, s: &str| -> String {
        let len = s.chars().count() as i64;
        if len >= width {
            s.to_string()
        } else {
            format!("{}{:width$}", s, " ", width = (width - len) as usize)
        }
    });

    engine.register_fn("strip_ansi", |s: &str| -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if let Some(&'[') = chars.peek() {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    });

    engine.register_fn("to_int", |s: &str| -> i64 {
        s.trim().parse().unwrap_or(0)
    });

    engine.register_fn("join", |arr: &mut rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });

    // ============================================================
    // OBJECT QUERY FUNCTIONS (EFUNS)
    // ============================================================

    // environment(obj) - Get parent object
    engine.register_fn("environment", |obj_data: &mut rhai::Map| -> Dynamic {
        // In full implementation, would return the env object
        // For now, return the environment name
        obj_data.get("env").cloned().unwrap_or(Dynamic::UNIT)
    });

    // all_inventory(obj) - Get all child objects
    engine.register_fn("all_inventory", |obj_data: &mut rhai::Map| -> Dynamic {
        obj_data.get("objs").cloned().unwrap_or(Dynamic::UNIT)
    });

    // present(name, env) - Find object by name in environment
    // Simplified version - returns UNIT for now
    // TODO: Implement full search in objs array
    engine.register_fn("present", |name: &str, _env: rhai::Map| -> Dynamic {
        // For now, just return UNIT
        // Full implementation would search through env["objs"] array
        let _ = (name, _env); // Suppress unused warning
        Dynamic::UNIT
    });

    // ============================================================
    // DATA LOADING FUNCTIONS (EFUNS)
    // ============================================================

    engine.register_fn("load_json", |path: &str| -> Dynamic {
        // Load JSON data from data/config/
        let full_path = format!("data/config/{}.json", path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                // Parse JSON (basic implementation)
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        // Convert to Rhai Dynamic
                        json_value_to_dynamic(value)
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_item_data", |name: &str| -> Dynamic {
        let full_path = format!("data/item/{}.json", name);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        // Extract 아이템정보
                        if let Some(obj) = value.get("아이템정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_mob_data", |name: &str| -> Dynamic {
        let full_path = format!("data/mob/{}.json", name);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(obj) = value.get("몹정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_room_data", |name: &str| -> Dynamic {
        // name format: "zone:room"
        let full_path = format!("data/map/{}.json", name.replace(":", "/"));
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(obj) = value.get("맵정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_skill_data", |name: &str| -> Dynamic {
        // Load skill.json and find the skill
        match std::fs::read_to_string("data/config/skill.json") {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(skills) = value.as_object() {
                            if let Some(skill) = skills.get(name) {
                                json_value_to_dynamic(skill.clone())
                            } else {
                                Dynamic::UNIT
                            }
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine
}

/// 바닥 아이템 이름별 묶음 포맷. 파이썬 viewMapData nStr. format_room_objs.rhai와 동일 로직을 Rust로 구현.
/// grouped: (name, count, desc1) 들. 공통: 봐/이동 시 방 표시.
pub fn format_room_objs_display(
    grouped: Vec<(String, usize, String)>,
) -> String {
    if grouped.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = Vec::with_capacity(grouped.len());
    for (name, count, desc1) in grouped {
        let name_a = format!("\x1b[33m{}\x1b[37m", name);
        let line = if desc1.is_empty() {
            if count == 1 {
                format!("○ {}{} 바닥에 떨어져 있다.", name_a, han_iga(&name))
            } else {
                format!("○ {} {}개가 바닥에 떨어져 있다.", name_a, count)
            }
        } else if count == 1 {
            desc1.replace("$아이템$", &name_a)
        } else {
            desc1.replace("$아이템$", &format!("{} {}개", name_a, count))
        };
        lines.push(line);
    }
    format!("\r\n{}", lines.join("\r\n"))
}

/// 바닥 아이템을 이름별로 묶어 format_room_objs_display로 포맷. room_objs + room_inv_stack 병합.
pub fn build_room_objs_grouped(
    room_objs: &[std::sync::Arc<std::sync::Mutex<Object>>],
    room_inv_stack: &std::collections::HashMap<String, i64>,
) -> String {
    let mut map: HashMap<String, (usize, String)> = HashMap::new();
    for arc in room_objs {
        if let Ok(o) = arc.lock() {
            let name = o.getName();
            let desc1 = o.getString("설명1");
            map.entry(name)
                .and_modify(|e| e.0 += 1)
                .or_insert((1, desc1));
        }
    }
    for (key, cnt) in room_inv_stack {
        if *cnt <= 0 {
            continue;
        }
        if let Some((name, _, _, _)) = get_item_info(key) {
            let desc1 = get_item_desc1(key);
            map.entry(name)
                .and_modify(|e| e.0 += *cnt as usize)
                .or_insert((*cnt as usize, desc1));
        }
    }
    let grouped: Vec<(String, usize, String)> = map
        .into_iter()
        .map(|(name, (count, desc1))| (name, count, desc1))
        .collect();
    format_room_objs_display(grouped)
}

/// 방 전체 문자열(헤더·설명·출구·몹·바닥아이템·다른유저). view_map_data efun 및 show_room_to_player_with_world와 동일 포맷.
/// other_player_descs: 같은 방의 다른 접속 유저 getDesc. 파이썬 viewMapData for obj in room.objs: is_player then getDesc.
pub fn build_room_lines(player_name: &str, other_player_descs: &[String]) -> String {
    let world = get_world_state().read().unwrap();
    let pos = match world.get_player_position(player_name) {
        Some(p) => p.clone(),
        None => {
            return "\x1b[1;31m위치 정보가 없습니다.\x1b[0;37m".to_string();
        }
    };
    if let Some(room) = world.room_cache.get_room_cached(&pos.zone, &pos.room.to_string()) {
        let room_ref = match room.read() {
            Ok(r) => r,
            Err(_) => return "\x1b[1;31m방 정보를 읽을 수 없습니다.\x1b[0;37m".to_string(),
        };
        let room_name_formatted = format_room_header(&room_ref.display_name);
        let exits_str = format_exits_long(&*room_ref);
        let mobs = world.mob_cache.get_mobs_in_room(&pos.zone, pos.room);
        let mob_str = if mobs.is_empty() {
            String::new()
        } else {
            let mut mob_msgs = Vec::new();
            for mob in mobs {
                if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                    if !mob_data.desc1.is_empty() {
                        mob_msgs.push(mob_data.desc1.clone());
                    }
                }
            }
            if mob_msgs.is_empty() {
                String::new()
            } else {
                format!("\r\n{}", mob_msgs.join("\r\n"))
            }
        };
        let room_objs = world.get_room_objs(&pos.zone, pos.room);
        let room_stack = world.get_room_objs_stack(&pos.zone, pos.room);
        let item_str = build_room_objs_grouped(&room_objs, &room_stack);
        let mut out = String::new();
        out.push_str("\r\n");
        out.push_str(&room_name_formatted);
        out.push_str("\r\n\r\n");
        out.push_str(&room_ref.description.join("\r\n"));
        out.push_str("\r\n\r\n");
        out.push_str(&exits_str);
        out.push_str("\r\n");
        if !mob_str.is_empty() {
            out.push_str(&mob_str);
            out.push_str("\r\n");
        }
        if !item_str.is_empty() {
            out.push_str(&item_str);
            out.push_str("\r\n");
        }
        for s in other_player_descs {
            out.push_str(s);
            out.push_str("\r\n");
        }
        out
    } else {
        format!(
            "\x1b[1;37m[{}:{}]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n",
            pos.zone, pos.room
        )
    }
}

/// data/item/{key}.json에서 아이템정보.계층, 아이템정보.이름 반환. 없으면 None.
fn get_item_slot_name(key: &str) -> Option<(String, String)> {
    let path = format!("data/item/{}.json", key);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let info = json.get("아이템정보")?.as_object()?;
    let slot = info
        .get("계층")
        .and_then(|v| v.as_str())
        .unwrap_or("기타")
        .to_string();
    let name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or(key)
        .to_string();
    Some((slot, name))
}

/// 파이썬 objs/player.view(ob). 나/다른 플레이어 상세: 이름·성격·배우자·나이·소속·직위·장비·HP.
fn player_view(body: &Body, _myself: bool) -> Vec<String> {
    let mut lines = vec!["━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string()];
    let m = body.get_string("무림별호");
    let m = if m.is_empty() { "무명객".to_string() } else { m };
    let c = body.get_string("성격");
    let c = if c.is_empty() { "없음".to_string() } else { c };
    let s = format!(
        "◆ 이  름 ▷ 『{}』 {}",
        m,
        body.get_name()
    );
    let c2 = format!("◆ 성격 ▷ 『{}』", c);
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}  {}\x1b[0m\x1b[37m\x1b[40m",
        s, c2
    ));
    let ba = body.get_string("배우자");
    let ba = if ba.is_empty() { "미혼".to_string() } else { ba };
    let age = body.get_int("나이");
    let sex = body.get_string("성별");
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 배우자 ▷ 『{}』  ◆ 나이 ▷ {}살({})\x1b[0m\x1b[37m\x1b[40m",
        ba, age, sex
    ));
    let so = body.get_string("소속");
    if !so.is_empty() {
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m■ 소  속 ▷ 『{}』\x1b[0m\x1b[37m\x1b[40m",
            so
        ));
        let jw = body.get_string("직위");
        let r = body.get_string("방파별호");
        let jw_line = if r.is_empty() {
            format!("■ 직  위 ▷ 『{}』", jw)
        } else {
            format!("■ 직  위 ▷ 『{}({})』", jw, r)
        };
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m",
            jw_line
        ));
    }
    lines.push("──────────────────────────────".to_string());
    let mut item_str = String::new();
    for &lv in ITEM_EQUIP_LEVELS.iter() {
        for arc in &body.object.objs {
            if let Ok(o) = arc.lock() {
                if !o.getBool("inUse") {
                    continue;
                }
                let sl = o.getString("계층");
                if sl != lv {
                    continue;
                }
                let disp = get_item_level_display(lv);
                item_str.push_str(&format!("[{}] \x1b[36m{}\x1b[37m\r\n", disp, o.getName()));
            }
        }
    }
    if item_str.is_empty() {
        lines.push("\x1b[36m☞ 혈혈단신 맨몸으로 강호를 주유중입니다.\x1b[37m".to_string());
    } else {
        lines.push(item_str.trim_end().to_string());
    }
    lines.push("──────────────────────────────".to_string());
    lines.push(format!("★ {}", body.get_hp_string()));
    lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// 파이썬 objs/mob.view(ob). 살아있는 몹: 이름·설명2·사용아이템·HP·HPbar. 시체: 이름의 시체.
fn mob_view(mob: &MobInstance, data: &RawMobData) -> Vec<String> {
    let mut lines = vec!["━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string()];
    if !mob.alive {
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}\x1b[0m\x1b[37m\x1b[40m",
            format!("{}의 시체", data.name)
        ));
        lines.push("──────────────────────────────".to_string());
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        return lines;
    }
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}\x1b[0m\x1b[37m\x1b[40m",
        data.name
    ));
    lines.push("──────────────────────────────".to_string());
    if data.desc2.is_empty() {
        // no desc2 lines
    } else {
        for d in &data.desc2 {
            lines.push(d.clone());
        }
    }
    lines.push("──────────────────────────────".to_string());
    let mut use_lines: Vec<(String, String)> = Vec::new();
    for &lv in ITEM_EQUIP_LEVELS.iter() {
        for (key, _cnt, _prob) in &data.use_items {
            if let Some((slot, iname)) = get_item_slot_name(key) {
                if slot == lv {
                    let disp = get_item_level_display(lv);
                    use_lines.push((disp.to_string(), iname));
                    break;
                }
            }
        }
    }
    for (disp, iname) in &use_lines {
        lines.push(format!("[{}] \x1b[36m{}\x1b[37m", disp, iname));
    }
    if !use_lines.is_empty() {
        lines.push("──────────────────────────────".to_string());
    }
    let max_hp = if mob.max_hp <= 0 { 1 } else { mob.max_hp };
    let pct = (mob.hp * 100) / max_hp;
    lines.push(format!(
        "★ {} ({}%)",
        get_hp_bar_string(mob.hp, mob.max_hp),
        pct
    ));
    lines.push(format!("☆ {} ({})", get_hp_bar_string(mob.hp, mob.max_hp), pct));
    lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// 아이템 상세 보기. 파이썬 objs/item.view(ob). find_target/look_at_target에서 사용.
fn item_view(obj: &Arc<Mutex<Object>>) -> Vec<String> {
    let o = obj.lock().unwrap();
    let name_a = o.getNameA();
    let mut lines = vec![
        "━━━━━━━━━━━━━━━━━━━━━".to_string(),
        format!("\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}\x1b[0m\x1b[37m\x1b[40m", o.getName()),
        format!("\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 종류 ▷ {}\x1b[0m\x1b[37m\x1b[40m", o.getString("종류")),
        "─────────────────────".to_string(),
    ];
    let desc2 = o.getString("설명2");
    let desc = if desc2.is_empty() {
        o.getString("설명1").replace("$아이템$", &name_a)
    } else {
        desc2.replace("$아이템$", &name_a)
    };
    for line in desc.lines() {
        lines.push(line.to_string());
    }
    let opt = o.getString("옵션");
    if !opt.is_empty() {
        lines.push(opt);
    }
    lines.push("━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// [대상] 봐: 나|findObjInven|find_in_room(아이템,몹,플레이어,출구) 검색 후 타입별 표시.
/// returns (viewer_lines, Option<(target_player_name, msg_to_target)>)
fn look_at_target(
    body: &Body,
    world: &WorldState,
    viewer_name: &str,
    target_line: &str,
    other_player_descs: &HashMap<String, String>,
) -> (Vec<String>, Option<(String, String)>) {
    let not_found = (
        vec!["\x1b[1;31m☞ 당신의 안광으로는 그런것을 볼수 없다네\x1b[0;37m".to_string()],
        None,
    );

    if target_line.trim() == "나" {
        return (player_view(body, true), None);
    }

    let (mut name, mut order) = CommandParser::parse_name_order(target_line);
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(o) = name.parse::<usize>() {
            if o >= 1 {
                name = String::new();
                order = o;
            }
        }
    }

    if !name.is_empty() {
        if let Some(obj) = body.object.findObjInven(&name, order) {
            return (item_view(&obj), None);
        }
    }

    let pos = match world.get_player_position(viewer_name) {
        Some(p) => p,
        None => return (vec!["위치 정보가 없습니다.".to_string()], None),
    };
    let zone = pos.zone.as_str();
    let room_i = pos.room;
    let mut c = 0usize;

    if name.is_empty() && order >= 1 {
        for mob in world.mob_cache.get_mobs_in_room(zone, room_i) {
            if !mob.alive {
                continue;
            }
            if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
                if data.mob_type == 7 {
                    continue;
                }
                c += 1;
                if c == order {
                    return (mob_view(mob, data), None);
                }
            }
        }
        return not_found;
    }

    if name == "시체" {
        for mob in world.mob_cache.get_mobs_in_room(zone, room_i) {
            if !mob.alive {
                c += 1;
                if c == order {
                    if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
                        return (mob_view(mob, data), None);
                    }
                }
            }
        }
        return not_found;
    }

    // 파이썬 room.findObjName: 이름==name, name in 반응이름, 또는 alias.find(name)==0(alias가 name으로 시작)
    let room_objs = world.get_room_objs(zone, room_i);
    for arc in &room_objs {
        let ok = {
            if let Ok(o) = arc.lock() {
                let n = o.getName();
                let reac = o.getString("반응이름");
                n == name
                    || reac.split_whitespace().any(|s| s == name || s.starts_with(name.as_str()))
            } else {
                false
            }
        };
        if ok {
            c += 1;
            if c == order {
                return (item_view(arc), None);
            }
        }
    }

    // 파이썬: 이름==name, name in 반응이름, 또는 reaction.find(name)==0
    for mob in world.mob_cache.get_mobs_in_room(zone, room_i) {
        if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
            let ok = data.name == name
                || data.name.starts_with(name.as_str())
                || data.reaction_names.iter().any(|r| r.as_str() == name || r.starts_with(name.as_str()));
            if ok {
                c += 1;
                if c == order {
                    return (mob_view(mob, data), None);
                }
            }
        }
    }

    // 파이썬: 이름 정확 or 대상 이름이 입력으로 시작(멍멍 → 멍멍이)
    for (pname, desc) in other_player_descs {
        if *pname == name || pname.starts_with(name.as_str()) {
            c += 1;
            if c == order {
                let msg = format!("{} 당신을 살펴봅니다.", body.han_iga());
                return (vec![desc.clone()], Some((pname.clone(), msg)));
            }
        }
    }

    if let Some(dir) = Direction::from_korean(&name) {
        if let Some(room_arc) = world.room_cache.get_room_cached(zone, &room_i.to_string()) {
            if let Ok(room_guard) = room_arc.read() {
                if room_guard.get_exit(dir).is_some() {
                    c += 1;
                    if c == order {
                        return (
                            vec![format!("{}쪽으로 갈 수 있습니다.", dir.korean_name())],
                            None,
                        );
                    }
                }
            }
        }
    }

    not_found
}

/// Create a Rhai engine with output collection support
///
/// This creates an engine where `send_line` and `send_room` write to a shared output collector.
pub fn create_engine_with_output(output_collector: Arc<Mutex<Vec<String>>>) -> Engine {
    let mut engine = Engine::new();

    // ============================================================
    // UTILITY FUNCTIONS
    // ============================================================

    engine.register_fn("random", |min: i64, max: i64| -> i64 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    });

    engine.register_fn("abs", |n: i64| -> i64 { n.abs() });

    // String utilities
    engine.register_fn("contains", |s: &str, pattern: &str| -> bool {
        s.contains(pattern)
    });
    engine.register_fn("starts_with", |s: &str, pattern: &str| -> bool {
        s.starts_with(pattern)
    });
    engine.register_fn("ends_with", |s: &str, pattern: &str| -> bool {
        s.ends_with(pattern)
    });
    engine.register_fn("trim", |s: &str| -> String {
        s.trim().to_string()
    });
    engine.register_fn("length", |s: &str| -> i64 {
        s.chars().count() as i64
    });

    // Array utilities
    engine.register_fn("len", |arr: &mut rhai::Array| -> i64 {
        arr.len() as i64
    });

    // ============================================================
    // ANSI COLOR CONVERSION
    // ============================================================

    engine.register_fn("ansi", |msg: &str, conv: bool| -> String {
        ansi_convert(msg, conv)
    });

    // ============================================================
    // KOREAN PARTICLE HELPERS
    // ============================================================

    engine.register_fn("han_iga", |name: &str| -> String {
        han_iga(name)
    });
    engine.register_fn("han_eul", |name: &str| -> String {
        han_eul(name)
    });
    engine.register_fn("han_eun", |name: &str| -> String {
        han_eun(name)
    });
    engine.register_fn("han_wa", |name: &str| -> String {
        han_wa(name)
    });

    // ============================================================
    // OUTPUT FUNCTIONS (with collection)
    // ============================================================

    let oc = output_collector.clone();
    engine.register_fn("send_line", move |_player_data: &mut rhai::Map, msg: &str| {
        println!("[SEND_LINE] {}", msg);
        if let Ok(mut output) = oc.lock() {
            output.push(msg.to_string());
        }
    });

    let oc = output_collector.clone();
    engine.register_fn("send_room", move |_player_data: &mut rhai::Map, msg: &str| {
        println!("[SEND_ROOM] {}", msg);
        if let Ok(mut output) = oc.lock() {
            output.push(msg.to_string());
        }
    });

    engine.register_fn("print", |s: &str| {
        println!("[SCRIPT] {}", s);
    });
    engine.register_fn("debug", |s: &str| {
        debug!("[SCRIPT] {}", s);
    });

    // ============================================================
    // ATTRIBUTE ACCESS
    // ============================================================

    engine.register_fn("get_attr", |player_data: &mut rhai::Map, key: &str| -> Dynamic {
        player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("set_attr", |player_data: &mut rhai::Map, key: &str, value: Dynamic| {
        player_data.insert(key.to_string().into(), value);
    });

    engine.register_fn("get_int", |player_data: &mut rhai::Map, key: &str| -> i64 {
        player_data.get(key)
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    engine.register_fn("get_string", |player_data: &mut rhai::Map, key: &str| -> String {
        player_data.get(key)
            .and_then(|v| {
                if v.is_string() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    // ============================================================
    // STRING MANIPULATION HELPERS
    // ============================================================

    engine.register_fn("fill_space", |width: i64, s: &str| -> String {
        let len = s.chars().count() as i64;
        if len >= width {
            s.to_string()
        } else {
            format!("{}{:width$}", s, " ", width = (width - len) as usize)
        }
    });

    engine.register_fn("strip_ansi", |s: &str| -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if let Some(&'[') = chars.peek() {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    });

    engine.register_fn("pad_start", |s: &str, width: i64, fill: &str| -> String {
        let len = s.chars().count() as i64;
        if len >= width {
            s.to_string()
        } else {
            format!("{}{:width$}", fill.repeat((width - len) as usize), s, width = width as usize)
        }
    });

    engine.register_fn("to_int", |s: &str| -> i64 {
        s.trim().parse().unwrap_or(0)
    });

    engine.register_fn("int_to_str", |i: i64| -> String {
        i.to_string()
    });

    engine.register_fn("split", |s: &str, sep: &str| -> rhai::Array {
        s.split(sep).map(|x| rhai::Dynamic::from(x.to_string())).collect()
    });

    // Parse "2검" to (order, name). Returns [order: i64, name: string] as Array.
    // Python getNameOrder: "1" 전부 숫자면 name="1" 유지(아이템 "1" 찾음). "2.검"이면 order=2, name=".검".
    engine.register_fn("parse_order_name", |s: &str| -> rhai::Array {
        let s = s.trim();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0usize;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let (order, name) = if i > 0 {
            let num_str: String = chars[..i].iter().collect();
            let n: i64 = num_str.parse().unwrap_or(1);
            let rest: String = chars[i..].iter().collect();
            // 전부 숫자("1","2")면 name=원문. 그래야 "1 버려"가 아이템 "1"을 찾고 없으면 실패.
            let name = if rest.is_empty() { s.to_string() } else { rest };
            (n.max(1), name)
        } else {
            (1i64, s.to_string())
        };
        let mut arr = rhai::Array::new();
        arr.push(rhai::Dynamic::from(order));
        arr.push(rhai::Dynamic::from(name));
        arr
    });

    // parse_name_order(s): "2.검" -> [name, order]. 주다 등. CommandParser::parse_name_order.
    engine.register_fn("parse_name_order", |s: &str| -> rhai::Array {
        let (name, order) = CommandParser::parse_name_order(s);
        let mut arr = rhai::Array::new();
        arr.push(rhai::Dynamic::from(name));
        arr.push(rhai::Dynamic::from(order as i64));
        arr
    });

    // ============================================================
    // COMMAND HELPER EFUNS (반복 패턴)
    // ============================================================

    engine.register_fn("is_empty", |s: &str| -> bool { s.trim().is_empty() });

    engine.register_fn("ob_name", |ob: &mut rhai::Map| -> String {
        ob.get("이름")
            .and_then(|v| {
                if v.is_string() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    engine.register_fn("ob_iga", |ob: &mut rhai::Map| -> String {
        let n = ob
            .get("이름")
            .and_then(|v| {
                if v.is_string() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        han_iga(&n)
    });

    engine.register_fn("line_args", |line: &str| -> rhai::Array {
        line.trim()
            .split_whitespace()
            .map(|x| rhai::Dynamic::from(x.to_string()))
            .collect()
    });

    let oc_req = output_collector.clone();
    engine.register_fn("require_arg", move |_ob: &mut rhai::Map, line: &str, usage: &str| -> bool {
        if line.trim().is_empty() {
            if let Ok(mut o) = oc_req.lock() {
                o.push(usage.to_string());
            }
            return false;
        }
        true
    });

    let oc_adm = output_collector.clone();
    engine.register_fn("require_admin", move |ob: &mut rhai::Map, min_level: i64| -> bool {
        let adm = ob
            .get("관리자등급")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0i64);
        if adm < min_level {
            if let Ok(mut o) = oc_adm.lock() {
                o.push("☞ 무슨 말인지 모르겠어요. *^_^*".to_string());
            }
            return false;
        }
        true
    });

    let oc_act = output_collector.clone();
    engine.register_fn(
        "send_item_action",
        move |_ob: &mut rhai::Map, name: &str, verb: &str, count: i64| {
            let eul = han_eul(name);
            let msg1 = if count == 1 {
                format!("당신이 \x1b[36m{}{}\x1b[37m {}", name, eul, verb)
            } else {
                format!("당신이 \x1b[36m{}\x1b[37m {}개를 {}", name, count, verb)
            };
            // msg2(이 OO은/는 verb — 방의 다른 사용자용)는 send_room 연동 시 푸시
            if let Ok(mut o) = oc_act.lock() {
                o.push(msg1);
            }
        },
    );

    // ============================================================
    // DATA LOADING (get_item_data, get_mob_data, get_room_data, get_skill_data)
    // ============================================================

    engine.register_fn("get_item_data", |name: &str| -> Dynamic {
        let full_path = format!("data/item/{}.json", name);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(obj) = value.get("아이템정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_mob_data", |name: &str| -> Dynamic {
        let full_path = format!("data/mob/{}.json", name);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(obj) = value.get("몹정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_room_data", |name: &str| -> Dynamic {
        let full_path = format!("data/map/{}.json", name.replace(":", "/"));
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(obj) = value.get("맵정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_skill_data", |name: &str| -> Dynamic {
        match std::fs::read_to_string("data/config/skill.json") {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(skills) = value.as_object() {
                            if let Some(skill) = skills.get(name) {
                                json_value_to_dynamic(skill.clone())
                            } else {
                                Dynamic::UNIT
                            }
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    // get_help(topic): data/config/help.json의 {"도움말": { "도움말": [...], ... }}에서
    // topic이 비거나 "도움말"이면 ["도움말"]["도움말"], 아니면 ["도움말"][topic] 배열을 "\r\n"으로 이어서 반환. 없으면 "".
    engine.register_fn("get_help", |topic: &str| -> String {
        let key = {
            let t = topic.trim();
            if t.is_empty() || t == "도움말" {
                "도움말"
            } else {
                t
            }
        };
        let content = match std::fs::read_to_string("data/config/help.json") {
            Ok(c) => c,
            Err(_) => return String::new(),
        };
        let root: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return String::new(),
        };
        let outer = match root.get("도움말") {
            Some(o) => o,
            None => return String::new(),
        };
        let arr = match outer.get(key).and_then(|v| v.as_array()) {
            Some(a) => a,
            None => return String::new(),
        };
        arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join("\r\n")
    });

    // ============================================================
    // PLAYER DATA ACCESS
    // ============================================================

    engine.register_fn("get_player_data", |player_data: &mut rhai::Map, key: &str| -> Dynamic {
        player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
    });

    // ============================================================
    // MATH FUNCTIONS
    // ============================================================

    engine.register_fn("min", |a: i64, b: i64| -> i64 { a.min(b) });
    engine.register_fn("max", |a: i64, b: i64| -> i64 { a.max(b) });

    engine
}

/// Create a Rhai engine with output collection and item efuns (item_create, item_drop, item_get, item_destroy).
/// Used by script commands that need to modify body inventory and room floor.
/// get_other_players_desc: (exclude_name) -> 같은 방 다른 유저 getDesc 목록. 봐 시 사용, None이면 빈 목록.
/// get_other_players_map: () -> (이름→getDesc). 봐 find_target에서 사용, None이면 빈 맵.
pub fn create_engine_with_body_and_output(
    body: &mut Body,
    output_collector: Arc<Mutex<Vec<String>>>,
    get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
    get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
    special_collector: Arc<Mutex<Option<CommandResult>>>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
    script_name: Option<&str>,
) -> Engine {
    let oc = output_collector.clone();
    let mut engine = create_engine_with_output(output_collector);
    let body_ptr = body as *mut Body;
    let spec = special_collector.clone();

    engine.register_fn("item_create", move |_ob: &mut rhai::Map, key: &str| -> String {
        let body = unsafe { &mut *body_ptr };
        if let Some((arc, name)) = object_from_item_json(key) {
            body.object.append(arc);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            name
        } else {
            String::new()
        }
    });

    engine.register_fn("item_drop", move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
        if name.is_empty() {
            return 0; // 빈 name이 "".contains("")로 전부 매칭되는 것 방지
        }
        let body = unsafe { &mut *body_ptr };
        let order = order.max(1) as usize;
        let count = count.max(1).min(100) as usize;
        let mut w = get_world_state().write().unwrap();
        let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
            Some(p) => (p.zone.clone(), p.room),
            None => return 0,
        };
        // 스택 아이템: inv_stack에서 빼서 room_inv_stack으로
        if let Some(ref key) = find_item_key_by_name(name) {
            if is_stackable(key) {
                let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                let drop_cnt = (count as i64).min(have).max(0);
                if drop_cnt > 0 {
                    let should_remove = {
                        let r = body.object.inv_stack.get_mut(key).unwrap();
                        *r -= drop_cnt;
                        *r <= 0
                    };
                    if should_remove {
                        body.object.inv_stack.remove(key);
                    }
                    let room_stack = w.get_room_objs_stack_mut(&zone, room);
                    *room_stack.entry(key.clone()).or_insert(0) += drop_cnt;
                    drop(w);
                    let path = format!("data/user/{}.json", body.get_name());
                    let _ = save_body_to_json(body, &path);
                    return drop_cnt;
                }
            }
        }
        // 비스택: objs에서 제거해 room_objs로
        let mut n = 0usize;
        let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
        for obj in &body.object.objs {
            if let Ok(o) = obj.lock() {
                let ok = o.getName() == name
                    || (!o.getString("반응이름").is_empty() && o.getString("반응이름").contains(name));
                if !ok || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함") {
                    continue;
                }
                if o.checkAttr("아이템속성", "버리지못함") {
                    continue;
                }
                n += 1;
                if n < order {
                    continue;
                }
                drop(o);
                to_remove.push(obj.clone());
                if to_remove.len() >= count {
                    break;
                }
            }
        }
        let dropped = to_remove.len();
        if dropped == 0 {
            return 0;
        }
        let room_objs = w.get_room_objs_mut(&zone, room);
        for arc in to_remove {
            body.object.remove(&arc);
            room_objs.push(arc);
        }
        if dropped > 0 {
            drop(w);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        }
        dropped as i64
    });

    engine.register_fn("item_get", move |_ob: &mut rhai::Map, name: &str, count: i64| -> i64 {
        let body = unsafe { &mut *body_ptr };
        let count = count.max(1).min(100) as usize;
        let mut w = get_world_state().write().unwrap();
        let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
            Some(p) => (p.zone.clone(), p.room),
            None => return 0,
        };
        let mut taken = 0usize;
        // 스택: room_inv_stack에서 가져와 body.inv_stack에
        if let Some(ref key) = find_item_key_by_name(name) {
            if is_stackable(key) {
                let room_stack = w.get_room_objs_stack_mut(&zone, room);
                let have = *room_stack.get(key).unwrap_or(&0);
                let take_cnt = (count as i64).min(have).max(0) as usize;
                if take_cnt > 0 {
                    let should_remove = {
                        let r = room_stack.get_mut(key).unwrap();
                        *r -= take_cnt as i64;
                        *r <= 0
                    };
                    if should_remove {
                        room_stack.remove(key);
                    }
                    *body.object.inv_stack.entry(key.clone()).or_insert(0) += take_cnt as i64;
                    taken += take_cnt;
                }
            }
        }
        // 바닥 Object에서 추가 (비스택 또는 예전 드랍)
        let room_list = w.get_room_objs_mut(&zone, room);
        let mut i = 0;
        while i < room_list.len() && taken < count {
            let matches = {
                let o = room_list[i].lock().unwrap();
                o.getName() == name
                    || (!o.getString("반응이름").is_empty() && o.getString("반응이름").contains(name))
            };
            if matches {
                let arc = room_list.remove(i);
                body.object.append(arc);
                taken += 1;
            } else {
                i += 1;
            }
        }
        if taken > 0 {
            drop(w);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        }
        taken as i64
    });

    engine.register_fn("item_destroy", move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
        let body = unsafe { &mut *body_ptr };
        let order = order.max(1) as usize;
        let count = count.max(1).min(100) as usize;
        // 스택: inv_stack에서 제거
        if order == 1 {
            if let Some(ref key) = find_item_key_by_name(name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    let destroy_cnt = (count as i64).min(have).max(0) as i64;
                    if destroy_cnt > 0 {
                        let should_remove = {
                            let r = body.object.inv_stack.get_mut(key).unwrap();
                            *r -= destroy_cnt;
                            *r <= 0
                        };
                        if should_remove {
                            body.object.inv_stack.remove(key);
                        }
                        let path = format!("data/user/{}.json", body.get_name());
                        let _ = save_body_to_json(body, &path);
                        return destroy_cnt;
                    }
                }
            }
        }
        // 비스택: objs에서 제거
        let mut n = 0usize;
        let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
        for obj in &body.object.objs {
            if let Ok(o) = obj.lock() {
                let ok = o.getName() == name
                    || (!o.getString("반응이름").is_empty() && o.getString("반응이름").contains(name));
                if !ok || o.getBool("inUse") {
                    continue;
                }
                n += 1;
                if n < order {
                    continue;
                }
                drop(o);
                to_remove.push(obj.clone());
                if to_remove.len() >= count {
                    break;
                }
            }
        }
        let len = to_remove.len();
        for arc in to_remove {
            body.object.remove(&arc);
        }
        if len > 0 {
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        }
        len as i64
    });

    // item_destroy_busha: like item_destroy but skips 부수지못함. Returns -1 if first candidate has it.
    engine.register_fn("item_destroy_busha", move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
        let body = unsafe { &mut *body_ptr };
        let order = order.max(1) as usize;
        let count = count.max(1).min(100) as usize;
        let mut n = 0usize;
        let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
        let mut hit_unbreakable = false;
        for obj in &body.object.objs {
            if let Ok(o) = obj.lock() {
                let ok = o.getName() == name
                    || (!o.getString("반응이름").is_empty() && o.getString("반응이름").contains(name));
                if !ok || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함") {
                    continue;
                }
                if o.checkAttr("아이템속성", "부수지못함") {
                    n += 1;
                    if n >= order && to_remove.is_empty() {
                        hit_unbreakable = true;
                    }
                    continue;
                }
                n += 1;
                if n < order {
                    continue;
                }
                drop(o);
                to_remove.push(obj.clone());
                if to_remove.len() >= count {
                    break;
                }
            }
        }
        if hit_unbreakable {
            return -1;
        }
        let len = to_remove.len();
        for arc in to_remove {
            body.object.remove(&arc);
        }
        len as i64
    });

    // list_inventory(ob): body.object.objs를 순회해 [이름, 갯수] 쌍 배열 반환. inUse/출력안함(비관리자) 제외.
    let body_ptr_inv = body_ptr;
    engine.register_fn("list_inventory", move |ob: &mut rhai::Map| -> rhai::Array {
        let admin = ob
            .get("관리자등급")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0i64);
        let body = unsafe { &*body_ptr_inv };
        let mut map: HashMap<String, i64> = HashMap::new();
        for arc in &body.object.objs {
            if let Ok(o) = arc.lock() {
                if o.getBool("inUse") {
                    continue;
                }
                if o.checkAttr("아이템속성", "출력안함") && admin < 1000 {
                    continue;
                }
                let name = o.getName();
                *map.entry(name).or_insert(0) += 1;
            }
        }
        for (key, cnt) in &body.object.inv_stack {
            if let Some((name, _, _, _)) = get_item_info(key) {
                *map.entry(name).or_insert(0) += cnt;
            }
        }
        let mut arr = rhai::Array::new();
        for (k, v) in map {
            let mut pair = rhai::Array::new();
            pair.push(rhai::Dynamic::from(k));
            pair.push(rhai::Dynamic::from(v));
            arr.push(rhai::Dynamic::from(pair));
        }
        arr
    });

    // get_merchant_script(ob): 현재 방의 상인(물건판매) 몹의 물건판매스크립을 "\r\n"으로 이어서 반환. 없으면 "".
    let body_ptr_merchant = body_ptr;
    engine.register_fn("get_merchant_script", move |_ob: &mut rhai::Map| -> String {
        let body = unsafe { &*body_ptr_merchant };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return String::new(),
        };
        let mobs = w.get_mobs_for_player(name.as_str());
        for m in mobs {
            if let Some(data) = w.mob_cache.get_instance_data(m) {
                if !data.items_for_sale.is_empty() && !data.sale_script.is_empty() {
                    return data.sale_script.join("\r\n");
                }
            }
        }
        String::new()
    });

    // get_merchant_buy_percent(ob): 현재 방의 물건구입 상인 몹의 구입 비율(1–100 등). 없으면 0.
    let body_ptr_buy = body_ptr;
    engine.register_fn("get_merchant_buy_percent", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_buy };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return 0,
        };
        let mobs = w.get_mobs_for_player(name.as_str());
        for m in mobs {
            if let Some(data) = w.mob_cache.get_instance_data(m) {
                if data.buy_percent > 0 {
                    return data.buy_percent;
                }
            }
        }
        0
    });

    // merchant_buy(ob, name, count): 상인에게 구입. {err, bought, display_name, total_cost}.
    // 상인 없음/물건 없음/은전/무게/칸 수 검사. 호위·alias 등은 미구현.
    let body_ptr_mbuy = body_ptr;
    engine.register_fn(
        "merchant_buy",
        move |_ob: &mut rhai::Map, name: &str, count: i64| -> Dynamic {
            let mut m = rhai::Map::new();
            let mut err = String::new();
            let mut bought = 0i64;
            let mut display_name = String::new();
            let mut total_cost = 0i64;
            if name.is_empty() {
                m.insert("err".into(), Dynamic::from("☞ 사용법: [물품이름] [수량] 구입".to_string()));
                m.insert("bought".into(), Dynamic::from(0i64));
                m.insert("display_name".into(), Dynamic::from(String::new()));
                m.insert("total_cost".into(), Dynamic::from(0i64));
                return Dynamic::from(m);
            }
            let body = unsafe { &mut *body_ptr_mbuy };
            let pname = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => {
                    m.insert("err".into(), Dynamic::from("☞ 품목을 보여줄 상인이 없어요. ^^".to_string()));
                    m.insert("bought".into(), Dynamic::from(0i64));
                    m.insert("display_name".into(), Dynamic::from(String::new()));
                    m.insert("total_cost".into(), Dynamic::from(0i64));
                    return Dynamic::from(m);
                }
            };
            let mobs = w.get_mobs_for_player(pname.as_str());
            let mut item_key = String::new();
            let mut unit_price = 0i64;
            let mut weight = 0i64;
            for m in mobs {
                let data = match w.mob_cache.get_instance_data(m) {
                    Some(d) if !d.items_for_sale.is_empty() => d,
                    _ => continue,
                };
                for (key, percent) in &data.items_for_sale {
                    let Some((iname, rn, price, wg)) = get_item_info(key) else { continue };
                    let ok = iname == name || (!rn.is_empty() && rn.contains(name));
                    if !ok {
                        continue;
                    }
                    let p = (*percent).max(1);
                    unit_price = price * 100 / p;
                    weight = wg;
                    display_name = iname;
                    item_key = key.clone();
                    break;
                }
                if !item_key.is_empty() {
                    break;
                }
            }
            if item_key.is_empty() {
                err = "☞ 그런 물건은 팔지 않아요. ^_^".to_string();
                m.insert("err".into(), Dynamic::from(err));
                m.insert("bought".into(), Dynamic::from(0i64));
                m.insert("display_name".into(), Dynamic::from(display_name));
                m.insert("total_cost".into(), Dynamic::from(0i64));
                return Dynamic::from(m);
            }
            let cnt = count.max(1).min(50) as i64;
            const MAX_ITEMS: usize = 50;
            for _ in 0..cnt {
                if body.get_item_count() >= MAX_ITEMS {
                    if bought == 0 {
                        err = "☞ 자네가 가질 물품의 한계라네".to_string();
                    }
                    break;
                }
                if body.get_item_weight() + weight > body.get_str() * 10 {
                    if bought == 0 {
                        err = "☞ 무거워서 더 이상 가질 수 없어요. ^^".to_string();
                    }
                    break;
                }
                if body.get_int("은전") < unit_price {
                    if bought == 0 {
                        err = "☞ 돈이 모자라네요. ^^".to_string();
                    }
                    break;
                }
                if is_stackable(&item_key) {
                    *body.object.inv_stack.entry(item_key.clone()).or_insert(0) += 1;
                    body.set("은전", body.get_int("은전") - unit_price);
                    bought += 1;
                    total_cost += unit_price;
                } else if let Some((arc, _)) = object_from_item_json(&item_key) {
                    body.object.append(arc);
                    body.set("은전", body.get_int("은전") - unit_price);
                    bought += 1;
                    total_cost += unit_price;
                } else {
                    break;
                }
            }
            if bought > 0 {
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            m.insert("err".into(), Dynamic::from(err));
            m.insert("bought".into(), Dynamic::from(bought));
            m.insert("display_name".into(), Dynamic::from(display_name.clone()));
            m.insert("total_cost".into(), Dynamic::from(total_cost));
            Dynamic::from(m)
        },
    );

    // item_sell(ob, name, order, count, percent): 소지품을 상인에게 판매.
    // Returns [sold, total, display_name, err] where err is "" or "no_item" or "cant_sell".
    let body_ptr_sell = body_ptr;
    engine.register_fn(
        "item_sell",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64, percent: i64| -> rhai::Array {
            use rhai::Dynamic;
            let body = unsafe { &mut *body_ptr_sell };
            let order = order.max(1) as usize;
            let count = count.max(1).min(100) as usize;
            let percent = percent.max(0);
            // 스택: order==1일 때 inv_stack에서 판매
            if order == 1 {
                if let Some(ref key) = find_item_key_by_name(name) {
                    if is_stackable(key) {
                        let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                        let sell_cnt = (count as i64).min(have).max(0) as i64;
                        if sell_cnt > 0 {
                            if let Some((iname, _, base_price, _)) = get_item_info(key) {
                                let unit = (base_price * percent) / 100;
                                let total = unit * sell_cnt;
                                let should_remove = {
                                    let r = body.object.inv_stack.get_mut(key).unwrap();
                                    *r -= sell_cnt;
                                    *r <= 0
                                };
                                if should_remove {
                                    body.object.inv_stack.remove(key);
                                }
                                body.set("은전", body.get_int("은전") + total);
                                let path = format!("data/user/{}.json", body.get_name());
                                let _ = save_body_to_json(body, &path);
                                let mut arr = rhai::Array::new();
                                arr.push(Dynamic::from(sell_cnt));
                                arr.push(Dynamic::from(total));
                                arr.push(Dynamic::from(iname));
                                arr.push(Dynamic::from(""));
                                return arr;
                            }
                        }
                    }
                }
            }
            let mut n = 0usize;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            let mut total = 0i64;
            let mut display_name = String::new();
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let nm = o.getName();
                    let rn = o.getString("반응이름");
                    let match_ = nm == name || (!rn.is_empty() && rn.contains(name));
                    if !match_
                        || o.getBool("inUse")
                        || o.checkAttr("아이템속성", "출력안함")
                    {
                        continue;
                    }
                    n += 1;
                    if n < order {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "팔지못함") {
                        let mut arr = rhai::Array::new();
                        arr.push(Dynamic::from(0i64));
                        arr.push(Dynamic::from(0i64));
                        arr.push(Dynamic::from(String::new()));
                        arr.push(Dynamic::from("cant_sell".to_string()));
                        return arr;
                    }
                    let price = (o.getInt("판매가격") * percent) / 100;
                    total += price;
                    if display_name.is_empty() {
                        display_name = o.getName();
                    }
                } else {
                    continue;
                }
                to_remove.push(obj.clone());
                if to_remove.len() >= count {
                    break;
                }
            }
            let err = if to_remove.is_empty() {
                "no_item".to_string()
            } else {
                for arc in &to_remove {
                    body.object.remove(arc);
                }
                body.set("은전", body.get_int("은전") + total);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
                String::new()
            };
            let mut arr = rhai::Array::new();
            arr.push(Dynamic::from(to_remove.len() as i64));
            arr.push(Dynamic::from(total));
            arr.push(Dynamic::from(display_name));
            arr.push(Dynamic::from(err));
            arr
        },
    );

    // view_map_data(ob): arg 없는 봐. build_room_lines(ob 이름, 같은 방 다른 유저 getDesc) → output 1회 push.
    let oc_view = oc.clone();
    let get_other = get_other_players_desc;
    engine.register_fn("view_map_data", move |ob: &mut rhai::Map| {
        let name: String = ob
            .get("이름")
            .or_else(|| ob.get("name"))
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let others = get_other.as_ref().map(|f| f(&name)).unwrap_or_default();
        let s = build_room_lines(&name, &others);
        if let Ok(mut out) = oc_view.lock() {
            out.push(s);
        }
    });

    // find_target(ob, line): [대상] 봐. other_player_descs=같은 방 다른 유저 (이름→getDesc). 파이썬 findObjName·반응이름 앞부분 매칭.
    let body_ptr_ft = body_ptr;
    let get_other_map_ft = get_other_players_map.clone();
    engine.register_fn("find_target", move |ob: &mut rhai::Map, line: &str| -> Dynamic {
        let viewer_name: String = ob
            .get("이름")
            .or_else(|| ob.get("name"))
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let world = get_world_state().read().unwrap();
        let other = get_other_map_ft.as_ref().map(|f| f()).unwrap_or_default();
        let (lines, to_target) =
            look_at_target(unsafe { &*body_ptr_ft }, &world, &viewer_name, line, &other);
        let found = !(lines.len() == 1 && lines[0].contains("안광으로는 그런것을 볼수 없다"));
        let mut m = rhai::Map::new();
        m.insert("found".into(), Dynamic::from(found));
        m.insert(
            "lines".into(),
            Dynamic::from(rhai::Array::from_iter(lines.into_iter().map(Dynamic::from))),
        );
        let mut to_map = rhai::Map::new();
        if let Some((n, msg)) = to_target {
            to_map.insert("name".into(), Dynamic::from(n));
            to_map.insert("msg".into(), Dynamic::from(msg));
        } else {
            to_map.insert("name".into(), Dynamic::from(""));
            to_map.insert("msg".into(), Dynamic::from(""));
        }
        m.insert("to_target".into(), Dynamic::from(to_map));
        Dynamic::from(m)
    });

    // get_all_online_players(): 전 접속자 목록. [{"이름","무림별호","성격","레벨초기화","소속"}, ...]. 누구 스크립트용.
    engine.register_fn("get_all_online_players", get_precomputed_all_online);

    // get_my_position(ob) -> {zone, room}. 어디 등.
    let body_ptr_pos = body_ptr;
    engine.register_fn("get_my_position", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &*body_ptr_pos };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Map::new().into(),
        };
        let pos = w.get_player_position(&name);
        let mut m = rhai::Map::new();
        if let Some(p) = pos {
            m.insert("zone".into(), Dynamic::from(p.zone.clone()));
            m.insert("room".into(), Dynamic::from(p.room));
        } else {
            m.insert("zone".into(), Dynamic::from(""));
            m.insert("room".into(), Dynamic::from(0i64));
        }
        Dynamic::from(m)
    });

    // get_room_name(zone, room) -> 방 이름 문자열. 어디 등.
    engine.register_fn("get_room_name", |zone: &str, room: i64| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return format!("{}:{}", zone, room),
        };
        let r = w.room_cache.get_room_cached(zone, &room.to_string());
        match r {
            Some(arc) => {
                let guard = arc.read().unwrap();
                if guard.display_name.is_empty() { guard.name.clone() } else { guard.display_name.clone() }
            }
            None => format!("{}:{}", zone, room),
        }
    });

    // get_equipped(ob) -> [{slot, name}, ...]. 장비 등. inUse이고 계층 있는 것. ITEM_EQUIP_LEVELS 순 정렬.
    let body_ptr_eq = body_ptr;
    engine.register_fn("get_equipped", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_eq };
        let mut pairs: Vec<(String, String)> = Vec::new();
        for arc in &body.object.objs {
            if let Ok(o) = arc.lock() {
                if !o.getBool("inUse") { continue; }
                let slot = o.getString("계층");
                if slot.is_empty() { continue; }
                pairs.push((slot, o.getName()));
            }
        }
        pairs.sort_by_cached_key(|(s, _)| ITEM_EQUIP_LEVELS.iter().position(|&l| l == s.as_str()).unwrap_or(999));
        let mut arr = rhai::Array::new();
        for (slot, name) in pairs {
            let mut m = rhai::Map::new();
            m.insert("slot".into(), Dynamic::from(slot));
            m.insert("name".into(), Dynamic::from(name));
            arr.push(Dynamic::from(m));
        }
        arr
    });

    // get_armor(ob), get_att_power(ob): 장비·점수 등. Body의 방어력/공격력.
    let body_ptr_arm = body_ptr;
    engine.register_fn("get_armor", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_arm };
        body.get_armor() as i64
    });
    let body_ptr_att = body_ptr;
    engine.register_fn("get_att_power", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_att };
        body.get_attack_power() as i64
    });

    // get_item_level_display(slot): 장비 슬롯 표기 문자열. "투구" -> "투    구" 등.
    engine.register_fn("get_item_level_display", |slot: &str| -> String {
        get_item_level_display(slot).to_string()
    });

    // set_act(ob, state): 행동 상태. "서"|0=Stand, "휴식"|2=Rest, "전투"|1=Fight, "이동"|3=Move.
    let body_ptr_act = body_ptr;
    engine.register_fn("set_act", move |_ob: &mut rhai::Map, state: rhai::Dynamic| {
        let body = unsafe { &mut *body_ptr_act };
        let n = if state.is_int() {
            state.as_int().unwrap_or(0)
        } else {
            let s = state.to_string();
            match s.trim() {
                "서" | "stand" => 0,
                "전투" | "fight" => 1,
                "휴식" | "rest" => 2,
                "이동" | "move" => 3,
                _ => 0,
            }
        };
        body.act = crate::player::ActState::from_i32(n as i32);
    });

    // has_room_property(zone, room, prop): 방 맵속성에 prop 포함 여부. 쉬어(쉼금지) 등.
    engine.register_fn("has_room_property", |zone: &str, room: i64, prop: &str| -> bool {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return false,
        };
        if let Some(arc) = w.room_cache.get_room_cached(zone, &room.to_string()) {
            if let Ok(r) = arc.read() {
                return r.properties.iter().any(|p| p == prop);
            }
        }
        false
    });

    // get_exits_string(zone, room): 출구 나침반 문자열. 지도/맵 등.
    engine.register_fn("get_exits_string", |zone: &str, room: i64| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return String::new(),
        };
        if let Some(arc) = w.room_cache.get_room_cached(zone, &room.to_string()) {
            if let Ok(r) = arc.read() {
                return format_exits_long(&*r);
            }
        }
        String::new()
    });

    // parse_room_spec(s): "존:방번호" 파싱 → {zone, room}. 이동 등.
    engine.register_fn("parse_room_spec", |s: &str| -> Dynamic {
        let mut m = rhai::Map::new();
        let parts: Vec<&str> = s.trim().splitn(2, ':').collect();
        if parts.len() < 2 {
            m.insert("zone".into(), Dynamic::from(String::new()));
            m.insert("room".into(), Dynamic::from(0i64));
            return Dynamic::from(m);
        }
        let zone = parts[0].trim().to_string();
        let room = parts[1].trim().parse::<i64>().unwrap_or(0);
        m.insert("zone".into(), Dynamic::from(zone));
        m.insert("room".into(), Dynamic::from(room));
        Dynamic::from(m)
    });

    // get_position_of(player_name): 해당 플레이어의 {zone, room}. 없으면 {zone:"", room:0}. 앞(소환) 등.
    engine.register_fn("get_position_of", |name: &str| -> Dynamic {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Map::new().into(),
        };
        let mut m = rhai::Map::new();
        if let Some(p) = w.get_player_position(name) {
            m.insert("zone".into(), Dynamic::from(p.zone.clone()));
            m.insert("room".into(), Dynamic::from(p.room));
        } else {
            m.insert("zone".into(), Dynamic::from(String::new()));
            m.insert("room".into(), Dynamic::from(0i64));
        }
        Dynamic::from(m)
    });

    // set_my_position(ob, zone, room): 관리자 순간이동. ""=성공, else 오류. 호출 후 view_map_data(ob) 권장.
    let body_ptr_setpos = body_ptr;
    engine.register_fn("set_my_position", move |_ob: &mut rhai::Map, zone: &str, room: i64| -> String {
        let body = unsafe { &*body_ptr_setpos };
        let name = body.get_name();
        if name.is_empty() {
            return "* 이동 실패!!!".to_string();
        }
        let mut w = match get_world_state().write() {
            Ok(g) => g,
            Err(_) => return "* 이동 실패!!!".to_string(),
        };
        let cur = w.get_player_position(&name).cloned();
        let (cz, cr) = cur.as_ref().map(|p| (p.zone.as_str(), p.room)).unwrap_or(("", 0));
        if cz == zone && cr == room {
            return "☞ 같은 자리에요. ^^".to_string();
        }
        if w.room_cache.get_room(zone, &room.to_string()).is_err() {
            return "* 이동 실패!!!".to_string();
        }
        w.set_player_position(&name, PlayerPosition::new(zone.to_string(), room));
        w.spawn_mobs_for_room(zone, room);
        String::new()
    });

    // set_value(ob, key, val): Body에 키-값 저장. 점프(cooltime) 등. val은 정수 또는 문자열.
    let body_ptr_setv = body_ptr;
    engine.register_fn("set_value", move |_ob: &mut rhai::Map, key: &str, val: rhai::Dynamic| {
        let body = unsafe { &mut *body_ptr_setv };
        if val.is_int() {
            body.set(key, val.as_int().unwrap_or(0));
        } else {
            body.set(key, val.to_string());
        }
    });

    // password_hash(plain): 평문을 SHA-512 해시 16진수 문자열로. 암호 저장/암호변경용.
    engine.register_fn("password_hash", |plain: &str| -> String { password_hash(plain) });
    // password_verify(stored, plain): 저장된 해시(또는 레거시 평문)와 평문 일치 여부. 암호변경 검증용.
    engine.register_fn("password_verify", |stored: &str, plain: &str| -> bool { password_verify(stored, plain) });
    // verify_password(ob, plain): Body 암호와 평문 일치 여부. 해시를 스크립트에 노출하지 않고 검증.
    let body_ptr_vp = body_ptr;
    engine.register_fn("verify_password", move |_ob: &mut rhai::Map, plain: &str| -> bool {
        let body = unsafe { &*body_ptr_vp };
        let stored = body.get_string("암호");
        password_verify(&stored, plain)
    });
    // parse_two_args(s): 첫 공백 기준 [앞, 뒤]. "a b c" -> ["a","b c"]. "a" -> ["a",""].
    engine.register_fn("parse_two_args", |s: &str| -> rhai::Array {
        let parts: Vec<&str> = s.splitn(2, ' ').collect();
        let mut a = rhai::Array::new();
        a.push(rhai::Dynamic::from(parts.get(0).unwrap_or(&"").to_string()));
        a.push(rhai::Dynamic::from(parts.get(1).unwrap_or(&"").to_string()));
        a
    });

    // get_body_int(ob, key): Body에서 정수 읽기. Map에 없는 런타임 키(예: cooltime)용.
    let body_ptr_getbi = body_ptr;
    engine.register_fn("get_body_int", move |_ob: &mut rhai::Map, key: &str| -> i64 {
        let body = unsafe { &*body_ptr_getbi };
        body.get_int(key)
    });

    // ---- 외쳐/전음/표현/주다: special_collector에 CommandResult 설정, handler에서 Shout/Tell/EmotionToRoom/GiveToPlayer 처리 ----

    // send_shout(ob, msg): 외쳐. 검사 후 Shout(formatted) 설정. 오류 시 oc에 push.
    let oc_sh = oc.clone();
    let spec_sh = spec.clone();
    let body_ptr_sh = body_ptr;
    engine.register_fn("send_shout", move |_ob: &mut rhai::Map, msg: &str| {
        let body = unsafe { &*body_ptr_sh };
        if msg.trim().is_empty() {
            if let Ok(mut o) = oc_sh.lock() {
                o.push("☞ 사용법: [내용] 외침(,)".to_string());
            }
            return;
        }
        if msg.len() > 160 {
            if let Ok(mut o) = oc_sh.lock() {
                o.push("☞ 너무 길어요. ^^".to_string());
            }
            return;
        }
        let config = body.get_string("설정상태");
        if config.contains("외침거부 1") {
            if let Ok(mut o) = oc_sh.lock() {
                o.push("☞ 외침거부중엔 외칠 수 없어요. ^^".to_string());
            }
            return;
        }
        if body.act == crate::player::ActState::Rest {
            if let Ok(mut o) = oc_sh.lock() {
                o.push("☞ 운기조식중에 외치게 되면 기가 흐트러집니다.".to_string());
            }
            return;
        }
        let name = body.get_name();
        let adm = body.get_int("관리자등급");
        let personality = body.get_string("성격");
        let shout_type = if adm >= 2000 {
            "\x1b[0;35m사자후\x1b[0;37m"
        } else if personality == "선인" {
            "\x1b[1;36m창룡후\x1b[0;37m"
        } else if personality == "기인" {
            "\x1b[1;32m사자후\x1b[0;37m"
        } else {
            "\x1b[32m외 침\x1b[37m"
        };
        let formatted = format!("{}({}) : {}", name, shout_type, msg);
        if let Ok(mut s) = spec_sh.lock() {
            *s = Some(CommandResult::Shout(formatted));
        }
    });

    // send_tell(ob, target, msg): 전음. 오류 시 oc에 push. self면 "중얼 중얼" push.
    let oc_te = oc.clone();
    let spec_te = spec.clone();
    let body_ptr_te = body_ptr;
    engine.register_fn("send_tell", move |_ob: &mut rhai::Map, target: &str, msg: &str| {
        let body = unsafe { &*body_ptr_te };
        if target.trim().is_empty() || msg.trim().is_empty() {
            if let Ok(mut o) = oc_te.lock() {
                o.push("☞ 사용법: [대상] [내용] 전음(/)".to_string());
            }
            return;
        }
        let config = body.get_string("설정상태");
        if config.contains("전음거부 1") {
            if let Ok(mut o) = oc_te.lock() {
                o.push("☞ 전음 거부중이에요. ^^".to_string());
            }
            return;
        }
        if target == body.get_name() {
            if let Ok(mut o) = oc_te.lock() {
                o.push("중얼 중얼 거립니다.".to_string());
            }
            return;
        }
        if let Ok(mut s) = spec_te.lock() {
            *s = Some(CommandResult::Tell(target.to_string(), msg.to_string()));
        }
    });

    // send_emotion(ob, action): 표현. to_self="당신이 {action}", to_room="{name}{iga} {action}".
    let oc_em = oc.clone();
    let spec_em = spec.clone();
    let body_ptr_em = body_ptr;
    engine.register_fn("send_emotion", move |_ob: &mut rhai::Map, action: &str| {
        let body = unsafe { &*body_ptr_em };
        if action.trim().is_empty() {
            if let Ok(mut o) = oc_em.lock() {
                o.push("☞ 사용법: 표현 [동작] 또는 ' [동작]".to_string());
            }
            return;
        }
        let name = body.get_name();
        let iga = han_iga(&name);
        let to_self = format!("당신이 {}", action);
        let to_room = format!("{}{} {}", name, iga, action);
        if let Ok(mut s) = spec_em.lock() {
            *s = Some(CommandResult::EmotionToRoom(to_self, to_room, None));
        }
    });

    // request_give_silver(ob, target, amt): 주다 은전. ""=성공(Shout설정), else 오류.
    let spec_gs = spec.clone();
    let body_ptr_gs = body_ptr;
    engine.register_fn("request_give_silver", move |_ob: &mut rhai::Map, target: &str, amt: i64| -> String {
        let body = unsafe { &*body_ptr_gs };
        if amt < 1 {
            return "☞ 사용법: [대상] [물품] [개수] 주다".to_string();
        }
        let have = body.get_int("은전");
        let give = amt.min(have.max(0));
        if give < 1 {
            return "☞ 돈이 모자라네요. ^^".to_string();
        }
        if let Ok(mut s) = spec_gs.lock() {
            *s = Some(CommandResult::GiveToPlayer {
                target_name: target.to_string(),
                giver_name: body.get_name(),
                give_silver: Some(give),
                give_gold: None,
                give_item: None,
                give_item_stack: None,
            });
        }
        String::new()
    });

    // request_give_gold(ob, target, amt): 주다 금전.
    let spec_gg = spec.clone();
    let body_ptr_gg = body_ptr;
    engine.register_fn("request_give_gold", move |_ob: &mut rhai::Map, target: &str, amt: i64| -> String {
        let body = unsafe { &*body_ptr_gg };
        if amt < 1 {
            return "☞ 사용법: [대상] [물품] [개수] 주다".to_string();
        }
        let have = body.get_int("금전");
        let give = amt.min(have.max(0));
        if give < 1 {
            return "☞ 돈이 모자라네요. ^^".to_string();
        }
        if let Ok(mut s) = spec_gg.lock() {
            *s = Some(CommandResult::GiveToPlayer {
                target_name: target.to_string(),
                giver_name: body.get_name(),
                give_silver: None,
                give_gold: Some(give),
                give_item: None,
                give_item_stack: None,
            });
        }
        String::new()
    });

    // request_give_item(ob, target, name, order, count): 주다 아이템. 검사 후 GiveToPlayer. 스택이면 give_item_stack.
    let spec_gi = spec.clone();
    let body_ptr_gi = body_ptr;
    engine.register_fn("request_give_item", move |_ob: &mut rhai::Map, target: &str, item_name: &str, order: i64, count: i64| -> String {
        let body = unsafe { &*body_ptr_gi };
        let order = order.max(1) as usize;
        let cnt = if order > 1 { 1i64 } else { count.max(1).min(50) };
        // 스택: inv_stack에서
        if let Some(ref key) = find_item_key_by_name(item_name) {
            if is_stackable(key) {
                let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                if have >= cnt {
                    if let Ok(mut s) = spec_gi.lock() {
                        *s = Some(CommandResult::GiveToPlayer {
                            target_name: target.to_string(),
                            giver_name: body.get_name(),
                            give_silver: None,
                            give_gold: None,
                            give_item: None,
                            give_item_stack: Some((key.clone(), cnt)),
                        });
                    }
                    return String::new();
                }
            }
        }
        // 비스택: findObjInven
        let cnt_u = cnt as usize;
        if body.object.findObjInven(item_name, order).is_none() {
            return "☞ 그런 아이템이 소지품에 없어요.".to_string();
        }
        if let Ok(mut s) = spec_gi.lock() {
            *s = Some(CommandResult::GiveToPlayer {
                target_name: target.to_string(),
                giver_name: body.get_name(),
                give_silver: None,
                give_gold: None,
                give_item: Some((item_name.to_string(), order, cnt_u)),
                give_item_stack: None,
            });
        }
        String::new()
    });

    // item_equip(ob, name, order): 장비 착용. ""=성공, else 오류문자열. 계층 중복·종류(방어구/무기) 검사.
    let body_ptr_equip = body_ptr;
    engine.register_fn("item_equip", move |_ob: &mut rhai::Map, name: &str, order: i64| -> String {
        if name.is_empty() { return "☞ 사용법: [아이템 이름] 입어".to_string(); }
        let order = order.max(1) as usize;
        let body = unsafe { &mut *body_ptr_equip };
        let arc = match body.object.findObjInven(name, order) {
            Some(a) => a,
            None => return "☞ 그런 아이템이 소지품에 없어요.".to_string(),
        };
        let (kind, slot, arm, att) = {
            let o = arc.lock().unwrap();
            let k = o.getString("종류");
            let s = o.getString("계층");
            if k != "방어구" && k != "무기" {
                return "☞ 착용할 수 있는것이 아니에요.".to_string();
            }
            (k, s, o.getInt("방어력") as i32, o.getInt("공격력") as i32)
        };
        let slot_used = body.object.objs.iter().any(|obj| {
            if std::sync::Arc::ptr_eq(obj, &arc) { return false; }
            obj.lock().map(|x| x.getBool("inUse") && x.getString("계층") == slot).unwrap_or(false)
        });
        if slot_used && !slot.is_empty() {
            return "☞ 더 이상 착용이 불가능해요.".to_string();
        }
        { let mut o = arc.lock().unwrap(); o.set("inUse", 1i64); }
        body.armor += arm;
        body.attpower += att;
        if kind == "무기" {
            body.weapon_item = Some(std::sync::Arc::downgrade(&arc));
        }
        String::new()
    });

    // item_unequip(ob, name, order): 장비 해제. ""=성공, else 오류. order 1-based.
    let body_ptr_ue = body_ptr;
    engine.register_fn("item_unequip", move |_ob: &mut rhai::Map, name: &str, order: i64| -> String {
        if name.is_empty() { return "☞ 사용법: [아이템 이름] 벗어  또는  [모두/전부] 벗어".to_string(); }
        let order = order.max(1) as usize;
        let body = unsafe { &mut *body_ptr_ue };
        let arc = match body.object.findObjInUse(name, order) {
            Some(a) => a,
            None => return "☞ 그런 아이템이 소지품에 없어요.".to_string(),
        };
        let (arm, att, is_weapon) = {
            let mut o = arc.lock().unwrap();
            o.set("inUse", 0i64);
            let arm = o.getInt("방어력") as i32;
            let att = o.getInt("공격력") as i32;
            let w = o.getString("종류") == "무기";
            (arm, att, w)
        };
        body.armor -= arm;
        body.attpower -= att;
        if body.attpower < 0 { body.attpower = 0; }
        if body.armor < 0 { body.armor = 0; }
        if is_weapon { body.weapon_item = None; }
        String::new()
    });

    // item_unequip_all(ob): 착용 중인 전부 해제. 해제한 개수 반환.
    let body_ptr_ua = body_ptr;
    engine.register_fn("item_unequip_all", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &mut *body_ptr_ua };
        let n = body.object.objs.iter()
            .filter(|o| o.lock().map(|x| x.getBool("inUse")).unwrap_or(false))
            .count();
        body.unwear_all();
        n as i64
    });

    // item_use_consumable(ob, name, order): 소모품 사용(먹어). {err: "", name: "아이템이름"} 또는 {err: "오류", name: ""}.
    let body_ptr_cons = body_ptr;
    engine.register_fn("item_use_consumable", move |_ob: &mut rhai::Map, name: &str, order: i64| -> Dynamic {
        let mut m = rhai::Map::new();
        if name.is_empty() {
            m.insert("err".into(), Dynamic::from("☞ 사용법: [아이템 이름] 먹어".to_string()));
            m.insert("name".into(), Dynamic::from(String::new()));
            return Dynamic::from(m);
        }
        let order = order.max(1) as usize;
        let body = unsafe { &mut *body_ptr_cons };
        if body.act == crate::player::ActState::Rest {
            m.insert("err".into(), Dynamic::from("☞ 먹을 수 있는 상황이 아니네요. ^_^".to_string()));
            m.insert("name".into(), Dynamic::from(String::new()));
            return Dynamic::from(m);
        }
        let arc = match body.object.findObjInven(name, order) {
            Some(a) => a,
            None => {
                m.insert("err".into(), Dynamic::from("☞ 그런 아이템이 소지품에 없어요.".to_string()));
                m.insert("name".into(), Dynamic::from(String::new()));
                return Dynamic::from(m);
            }
        };
        let (item_name, hp, mp) = {
            let o = arc.lock().unwrap();
            if o.getString("종류") != "먹는것" {
                m.insert("err".into(), Dynamic::from("☞ 먹을 수 있는것이 아니에요. ^_^".to_string()));
                m.insert("name".into(), Dynamic::from(String::new()));
                return Dynamic::from(m);
            }
            (o.getName(), o.getInt("체력"), o.getInt("내공"))
        };
        let max_hp = body.get_max_hp();
        let max_mp = body.get_max_mp();
        let cur_hp = body.get_hp();
        let cur_mp = body.get_mp();
        let new_hp = (cur_hp + hp).min(max_hp).max(0);
        let new_mp = (cur_mp + mp).min(max_mp).max(0);
        body.set("체력", new_hp);
        body.set("내공", new_mp);
        body.object.objs.retain(|x| !std::sync::Arc::ptr_eq(x, &arc));
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);
        m.insert("err".into(), Dynamic::from(String::new()));
        m.insert("name".into(), Dynamic::from(item_name));
        Dynamic::from(m)
    });

    // send_to_player(name, msg): 당신을 살펴봅니다 전송. 당분간 no-op (broadcaster 미전달).
    engine.register_fn("send_to_player", |_name: &str, _msg: &str| {});

    // body_save(ob): 캐릭터 저장. data/user/{이름}.json 에 저장.
    engine.register_fn("body_save", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr };
        let path = format!("data/user/{}.json", body.get_name());
        save_body_to_json(body, &path)
    });

    // call_out / call_later / remove_call_out — 점프 2초 후 착지 등. script_name이 있을 때만 등록(지연 시 스크립트 함수 실행).
    if let (Some(sched), Some(sn)) = (call_out_scheduler, script_name) {
        let s = sched.clone();
        let script_owned = sn.to_string();
        engine.register_fn("call_out", move |target: &str, function: &str, delay: i64| {
            let d = Duration::from_secs(delay.max(0) as u64);
            s.call_out(target, function, d, vec![], Some(script_owned.clone()));
        });
        let s2 = sched.clone();
        let script_owned2 = sn.to_string();
        engine.register_fn("call_later", move |target: &str, function: &str, delay: i64| {
            let d = Duration::from_secs(delay.max(0) as u64);
            s2.call_out(target, function, d, vec![], Some(script_owned2.clone()));
        });
        let s3 = sched.clone();
        engine.register_fn("remove_call_out", move |target: &str, function: &str| -> bool {
            s3.remove_call_out_by_name(target, function)
        });
    }

    engine
}

/// Create a new Rhai engine with global data access
///
/// 글로벌 데이터 캐시에 접근할 수 있는 efuns을 등록합니다.
pub fn create_engine_with_global_data(global_data: SharedGlobalData) -> Engine {
    let mut engine = create_engine();

    // 글로벌 데이터를 clone하여 캡처
    let gd = global_data.clone();

    // ============================================================
    // GLOBAL DATA ACCESS FUNCTIONS
    // ============================================================

    // get_global(file) - 전체 파일 데이터 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_global", move |file: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get(file) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_global_key(file, key) - 파일에서 특정 키의 데이터 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_global_key", move |file: &str, key: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_path(file, key) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_skill(name) - 스킬 데이터 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_skill", move |name: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_skill(name) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_murim_config(key) - 무림 설정 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_murim_config", move |key: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_murim_config(key) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_map_path(zone) - 맵 경로 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_map_path", move |zone: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_map_path(zone) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // has_global(file) - 파일 존재 확인
    let gd_clone = global_data.clone();
    engine.register_fn("has_global", move |file: &str| -> bool {
        if let Ok(data) = gd_clone.try_read() {
            data.contains(file)
        } else {
            false
        }
    });

    // has_global_key(file, key) - 파일의 키 존재 확인
    let gd_clone = global_data.clone();
    engine.register_fn("has_global_key", move |file: &str, key: &str| -> bool {
        if let Ok(data) = gd_clone.try_read() {
            data.contains_key(file, key)
        } else {
            false
        }
    });

    // get_global_keys(file) - 파일의 모든 키 목록
    let gd_clone = global_data.clone();
    engine.register_fn("get_global_keys", move |file: &str| -> rhai::Array {
        if let Ok(data) = gd_clone.try_read() {
            let keys: rhai::Array = data.keys(file)
                .into_iter()
                .map(Dynamic::from)
                .collect();
            keys
        } else {
            rhai::Array::new()
        }
    });

    // list_globals() - 모든 파일 이름 목록
    let gd_clone = global_data.clone();
    engine.register_fn("list_globals", move || -> rhai::Array {
        if let Ok(data) = gd_clone.try_read() {
            let names: rhai::Array = data.file_names()
                .into_iter()
                .map(Dynamic::from)
                .collect();
            names
        } else {
            rhai::Array::new()
        }
    });

    // reload_global(file) - 특정 파일 다시 로드
    let gd_clone = global_data.clone();
    engine.register_fn("reload_global", move |file: &str| -> bool {
        if let Ok(mut data) = gd_clone.try_write() {
            data.reload(file).unwrap_or(false)
        } else {
            false
        }
    });

    // reload_all_globals() - 모든 파일 다시 로드
    let gd_clone = global_data.clone();
    engine.register_fn("reload_all_globals", move || -> i64 {
        if let Ok(mut data) = gd_clone.try_write() {
            data.reload_all().unwrap_or(0) as i64
        } else {
            0
        }
    });

    engine
}

/// Convert serde_json::Value to Rhai Dynamic
/// 내부적으로 data 모듈의 json_to_dynamic를 사용합니다.
fn json_value_to_dynamic(value: serde_json::Value) -> Dynamic {
    crate::data::json_to_dynamic(&value)
}

/// Script storage - stores raw script source code
pub struct ScriptStorage {
    scripts: HashMap<String, StoredScript>,
    config: ScriptConfig,
    /// 글로벌 데이터 캐시 참조
    global_data: Option<SharedGlobalData>,
}

unsafe impl Send for ScriptStorage {}
unsafe impl Sync for ScriptStorage {}

impl ScriptStorage {
    pub fn new(config: ScriptConfig) -> Self {
        let mut storage = Self {
            scripts: HashMap::new(),
            config,
            global_data: None,
        };
        storage.load_all_scripts().ok();
        storage
    }

    /// 글로벌 데이터 캐시와 함께 생성합니다.
    pub fn with_global_data(config: ScriptConfig, global_data: SharedGlobalData) -> Self {
        let mut storage = Self {
            scripts: HashMap::new(),
            config,
            global_data: Some(global_data),
        };
        storage.load_all_scripts().ok();
        storage
    }

    pub fn default() -> Self {
        Self::new(ScriptConfig::default())
    }

    /// 글로벌 데이터 캐시를 설정합니다.
    pub fn set_global_data(&mut self, global_data: SharedGlobalData) {
        self.global_data = Some(global_data);
    }

    /// 글로벌 데이터 캐시를 가져옵니다.
    pub fn get_global_data(&self) -> Option<SharedGlobalData> {
        self.global_data.clone()
    }

    pub fn load_all_scripts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = self.config.script_dir.clone();
        if !dir.exists() {
            info!("Creating script directory: {:?}", dir);
            std::fs::create_dir_all(&dir)?;
            return Ok(());
        }

        let entries = std::fs::read_dir(&dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rhai") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                self.load_script(&name, &path)?;
            }
        }

        info!("Loaded {} scripts from {:?}", self.scripts.len(), dir);
        Ok(())
    }

    pub fn load_script(&mut self, name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;
        let source = std::fs::read_to_string(path)?;
        self.scripts.insert(name.to_string(), StoredScript {
            source,
            modified,
            name: name.to_string(),
        });
        debug!("Loaded script: {} from {:?}", name, path);
        Ok(())
    }

    pub fn reload_script(&mut self, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let script_path = self.config.script_dir.join(format!("{}.rhai", name));
        if !script_path.exists() {
            return Ok(false);
        }

        let metadata = std::fs::metadata(&script_path)?;
        let modified = metadata.modified()?;

        if let Some(script) = self.scripts.get(name) {
            if modified <= script.modified {
                return Ok(false);
            }
        }

        let source = std::fs::read_to_string(&script_path)?;
        self.scripts.insert(name.to_string(), StoredScript {
            source,
            modified,
            name: name.to_string(),
        });

        info!("Reloaded script: {}", name);
        Ok(true)
    }

    pub fn reload_all(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut reloaded = 0;
        let names: Vec<String> = self.scripts.keys().cloned().collect();
        for name in names {
            if self.reload_script(&name)? {
                reloaded += 1;
            }
        }
        Ok(reloaded)
    }

    /// get_other_players_desc: 봐 시 같은 방 다른 유저 getDesc. None이면 빈 목록.
    /// get_other_players_map: 봐 find_target에서 (이름→getDesc). None이면 빈 맵.
    /// call_out_scheduler: Some이면 call_out/call_later 사용 가능(지연 시 스크립트 함수 실행).
    /// Returns (outputs, special). special=Some(CommandResult)이면 Shout/Tell/EmotionToRoom/GiveToPlayer 등.
    pub fn execute(
        &self,
        name: &str,
        player: &mut Body,
        line: &str,
        get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
        get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
        call_out_scheduler: Option<Arc<CallOutScheduler>>,
    ) -> Result<(Vec<String>, Option<CommandResult>), Box<dyn std::error::Error>> {
        let script = self.scripts.get(name)
            .ok_or_else(|| format!("Script not found: {}", name))?;

        let output_collector = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output_collector.clone();
        let special_collector = Arc::new(Mutex::new(None));

        let engine = create_engine_with_body_and_output(
            player,
            output_clone,
            get_other_players_desc,
            get_other_players_map,
            special_collector.clone(),
            call_out_scheduler,
            Some(name),
        );
        let mut scope = Scope::new();

        let mut player_data = build_ob_from_body(player);
        scope.push("player", player_data.clone());
        scope.push("me", player_data.clone());
        scope.push("ob", player_data);
        scope.push("cmdline", rhai::Dynamic::from(line.to_string()));

        let script_with_main = format!("{}\nmain(ob, cmdline)", script.source);
        engine.run_with_scope(&mut scope, &script_with_main)
            .map_err(|e| format!("스크립트 실행 오류: {}", e))?;

        let outputs = output_collector.lock().unwrap().clone();
        let expanded: Vec<String> = outputs
            .into_iter()
            .map(|s| expand_abbreviated_ansi(&s))
            .collect();
        let special = special_collector.lock().unwrap().take();
        Ok((expanded, special))
    }

    pub fn execute_with_scope(
        &self,
        name: &str,
        scope: &mut Scope,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let script = self.scripts.get(name)
            .ok_or_else(|| format!("Script not found: {}", name))?;
        let engine = create_engine();
        engine.run_with_scope(scope, &script.source)?;
        Ok(())
    }

    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }

    pub fn script_names(&self) -> Vec<String> {
        self.scripts.keys().cloned().collect()
    }

    /// Get script source by name. For call_out script_runner to run a function from the script.
    pub fn get_script_source(&self, name: &str) -> Option<String> {
        self.scripts.get(name).map(|s| s.source.clone())
    }
}

/// Body로부터 Rhai ob(Map) 생성. execute 및 call_out 콜백에서 사용.
fn build_ob_from_body(body: &Body) -> rhai::Map {
    let mut m = rhai::Map::new();
    m.insert("name".into(), body.get_name().into());
    m.insert("hp".into(), body.get_hp().into());
    m.insert("max_hp".into(), body.get_max_hp().into());
    m.insert("mp".into(), body.get_mp().into());
    m.insert("max_mp".into(), body.get_max_mp().into());
    m.insert("level".into(), body.get_int("레벨").into());
    m.insert("레벨".into(), body.get_int("레벨").into());
    m.insert("나이".into(), body.get_int("나이").into());
    m.insert("맷집".into(), body.get_int("맷집").into());
    m.insert("현재경험치".into(), body.get_int("현재경험치").into());
    m.insert("money".into(), body.get_int("은전").into());
    m.insert("은전".into(), body.get_int("은전").into());
    m.insert("금전".into(), body.get_int("금전").into());
    m.insert("str".into(), body.get_str().into());
    m.insert("dex".into(), body.get_dex().into());
    m.insert("이름".into(), body.get_name().into());
    m.insert("관리자등급".into(), body.get_int("관리자등급").into());
    m.insert("act".into(), (body.act.to_i32() as i64).into());
    m.insert("성격".into(), body.get_string("성격").into());
    m.insert("소속".into(), body.get_string("소속").into());
    m.insert("env".into(), "".into());
    m.insert("objs".into(), rhai::Dynamic::from(rhai::Array::new()));
    m
}

/// call_out 만료 시 Rhai 스크립트 함수를 실행하는 runner 생성.
/// (target, script, function, args) -> Result. process_due에서 호출.
pub fn create_call_out_script_runner(
    script_storage: Arc<tokio::sync::RwLock<ScriptStorage>>,
    broadcaster: Arc<Broadcaster>,
) -> Arc<dyn Fn(&str, Option<&str>, &str, Vec<serde_json::Value>) -> Result<(), String> + Send + Sync> {
    Arc::new(move |target: &str, script: Option<&str>, function: &str, _args: Vec<serde_json::Value>| {
        let script = script.ok_or_else(|| "call_out: script name required".to_string())?;
        // process_due는 tokio 워커에서 호출되므로 blocking_read 전에 block_in_place로 블로킹 허용
        let source = tokio::task::block_in_place(|| {
            script_storage.blocking_read().get_script_source(script)
        });
        let source = source.ok_or_else(|| format!("script not found: {}", script))?;

        // 클로저 안에서는 clients 락이 잡혀 있으므로 send_to_by_player_name(→clients.lock()) 호출 금지.
        // 메시지만 수집하고, 락 해제 후 밖에서 전송.
        let to_send = broadcaster
            .with_player_body_by_name(target, |body| {
                let output_collector = Arc::new(Mutex::new(Vec::new()));
                let special_collector = Arc::new(Mutex::new(None));
                let engine = create_engine_with_body_and_output(
                    body,
                    output_collector.clone(),
                    None,
                    None,
                    special_collector,
                    None,
                    None,
                );
                let ast = engine.compile(&source).map_err(|e| format!("compile: {}", e))?;
                let mut scope = Scope::new();
                let ob = Dynamic::from(build_ob_from_body(body));
                engine
                    .call_fn::<Dynamic>(&mut scope, &ast, function, (ob,))
                    .map_err(|e| format!("call_fn {}: {}", function, e))?;

                let outputs = output_collector.lock().unwrap().clone();
                let messages: Vec<String> = outputs
                    .iter()
                    .map(|line| {
                        let expanded = expand_abbreviated_ansi(line);
                        format!("{}\r\n", expanded)
                    })
                    .collect();
                Ok::<_, String>(messages)
            })
            .ok_or_else(|| format!("player not found: {}", target))?;

        let messages = to_send?;
        for msg in messages {
            let _ = broadcaster.send_to_by_player_name(target, &msg);
        }
        Ok(())
    })
}

pub struct SharedScriptStorage {
    inner: Arc<RwLock<ScriptStorage>>,
}

impl SharedScriptStorage {
    pub fn new(config: ScriptConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ScriptStorage::new(config))),
        }
    }

    pub fn default() -> Self {
        Self::new(ScriptConfig::default())
    }

    pub async fn execute(&self, name: &str, player: &mut Body, line: &str, get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>, get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>, call_out_scheduler: Option<Arc<CallOutScheduler>>) -> Result<(Vec<String>, Option<CommandResult>), Box<dyn std::error::Error>> {
        let storage = self.inner.read().await;
        storage.execute(name, player, line, get_other_players_desc, get_other_players_map, call_out_scheduler)
    }

    pub async fn reload_all(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut storage = self.inner.write().await;
        storage.reload_all()
    }

    pub async fn has_script(&self, name: &str) -> bool {
        let storage = self.inner.read().await;
        storage.has_script(name)
    }

    pub async fn script_names(&self) -> Vec<String> {
        let storage = self.inner.read().await;
        storage.script_names()
    }
}

pub type ScriptEngine = ScriptStorage;
pub type SharedScriptEngine = SharedScriptStorage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_config_default() {
        let config = ScriptConfig::default();
        assert_eq!(config.script_dir, PathBuf::from("cmds"));
        assert!(config.hot_reload);
        assert_eq!(config.extension, ".rhai");
    }

    #[test]
    fn test_script_storage_new() {
        let storage = ScriptStorage::default();
        assert!(storage.config.script_dir.ends_with("cmds"));
    }

    #[test]
    fn test_has_script() {
        let storage = ScriptStorage::default();
        assert!(!storage.has_script("nonexistent"));
    }

    #[test]
    fn test_ansi_convert() {
        let result = ansi_convert("{밝}hello{어}", true);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("\x1b[0m"));

        let result = ansi_convert("{밝}hello{어}", false);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_han_iga() {
        assert_eq!(han_iga("사과"), "가");
        assert_eq!(han_iga("검"), "이");
    }

    #[tokio::test]
    async fn test_shared_storage() {
        let shared = SharedScriptStorage::new(ScriptConfig::default());
        let storage = shared.inner.read().await;
        assert!(storage.config.script_dir.ends_with("cmds"));
    }

    #[test]
    fn test_item_commands_create_drop_get_destroy() {
        use crate::player::Body;
        use crate::world::{get_world_state, PlayerPosition};

        let mut body = Body::new();
        body.set("이름", "item_test_player");
        body.set("관리자등급", 2000i64);

        // 플레이어 위치를 낙양성:1로 설정 (버리기/가져오기에 필요)
        {
            let mut w = get_world_state().write().unwrap();
            w.set_player_position("item_test_player", PlayerPosition::new("낙양성".to_string(), 1));
        }

        let storage = ScriptStorage::default();
        if !storage.has_script("생성") {
            return; // cmds/생성.rhai가 없으면 스킵
        }

        // data/item/289.json 필요 (cargo test 시 cwd=프로젝트 루트)
        if !std::path::Path::new("data/item/289.json").exists() {
            return; // 데이터 없으면 스킵
        }

        // 1) 생성 289 (data/item/289.json = 철퇴)
        let res = storage.execute("생성", &mut body, "289", None, None, None);
        assert!(res.is_ok(), "생성 실패: {:?}", res.err());
        let (out, _) = res.as_ref().unwrap();
        assert_eq!(body.object.objs.len(), 1, "생성 후 인벤 1개 (outputs: {:?})", out);
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "철퇴");

        // 2) 버리기 철퇴
        let res = storage.execute("버려", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "버리기 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "버린 후 인벤 비어있음");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", 1);
            assert_eq!(ro.len(), 1, "방 바닥에 1개");
            assert_eq!(ro[0].lock().unwrap().getName(), "철퇴");
        }

        // 3) 가져오기 철퇴
        let res = storage.execute("가져", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "가져오기 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 1, "가져온 후 인벤 1개");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", 1);
            assert_eq!(ro.len(), 0, "가져온 후 방 바닥 비어있음");
        }

        // 4) 소각 철퇴
        let res = storage.execute("소각", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "소각 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "소각 후 인벤 비어있음");

        // 5) 생성 → 부셔
        let _ = storage.execute("생성", &mut body, "289", None, None, None);
        assert_eq!(body.object.objs.len(), 1);
        let res = storage.execute("부셔", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "부셔 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "부신 후 인벤 비어있음");
    }
}
