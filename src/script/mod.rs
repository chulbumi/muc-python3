//! Rhai scripting engine for MUD server
//!
//! Provides hot-reloadable scripting support using Rhai.
//! Scripts are stored in cmds/ directory and automatically reloaded on change.

use rhai::{Engine, Scope, Dynamic};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::player::Body;
use crate::object::Object;
use crate::data::{GlobalData, SharedGlobalData};
use crate::command::parser::CommandParser;
use crate::player::{get_hp_bar_string, get_item_level_display, ITEM_EQUIP_LEVELS};
use crate::world::{
    get_world_state, Direction, format_exits_long, format_room_header, MobInstance, RawMobData,
    WorldState,
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

/// 바닥 아이템을 이름별로 묶어 format_room_objs_display로 포맷. view_map_data/build_room_lines·display_room·show_room_to_player 공용.
pub fn build_room_objs_grouped(
    room_objs: &[std::sync::Arc<std::sync::Mutex<Object>>],
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
    let grouped: Vec<(String, usize, String)> = map
        .into_iter()
        .map(|(name, (count, desc1))| (name, count, desc1))
        .collect();
    format_room_objs_display(grouped)
}

/// 방 전체 문자열(헤더·설명·출구·몹·바닥아이템). view_map_data efun 및 show_room_to_player_with_world와 동일 포맷.
pub fn build_room_lines(player_name: &str) -> String {
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
        let item_str = build_room_objs_grouped(&room_objs);
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

    let room_objs = world.get_room_objs(zone, room_i);
    for arc in &room_objs {
        let ok = {
            if let Ok(o) = arc.lock() {
                let n = o.getName();
                let reac = o.getString("반응이름");
                n == name || reac.split_whitespace().any(|s| s == name)
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

    for mob in world.mob_cache.get_mobs_in_room(zone, room_i) {
        if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
            let ok = data.name == name
                || data.reaction_names.iter().any(|r| r.as_str() == name);
            if ok {
                c += 1;
                if c == order {
                    return (mob_view(mob, data), None);
                }
            }
        }
    }

    for (pname, desc) in other_player_descs {
        if pname == &name {
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
pub fn create_engine_with_body_and_output(
    body: &mut Body,
    output_collector: Arc<Mutex<Vec<String>>>,
) -> Engine {
    let oc = output_collector.clone();
    let mut engine = create_engine_with_output(output_collector);
    let body_ptr = body as *mut Body;

    engine.register_fn("item_create", move |_ob: &mut rhai::Map, key: &str| -> String {
        let body = unsafe { &mut *body_ptr };
        if let Some((arc, name)) = object_from_item_json(key) {
            body.object.append(arc);
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
        let mut w = get_world_state().write().unwrap();
        let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
            Some(p) => (p.zone.clone(), p.room),
            None => return 0,
        };
        let room_objs = w.get_room_objs_mut(&zone, room);
        for arc in to_remove {
            body.object.remove(&arc);
            room_objs.push(arc);
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
        let room_list = w.get_room_objs_mut(&zone, room);
        let mut taken = 0usize;
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
        taken as i64
    });

    engine.register_fn("item_destroy", move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
        let body = unsafe { &mut *body_ptr };
        let order = order.max(1) as usize;
        let count = count.max(1).min(100) as usize;
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

    // view_map_data(ob): arg 없는 봐. build_room_lines(ob 이름) → output 1회 push.
    let oc_view = oc.clone();
    engine.register_fn("view_map_data", move |ob: &mut rhai::Map| {
        let name: String = ob
            .get("이름")
            .or_else(|| ob.get("name"))
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let s = build_room_lines(&name);
        if let Ok(mut out) = oc_view.lock() {
            out.push(s);
        }
    });

    // find_target(ob, line): [대상] 봐. other_player_descs=빈 map. 반환 Map: found, lines, to_target
    let body_ptr_ft = body_ptr;
    engine.register_fn("find_target", move |ob: &mut rhai::Map, line: &str| -> Dynamic {
        let viewer_name: String = ob
            .get("이름")
            .or_else(|| ob.get("name"))
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let world = get_world_state().read().unwrap();
        let other: HashMap<String, String> = HashMap::new();
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

    // send_to_player(name, msg): 당신을 살펴봅니다 전송. 당분간 no-op (broadcaster 미전달).
    engine.register_fn("send_to_player", |_name: &str, _msg: &str| {});

    // body_save(ob): 캐릭터 저장. 현재는 Body 직렬화/디스크 저장 미구현으로 스텁이 true 반환.
    engine.register_fn("body_save", |_ob: &mut rhai::Map| -> bool { true });

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

    pub fn execute(&self, name: &str, player: &mut Body, line: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let script = self.scripts.get(name)
            .ok_or_else(|| format!("Script not found: {}", name))?;

        let output_collector = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output_collector.clone();

        let engine = create_engine_with_body_and_output(player, output_clone);
        let mut scope = Scope::new();

        let mut player_data = rhai::Map::new();
        player_data.insert("name".into(), player.get_name().into());
        player_data.insert("hp".into(), player.get_hp().into());
        player_data.insert("max_hp".into(), player.get_max_hp().into());
        player_data.insert("mp".into(), player.get_mp().into());
        player_data.insert("max_mp".into(), player.get_max_mp().into());
        player_data.insert("level".into(), player.get_int("레벨").into());
        player_data.insert("money".into(), player.get_int("은전").into());
        player_data.insert("은전".into(), player.get_int("은전").into());
        player_data.insert("금전".into(), player.get_int("금전").into());
        player_data.insert("str".into(), player.get_str().into());
        player_data.insert("dex".into(), player.get_dex().into());
        player_data.insert("이름".into(), player.get_name().into());
        player_data.insert("관리자등급".into(), player.get_int("관리자등급").into());
        player_data.insert("act".into(), (player.act.to_i32() as i64).into());
        player_data.insert("성격".into(), player.get_string("성격").into());
        player_data.insert("env".into(), "".into());

        let objs_array = rhai::Array::new();
        player_data.insert("objs".into(), rhai::Dynamic::from(objs_array));

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
        Ok(expanded)
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

    pub async fn execute(&self, name: &str, player: &mut Body, line: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let storage = self.inner.read().await;
        storage.execute(name, player, line)
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
        let res = storage.execute("생성", &mut body, "289");
        assert!(res.is_ok(), "생성 실패: {:?}", res.err());
        let out = res.as_ref().unwrap();
        assert_eq!(body.object.objs.len(), 1, "생성 후 인벤 1개 (outputs: {:?})", out);
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "철퇴");

        // 2) 버리기 철퇴
        let res = storage.execute("버려", &mut body, "철퇴");
        assert!(res.is_ok(), "버리기 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "버린 후 인벤 비어있음");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", 1);
            assert_eq!(ro.len(), 1, "방 바닥에 1개");
            assert_eq!(ro[0].lock().unwrap().getName(), "철퇴");
        }

        // 3) 가져오기 철퇴
        let res = storage.execute("가져", &mut body, "철퇴");
        assert!(res.is_ok(), "가져오기 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 1, "가져온 후 인벤 1개");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", 1);
            assert_eq!(ro.len(), 0, "가져온 후 방 바닥 비어있음");
        }

        // 4) 소각 철퇴
        let res = storage.execute("소각", &mut body, "철퇴");
        assert!(res.is_ok(), "소각 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "소각 후 인벤 비어있음");

        // 5) 생성 → 부셔
        let _ = storage.execute("생성", &mut body, "289");
        assert_eq!(body.object.objs.len(), 1);
        let res = storage.execute("부셔", &mut body, "철퇴");
        assert!(res.is_ok(), "부셔 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "부신 후 인벤 비어있음");
    }
}
