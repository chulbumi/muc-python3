//! Rhai scripting engine for MUD server
//!
//! Provides hot-reloadable scripting support using Rhai.
//! Scripts are stored in cmds/ directory and automatically reloaded on change.

#![allow(clippy::type_complexity)]
#![allow(static_mut_refs)]

use rhai::{Dynamic, Engine, Scope};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::command::parser::CommandParser;
use crate::command::CommandResult;
use crate::data::SharedGlobalData;
use crate::network::Broadcaster;
use crate::object::{Object, Value};
use crate::player::{get_hp_bar_string, get_item_level_display, ITEM_EQUIP_LEVELS};
use crate::player::{Body, MemoRecord};
use crate::scheduler::CallOutScheduler;
use crate::world::guild::{
    guild_attr_keys, guild_get, guild_has, guild_list, guild_remove, guild_save, guild_set,
};
use crate::world::rank::{rank_clear, rank_get_all, rank_get_num, rank_read, rank_write};
use crate::world::{
    format_exits_long, format_room_header, get_world_state, Direction, MobInstance, PlayerPosition,
    RawMobData, WorldState,
};
use std::time::Duration;

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
    /// Directory containing library .rhai scripts (hot-reloadable)
    pub lib_dir: PathBuf,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            script_dir: PathBuf::from("cmds"),
            hot_reload: true,
            extension: ".rhai".to_string(),
            data_dir: PathBuf::from("data/config"),
            lib_dir: PathBuf::from("lib"),
        }
    }
}

// 스크립트용: handle_game_command에서 미리 채워 둔 전 접속자 목록. get_all_online_players()가 참조.
thread_local! {
    static PRE_COMPUTED_ALL_ONLINE: RefCell<Option<rhai::Array>> = const { RefCell::new(None) };
}

/// handle_game_command에서 호출. 전 접속자(이름, 무림별호, 성격, 레벨초기화, 소속) 배열 세팅.
pub fn set_precomputed_all_online(a: rhai::Array) {
    PRE_COMPUTED_ALL_ONLINE.with(|c| *c.borrow_mut() = Some(a));
}

/// 스크립트 get_all_online_players()에서 호출.
pub fn get_precomputed_all_online() -> rhai::Array {
    PRE_COMPUTED_ALL_ONLINE
        .with(|c| c.borrow().clone())
        .unwrap_or_default()
}

/// PreComputedOtherDescsGuard Drop에서 호출.
pub fn clear_precomputed_all_online() {
    PRE_COMPUTED_ALL_ONLINE.with(|c| *c.borrow_mut() = None);
}

/// 설정상태 문자열 파싱: "키 값" (줄바꿈 또는 공백 구분). ob["설정"][키]에 대응.
fn parse_config_string(s: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    if s.is_empty() {
        return out;
    }
    let mut pairs: Vec<(String, String)> = Vec::new();
    if s.contains('\n') {
        for line in s.split('\n') {
            let line = line.trim();
            if let Some(sp) = line.find(' ') {
                let (k, v) = (line[..sp].to_string(), line[sp + 1..].trim().to_string());
                if !k.is_empty() {
                    pairs.push((k, v));
                }
            }
        }
    } else {
        let toks: Vec<&str> = s.split_whitespace().collect();
        let mut i = 0;
        while i + 1 < toks.len() {
            pairs.push((toks[i].to_string(), toks[i + 1].to_string()));
            i += 2;
        }
    }
    for (k, v) in pairs {
        out.insert(k, v);
    }
    out
}

/// 설정상태 맵을 문자열로 직렬화. "키 값"을 \n으로 이어붙임.
fn format_config_string(m: &std::collections::HashMap<String, String>) -> String {
    let mut v: Vec<_> = m.iter().map(|(k, val)| format!("{} {}", k, val)).collect();
    v.sort();
    v.join("\n")
}

/// 이벤트설정리스트 파싱: "키=값" 또는 "키" 한 줄씩(\n 구분). ob["이벤트"][키]에 대응.
/// world::event::do_event에서도 사용. pub(crate).
pub(crate) fn parse_event_string(s: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for line in s.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(eq) = line.find('=') {
            out.insert(line[..eq].to_string(), line[eq + 1..].to_string());
        } else {
            out.insert(line.to_string(), "1".to_string());
        }
    }
    out
}

pub(crate) fn format_event_string(m: &std::collections::HashMap<String, String>) -> String {
    let mut v: Vec<_> = m
        .iter()
        .map(|(k, val)| {
            if val == "1" {
                k.clone()
            } else {
                format!("{}={}", k, val)
            }
        })
        .collect();
    v.sort();
    v.join("\n")
}

// ============================================================
// 호위 (Guard) 시스템 관련 타입 및 헬퍼 함수
// ============================================================

/// 호위 데이터 구조체
#[derive(Debug, Clone)]
struct GuardData {
    name: String,
    hp: i64,
    max_hp: i64,
    description: String,
}

/// 호위 리스트 파싱: JSON 형식 문자열에서 GuardData 벡터로
fn parse_guards_list(s: &str) -> Vec<GuardData> {
    if s.trim().is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Array(arr)) => {
            let mut guards = Vec::new();
            for v in arr {
                if let Some(obj) = v.as_object() {
                    let name = obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .or_else(|| obj.get("이름").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let hp = obj
                        .get("hp")
                        .and_then(|v| v.as_i64())
                        .or_else(|| obj.get("체력").and_then(|v| v.as_i64()))
                        .unwrap_or(100);
                    let max_hp = obj
                        .get("max_hp")
                        .and_then(|v| v.as_i64())
                        .or_else(|| obj.get("max_체력").and_then(|v| v.as_i64()))
                        .or_else(|| obj.get("최고체력").and_then(|v| v.as_i64()))
                        .unwrap_or(hp);
                    let description = obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .or_else(|| obj.get("설명").and_then(|v| v.as_str()))
                        .or_else(|| obj.get("설명2").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();

                    if !name.is_empty() {
                        guards.push(GuardData {
                            name,
                            hp,
                            max_hp,
                            description,
                        });
                    }
                }
            }
            guards
        }
        _ => Vec::new(),
    }
}

/// 호위 리스트를 JSON 형식 문자열로 변환
fn format_guards_list(guards: &[GuardData]) -> String {
    let arr: Vec<serde_json::Value> = guards
        .iter()
        .map(|g| {
            serde_json::json!({
                "name": g.name,
                "hp": g.hp,
                "max_hp": g.max_hp,
                "description": g.description
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_default()
}

/// 몹 이름으로 몹 데이터 조회 (get_mob_by_name 구현)
fn get_mob_by_name_impl(mob_name: &str) -> Option<serde_json::Value> {
    let full_path = format!("data/mob/{}.json", mob_name);
    std::fs::read_to_string(&full_path)
        .ok()
        .and_then(|content| {
            serde_json::from_str::<serde_json::Value>(&content)
                .ok()
                .and_then(|v| v.get("몹정보").cloned())
        })
}

/// 접속 중인 이름 목록. get_precomputed_all_online에서 이름만 추출.
pub fn get_online_names() -> rhai::Array {
    use rhai::Dynamic;
    PRE_COMPUTED_ALL_ONLINE.with(|c| {
        let a = c.borrow();
        if let Some(ref arr) = *a {
            let mut out = rhai::Array::new();
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    if let Some(n) = m
                        .get("이름")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                    {
                        if !n.is_empty() {
                            out.push(Dynamic::from(n));
                        }
                    }
                }
            }
            out
        } else {
            rhai::Array::new()
        }
    })
}

/// 해당 이름이 설정(ob["설정"]["외침거부"])에서 "1"인지. get_precomputed_all_online의 설정상태 파싱.
pub fn user_refuses_shout(name: &str) -> bool {
    use rhai::Dynamic;
    PRE_COMPUTED_ALL_ONLINE.with(|c| {
        let a = c.borrow();
        if let Some(ref arr) = *a {
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    let n: String = m
                        .get("이름")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    if n == name {
                        let cfg: String = m
                            .get("설정상태")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                            .unwrap_or_default();
                        return parse_config_string(&cfg)
                            .get("외침거부")
                            .map(|v| v.as_str())
                            == Some("1");
                    }
                }
            }
        }
        false
    })
}

/// Stored script with metadata
struct StoredScript {
    /// Source code of the script
    source: String,
    /// Last modification time
    modified: std::time::SystemTime,
    /// Script name
    _name: String,
}

/// Equipment stats for applying/removing bonuses
struct EquipStats {
    attack: i32,
    defense: i32,
    strength: i32,
    dexterity: i32,
    armor: i32,
    max_hp: i32,
    max_mp: i32,
    hit: i32,
    miss: i32,
    critical: i32,
    luck: i32,
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

/// Korean particle helper (은/는)
fn han_eun(name: &str) -> String {
    use crate::hangul::han_un;
    han_un(name).to_string()
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
    use sha2::{Digest, Sha512};
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
            // 배열을 파이프로 구분된 문자열로 변환 (Rust 내부 형식)
            // Python은 ["skill1", "skill2"] 또는 ["skill1 100 100", "skill2 100 100"] 형식으로 저장
            let s = arr
                .iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join("|");
            Value::String(s)
        }
        serde_json::Value::Object(_) => Value::String(serde_json::to_string(v).unwrap_or_default()),
    }
}

/// Body를 data/user/{이름}.json 에 저장. 소지품(objs, inv_stack) 포함.
/// 저장 직전에 마지막저장시간을 갱신한다.
pub fn save_body_to_json(body: &mut Body, path: &str) -> bool {
    if std::fs::create_dir_all(Path::new(path).parent().unwrap_or(Path::new("."))).is_err() {
        return false;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    body.object
        .attr
        .insert("마지막저장시간".to_string(), Value::Int(now));

    let mut uso = serde_json::Map::new();
    for (k, v) in &body.object.attr {
        // Python 호환성: 파이프 구분 문자열을 배열로 변환
        if k == "무공숙련도" || k == "무공이름" {
            if let Value::String(s) = v {
                if !s.is_empty() {
                    // "skill1|skill2" 또는 "skill1 level exp|skill2 level exp" 형식을 배열로 변환
                    let parts: Vec<serde_json::Value> = s
                        .split('|')
                        .map(|p| serde_json::Value::String(p.trim().to_string()))
                        .filter(|p| !p.as_str().map(|s| s.is_empty()).unwrap_or(true))
                        .collect();
                    uso.insert(k.clone(), serde_json::Value::Array(parts));
                    continue;
                }
            }
        }
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
                let arr: Vec<serde_json::Value> = rn
                    .split_whitespace()
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect();
                rec.insert("반응이름".to_string(), serde_json::Value::Array(arr));
            }
            for key in &[
                "공격력",
                "방어력",
                "기량",
                "옵션",
                "아이템속성",
                "확장 이름",
                "체력",
                "고유번호",
            ] {
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
    // 0인 항목은 저장하지 않음
    let stack_map: serde_json::Map<String, serde_json::Value> = body
        .object
        .inv_stack
        .iter()
        .filter(|(_, v)| **v > 0)
        .map(|(k, v)| {
            (
                k.clone(),
                serde_json::Value::Number(serde_json::Number::from(*v)),
            )
        })
        .collect();
    root.insert(
        "소지품_수량".to_string(),
        serde_json::Value::Object(stack_map),
    );

    for (k, v) in &body.memos {
        if let Ok(val) = serde_json::to_value(v) {
            root.insert(k.clone(), val);
        }
    }

    // 대화 기록 저장
    if !body.talk_history.is_empty() {
        let talk_arr: Vec<serde_json::Value> = body
            .talk_history
            .iter()
            .map(|s| serde_json::Value::String(s.clone()))
            .collect();
        root.insert("대화기록".to_string(), serde_json::Value::Array(talk_arr));
    }

    let j = serde_json::Value::Object(root);
    std::fs::write(path, serde_json::to_string_pretty(&j).unwrap_or_default()).is_ok()
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

        // Python 호환성: 금화/은화를 은전으로 변환
        // 일부 Python JSON은 "금화", "은화" 필드를 사용하지만
        // Rust 내부와 최신 Python은 "은전" 필드를 사용
        let has_gold = uso.contains_key("금화") || uso.contains_key("은화");
        let has_money = uso.contains_key("은전");
        if has_gold && !has_money {
            let gold = body.object.getInt("금화");
            let silver = body.object.getInt("은화");
            // 금화 1개 = 은전 10000개 (Python 규칙)
            let total_money = gold * 10000 + silver;
            body.object.set("은전", total_money);
        }

        // Python 호환성: "현재방" 필드를 "위치"로도 복사
        // Python JSON은 "현재방" 필드를 사용하지만 Rust 내부에서는 "위치"를 사용
        if uso.contains_key("현재방") && !uso.contains_key("위치") {
            let current_room = body.object.getString("현재방");
            if !current_room.is_empty() {
                body.object.set("위치", current_room);
            }
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

    body.memos.clear();
    for (k, v) in root.iter() {
        if k.starts_with("메모:") {
            if let Some(obj) = v.as_object() {
                let record = MemoRecord {
                    제목: obj
                        .get("제목")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    시간: obj
                        .get("시간")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    작성자: obj
                        .get("작성자")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    내용: obj
                        .get("내용")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                };
                body.memos.insert(k.clone(), record);
            }
        }
    }

    // 대화 기록 로드
    body.talk_history.clear();
    if let Some(arr) = root.get("대화기록").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                body.talk_history.push(s.to_string());
            }
        }
    }

    true
}

/// data/script/{path} 로드. JSON 배열이면 파싱, 아니면 줄 단위. $스크립트호출·무기강화용.
pub(crate) fn load_script_file(path: &str) -> Option<Vec<String>> {
    let p = std::path::Path::new("data/script").join(path);
    let content = std::fs::read_to_string(&p).ok()?;
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(content.trim()) {
        return Some(arr);
    }
    Some(content.lines().map(|s| s.to_string()).collect())
}

/// Create an Object from data/item/{key}.json 아이템정보.
/// Returns None if file missing or invalid; else Some((object, 아이템정보.이름 or key)).
/// world::event::$아이템주기에서 사용. pub(crate).
pub(crate) fn object_from_item_json(key: &str) -> Option<(Arc<Mutex<Object>>, String)> {
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
    let name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let rn = info
        .get("반응이름")
        .map(|v| {
            if let Some(s) = v.as_str() {
                s.to_string()
            } else if let Some(arr) = v.as_array() {
                arr.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
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

/// 소비성 아이템 정보 가져오기 (이름, 체력회복, 내공회복)
/// 종류가 "먹는것"인 경우에만 값을 반환, 아니면 (0, 0, 0)
fn get_consumable_info(key: &str) -> (String, i64, i64) {
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (String::new(), 0, 0),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return (String::new(), 0, 0),
    };
    let info = match json.get("아이템정보").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return (String::new(), 0, 0),
    };
    let kind = info.get("종류").and_then(|v| v.as_str()).unwrap_or("");
    if kind != "먹는것" {
        return (String::new(), 0, 0);
    }
    let name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let hp = info.get("체력").and_then(|v| v.as_i64()).unwrap_or(0);
    let mp = info.get("내공").and_then(|v| v.as_i64()).unwrap_or(0);
    (name, hp, mp)
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
        if p.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let key = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
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
    unsafe { CURRENT_OBJECT.clone() }
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
    engine.register_fn("trim", |s: &str| -> String { s.trim().to_string() });
    engine.register_fn("substring", |s: &str, start: i64, end: i64| -> String {
        let chars: Vec<char> = s.chars().collect();
        let start_idx = if start < 0 { 0 } else { start as usize };
        let end_idx = if end < 0 { chars.len() } else { end as usize };
        if start_idx >= chars.len() {
            return String::new();
        }
        let end_idx = end_idx.min(chars.len());
        chars[start_idx..end_idx].iter().collect()
    });
    engine.register_fn("length", |s: &str| -> i64 { s.chars().count() as i64 });

    // Array utilities
    engine.register_fn("len", |arr: &mut rhai::Array| -> i64 { arr.len() as i64 });
    engine.register_fn("length", |arr: &mut rhai::Array| -> i64 {
        arr.len() as i64
    });
    engine.register_fn("join", |arr: &mut rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
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

    engine.register_fn("han_iga", |name: &str| -> String { han_iga(name) });
    engine.register_fn("han_eul", |name: &str| -> String { han_eul(name) });
    engine.register_fn("han_eun", |name: &str| -> String { han_eun(name) });
    engine.register_fn("han_wa", |name: &str| -> String { han_wa(name) });

    // 이름 ANSI(노랑), 문자열 치환, 정수→문자. format_room_objs.rhai 등에서 사용.
    engine.register_fn("name_ansi", |s: &str| -> String {
        format!("\x1b[33m{}\x1b[37m", s)
    });
    engine.register_fn("replace_str", |s: &str, from: &str, to: &str| -> String {
        s.replace(from, to)
    });
    engine.register_fn("int_to_str", |i: i64| -> String { i.to_string() });

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

    engine.register_fn(
        "get_attr",
        |player_data: &mut rhai::Map, key: &str| -> Dynamic {
            player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
        },
    );

    engine.register_fn(
        "set_attr",
        |player_data: &mut rhai::Map, key: &str, value: Dynamic| {
            player_data.insert(key.to_string().into(), value);
        },
    );

    engine.register_fn("get_int", |player_data: &mut rhai::Map, key: &str| -> i64 {
        player_data
            .get(key)
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    engine.register_fn(
        "get_string",
        |player_data: &mut rhai::Map, key: &str| -> String {
            player_data
                .get(key)
                .and_then(|v| {
                    if v.is_string() {
                        Some(v.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        },
    );

    // ============================================================
    // DIFFICULTY ZONE FUNCTIONS
    // ============================================================

    // Get difficulty level from zone name (e.g., "낙양성1" -> 1, "낙양성" -> 0)
    engine.register_fn("get_difficulty_from_zone", |zone: &str| -> i64 {
        use crate::world::difficulty_from_zone;
        difficulty_from_zone(zone) as i64
    });

    // Get base zone name (e.g., "낙양성1" -> "낙양성")
    engine.register_fn("get_base_zone_name", |zone: &str| -> String {
        use crate::world::base_zone_name;
        base_zone_name(zone).to_string()
    });

    // Get minimum level required for a difficulty zone
    engine.register_fn("get_min_level_for_difficulty", |difficulty: i64| -> i64 {
        use crate::world::DifficultyConfig;
        DifficultyConfig::min_level_for_difficulty(difficulty as u8)
    });

    // Get difficulty config for a level
    engine.register_fn("get_difficulty_config", |difficulty: i64| -> rhai::Map {
        use crate::world::DifficultyConfig;
        let config = DifficultyConfig::get(difficulty as u8);
        let mut map = rhai::Map::new();
        map.insert("level_bonus".into(), rhai::Dynamic::from(config.level_bonus));
        map.insert("hp_multiplier".into(), rhai::Dynamic::from(config.hp_multiplier as i64));
        map.insert("str_multiplier".into(), rhai::Dynamic::from(config.str_multiplier as i64));
        map.insert("arm_multiplier".into(), rhai::Dynamic::from(config.arm_multiplier as i64));
        map.insert("agi_multiplier".into(), rhai::Dynamic::from(config.agi_multiplier as i64));
        map.insert("exp_multiplier".into(), rhai::Dynamic::from(config.exp_multiplier as i64));
        map.insert("gold_multiplier".into(), rhai::Dynamic::from(config.gold_multiplier as i64));
        map
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

    engine.register_fn("to_int", |s: &str| -> i64 { s.trim().parse().unwrap_or(0) });

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

    // present(env, name) - Find object by name in environment
    // Searches through env["objs"] array for matching name/반응이름/설명1
    engine.register_fn("present", |env: &mut rhai::Map, name: &str| -> Dynamic {
        use rhai::Dynamic;

        // Get objs array from environment
        if let Some(objs_value) = env.get("objs") {
            if let Some(objs) = objs_value.clone().try_cast::<rhai::Array>() {
                for obj in &objs {
                    if let Some(obj_map) = obj.clone().try_cast::<rhai::Map>() {
                        // Check 이름
                        if let Some(name_value) = obj_map.get("이름") {
                            if let Some(obj_name) = name_value.clone().try_cast::<String>() {
                                if obj_name == name {
                                    return obj.clone();
                                }
                            }
                        }
                        // Check 반응이름 (array)
                        if let Some(reactions_value) = obj_map.get("반응이름") {
                            if let Some(reactions) =
                                reactions_value.clone().try_cast::<rhai::Array>()
                            {
                                for reaction in &reactions {
                                    if let Some(reaction_str) =
                                        reaction.clone().try_cast::<String>()
                                    {
                                        if reaction_str == name {
                                            return obj.clone();
                                        }
                                    }
                                }
                            }
                        }
                        // Check 설명1 (display name)
                        if let Some(desc_value) = obj_map.get("설명1") {
                            if let Some(desc1) = desc_value.clone().try_cast::<String>() {
                                if desc1 == name {
                                    return obj.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
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
        // Support both "zone:filename" and plain "filename" formats
        let full_path = if name.contains(':') {
            let parts: Vec<&str> = name.splitn(2, ':').collect();
            if parts.len() == 2 {
                format!("data/mob/{}/{}.json", parts[0], parts[1])
            } else {
                format!("data/mob/{}.json", name)
            }
        } else {
            format!("data/mob/{}.json", name)
        };
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("몹정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_room_data", |name: &str| -> Dynamic {
        // name format: "zone:room"
        let full_path = format!("data/map/{}.json", name.replace(":", "/"));
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("맵정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_skill_data", |name: &str| -> Dynamic {
        // Load skill.json and find the skill
        match std::fs::read_to_string("data/config/skill.json") {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
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
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    // ============================================================
    // SKILL UTILITY FUNCTIONS
    // ============================================================

    engine.register_fn("get_skill_defense_head", |name: &str| -> String {
        crate::world::skill::get_skill_defense_head(name)
    });

    engine.register_fn("get_skill_type", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| {
                    match s.skill_type {
                        crate::world::skill::SkillType::Combat => "전투",
                        crate::world::skill::SkillType::Defense => "방어",
                        crate::world::skill::SkillType::Internal => "내공",
                        crate::world::skill::SkillType::Other => "기타",
                    }
                    .to_string()
                })
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("get_skill_mp_cost", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.mp_cost).unwrap_or(0)
        } else {
            0
        }
    });

    engine.register_fn("get_skill_hp_cost", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.hp_cost).unwrap_or(0)
        } else {
            0
        }
    });

    engine.register_fn("get_skill_probability", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.probability).unwrap_or(100)
        } else {
            100
        }
    });

    engine.register_fn("get_skill_hit_rate", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.hit_rate as i64).unwrap_or(0)
        } else {
            0
        }
    });

    engine.register_fn("get_skill_mugong_script", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.mugong_script.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("get_skill_fail_message", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.fail_message.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("is_all_attack_skill", |name: &str| -> bool {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.is_all_attack()).unwrap_or(false)
        } else {
            false
        }
    });

    engine.register_fn("get_skill_category", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.category.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("get_skill_anti_type", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.get_anti_type().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    // Calculate normal attacks from remaining dex (after skill execution)
    // Returns array: [attack_count, remainder_dex]
    engine.register_fn("calculate_normal_attacks", |dex: i64| -> rhai::Array {
        let (count, remainder) = crate::world::skill::calculate_normal_attacks(dex);
        vec![Dynamic::from(count), Dynamic::from(remainder)]
    });

    // Note: 비전 (Secret Skill) functions are available directly via Body methods
    // and commands in vision.rs. Script efuns for 비전 removed since they require
    // a player cache system not yet implemented.

    engine
}

/// 바닥 아이템 이름별 묶음 포맷. 파이썬 viewMapData nStr. format_room_objs.rhai와 동일 로직을 Rust로 구현.
/// grouped: (name, count, desc1) 들. 공통: 봐/이동 시 방 표시.
pub fn format_room_objs_display(grouped: Vec<(String, usize, String)>) -> String {
    if grouped.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = Vec::with_capacity(grouped.len());
    for (name, count, desc1) in grouped {
        let name_a = format!("\x1b[36m{}\x1b[37m", name);
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
/// 오류 시 Err((코드, zone, room)): "no_position"|"room_error"|"unknown_room". 성공 시 Ok(문자열).
/// other_player_descs: 같은 방의 다른 접속 유저 getDesc.
pub fn build_room_lines(
    player_name: &str,
    other_player_descs: &[String],
) -> Result<String, (String, String, String)> {
    let world = get_world_state().read().unwrap();
    let pos = match world.get_player_position(player_name) {
        Some(p) => p.clone(),
        None => return Err(("no_position".to_string(), String::new(), "0".to_string())),
    };
    if let Some(room) = world.room_cache.get_room_cached(&pos.zone, &pos.room) {
        let room_ref = match room.read() {
            Ok(r) => r,
            Err(_) => return Err(("room_error".to_string(), String::new(), "0".to_string())),
        };
        let room_name_formatted = format_room_header(&room_ref.display_name);
        let exits_str = format_exits_long(&room_ref);
        let mobs = world.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
        let mob_str = if mobs.is_empty() {
            String::new()
        } else {
            let mut mob_msgs = Vec::new();
            for mob in mobs {
                if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                    // Python viewMapData mob display logic:
                    // 몹종류 7: skip
                    if mob_data.mob_type == 7 {
                        continue;
                    }
                    // ACT_REGEN (3): skip
                    // ACT_REST (4): "이/가 흐트러진 진기를 추스리고 있습니다."
                    // ACT_STAND (0): getDesc1()
                    // ACT_FIGHT (1): 방어상태머리말 + "이/가 목숨을 건 사투를 벌이고 있습니다."
                    // ACT_DEATH (2): "의 싸늘한 시체가 있습니다."

                    if mob.act == 3 {
                        // ACT_REGEN - skip
                        continue;
                    }

                    if mob.act == 4 {
                        // ACT_REST
                        let suffix = if crate::hangul::ends_with_consonant(&mob_data.name) {
                            format!("{} 흐트러진 진기를 추스리고 있습니다.", mob_data.name)
                        } else {
                            format!("{}가 흐트러진 진기를 추스리고 있습니다.", mob_data.name)
                        };
                        mob_msgs.push(suffix);
                    } else if mob.act == 0 {
                        // ACT_STAND - show desc1
                        if !mob_data.desc1.is_empty() {
                            mob_msgs.push(mob_data.desc1.clone());
                        }
                    } else if mob.act == 1 {
                        // ACT_FIGHT
                        let mut prefix = String::new();
                        for skill_name in &mob.skills {
                            let defense_head = crate::data::get_skill_defense_head(skill_name);
                            if !defense_head.is_empty() {
                                prefix.push_str(&defense_head);
                                prefix.push(' ');
                            }
                        }
                        let suffix = if crate::hangul::ends_with_consonant(&mob_data.name) {
                            format!("{}목숨을 건 사투를 벌이고 있습니다.", mob_data.name)
                        } else {
                            format!("{}가 목숨을 건 사투를 벌이고 있습니다.", mob_data.name)
                        };
                        mob_msgs.push(format!("{}{}", prefix, suffix));
                    } else if mob.act == 2 {
                        // ACT_DEATH
                        #[allow(clippy::if_same_then_else)]
                        let suffix = if crate::hangul::ends_with_consonant(&mob_data.name) {
                            format!("{}의 싸늘한 시체가 있습니다.", mob_data.name)
                        } else {
                            format!("{}의 싸늘한 시체가 있습니다.", mob_data.name)
                        };
                        mob_msgs.push(suffix);
                    } else {
                        // Other states - show desc1
                        if !mob_data.desc1.is_empty() {
                            mob_msgs.push(mob_data.desc1.clone());
                        }
                    }
                }
            }
            if mob_msgs.is_empty() {
                String::new()
            } else {
                format!("\r\n{}", mob_msgs.join("\r\n"))
            }
        };
        let room_objs = world.get_room_objs(&pos.zone, &pos.room);
        let room_stack = world.get_room_objs_stack(&pos.zone, &pos.room);
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
        Ok(out)
    } else {
        Err((
            "unknown_room".to_string(),
            pos.zone.clone(),
            pos.room.clone(),
        ))
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
    let m = if m.is_empty() {
        "무명객".to_string()
    } else {
        m
    };
    let c = body.get_string("성격");
    let c = if c.is_empty() {
        "없음".to_string()
    } else {
        c
    };
    let s = format!("◆ 이  름 ▷ 『{}』 {}", m, body.get_name());
    let c2 = format!("◆ 성격 ▷ 『{}』", c);
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}  {}\x1b[0m\x1b[37m\x1b[40m",
        s, c2
    ));
    let ba = body.get_string("배우자");
    let ba = if ba.is_empty() {
        "미혼".to_string()
    } else {
        ba
    };
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
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}의 시체\x1b[0m\x1b[37m\x1b[40m",
            data.name
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
    lines.push(format!(
        "☆ {} ({})",
        get_hp_bar_string(mob.hp, mob.max_hp),
        pct
    ));
    lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// 아이템 상세 보기. 파이썬 objs/item.view(ob). find_target/look_at_target에서 사용.
fn item_view(obj: &Arc<Mutex<Object>>) -> Vec<String> {
    let o = obj.lock().unwrap();
    let name_a = o.getNameA();
    let mut lines = vec![
        "━━━━━━━━━━━━━━━━━━━━━".to_string(),
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}\x1b[0m\x1b[37m\x1b[40m",
            o.getName()
        ),
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 종류 ▷ {}\x1b[0m\x1b[37m\x1b[40m",
            o.getString("종류")
        ),
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
    let room_s = pos.room.as_str();
    let mut c = 0usize;

    if name.is_empty() && order >= 1 {
        for mob in world.mob_cache.get_mobs_in_room(zone, room_s) {
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
        for mob in world.mob_cache.get_mobs_in_room(zone, room_s) {
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
    let room_objs = world.get_room_objs(zone, room_s);
    for arc in &room_objs {
        let ok = {
            if let Ok(o) = arc.lock() {
                let n = o.getName();
                let reac = o.getString("반응이름");
                n == name
                    || reac
                        .split_whitespace()
                        .any(|s| s == name || s.starts_with(name.as_str()))
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
    for mob in world.mob_cache.get_mobs_in_room(zone, room_s) {
        if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
            let ok = data.name == name
                || data.name.starts_with(name.as_str())
                || data
                    .reaction_names
                    .iter()
                    .any(|r| r.as_str() == name || r.starts_with(name.as_str()));
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
        if let Some(room_arc) = world.room_cache.get_room_cached(zone, room_s) {
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
    engine.register_fn("trim", |s: &str| -> String { s.trim().to_string() });
    engine.register_fn("substring", |s: &str, start: i64, end: i64| -> String {
        let chars: Vec<char> = s.chars().collect();
        let start_idx = if start < 0 { 0 } else { start as usize };
        let end_idx = if end < 0 { chars.len() } else { end as usize };
        if start_idx >= chars.len() {
            return String::new();
        }
        let end_idx = end_idx.min(chars.len());
        chars[start_idx..end_idx].iter().collect()
    });
    engine.register_fn("length", |s: &str| -> i64 { s.chars().count() as i64 });

    // Array utilities
    engine.register_fn("len", |arr: &mut rhai::Array| -> i64 { arr.len() as i64 });
    engine.register_fn("length", |arr: &mut rhai::Array| -> i64 {
        arr.len() as i64
    });
    engine.register_fn("join", |arr: &mut rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
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

    engine.register_fn("han_iga", |name: &str| -> String { han_iga(name) });
    engine.register_fn("han_eul", |name: &str| -> String { han_eul(name) });
    engine.register_fn("han_eun", |name: &str| -> String { han_eun(name) });
    engine.register_fn("han_wa", |name: &str| -> String { han_wa(name) });

    // ============================================================
    // OUTPUT FUNCTIONS (with collection)
    // ============================================================

    let oc = output_collector.clone();
    engine.register_fn(
        "send_line",
        move |_player_data: &mut rhai::Map, msg: &str| {
            println!("[SEND_LINE] Called with msg: {}", msg);
            match oc.lock() {
                Ok(mut output) => {
                    println!("[SEND_LINE] Lock acquired, pushing to output");
                    output.push(msg.to_string());
                    println!("[SEND_LINE] Output now has {} items", output.len());
                }
                Err(e) => {
                    println!("[SEND_LINE] Lock error: {:?}", e);
                }
            }
        },
    );

    let oc = output_collector.clone();
    engine.register_fn(
        "send_room",
        move |_player_data: &mut rhai::Map, msg: &str| {
            println!("[SEND_ROOM] {}", msg);
            if let Ok(mut output) = oc.lock() {
                output.push(msg.to_string());
            }
        },
    );

    engine.register_fn("print", |s: &str| {
        println!("[SCRIPT] {}", s);
    });
    engine.register_fn("debug", |s: &str| {
        debug!("[SCRIPT] {}", s);
    });

    // ============================================================
    // ATTRIBUTE ACCESS
    // ============================================================

    engine.register_fn(
        "get_attr",
        |player_data: &mut rhai::Map, key: &str| -> Dynamic {
            player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
        },
    );

    engine.register_fn(
        "set_attr",
        |player_data: &mut rhai::Map, key: &str, value: Dynamic| {
            player_data.insert(key.to_string().into(), value);
        },
    );

    engine.register_fn("get_int", |player_data: &mut rhai::Map, key: &str| -> i64 {
        player_data
            .get(key)
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    engine.register_fn(
        "get_string",
        |player_data: &mut rhai::Map, key: &str| -> String {
            player_data
                .get(key)
                .and_then(|v| {
                    if v.is_string() {
                        Some(v.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        },
    );

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
            format!(
                "{}{:width$}",
                fill.repeat((width - len) as usize),
                s,
                width = width as usize
            )
        }
    });

    // repeat function for Rhai scripts
    engine.register_fn("repeat", |s: &str, count: i64| -> String {
        s.repeat(count.max(0) as usize)
    });

    engine.register_fn("to_int", |s: &str| -> i64 { s.trim().parse().unwrap_or(0) });

    engine.register_fn("int_to_str", |i: i64| -> String { i.to_string() });

    engine.register_fn("split", |s: &str, sep: &str| -> rhai::Array {
        s.split(sep)
            .map(|x| rhai::Dynamic::from(x.to_string()))
            .collect()
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
        vec![rhai::Dynamic::from(order), rhai::Dynamic::from(name)]
    });

    // parse_name_order(s): "2.검" -> [name, order]. 주다 등. CommandParser::parse_name_order.
    engine.register_fn("parse_name_order", |s: &str| -> rhai::Array {
        let (name, order) = CommandParser::parse_name_order(s);
        vec![rhai::Dynamic::from(name), rhai::Dynamic::from(order as i64)]
    });

    // ============================================================
    // COMMAND HELPER EFUNS (반복 패턴)
    // ============================================================

    engine.register_fn("is_empty", |s: &str| -> bool { s.trim().is_empty() });

    // is_unit(value) - Check if a Dynamic value is unit (empty/not found)
    engine.register_fn("is_unit", |value: rhai::Dynamic| -> bool { value.is_unit() });

    // int_to_str(value) - Convert integer to string (handles both int and string inputs)
    engine.register_fn("int_to_str", |value: rhai::Dynamic| -> String {
        if value.is_int() {
            value.as_int().unwrap_or(0).to_string()
        } else if value.is_string() {
            value.into_string().unwrap_or_default()
        } else {
            "".to_string()
        }
    });

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
        line
            .split_whitespace()
            .map(|x| rhai::Dynamic::from(x.to_string()))
            .collect()
    });

    // require_arg: 기능만. line이 비었으면 false. usage/오류 메시지는 Rhai에서 send_line.
    engine.register_fn("require_arg", |_ob: &mut rhai::Map, line: &str| -> bool {
        !line.trim().is_empty()
    });

    // require_admin: 기능만. 관리자등급 < min_level 이면 false. 오류 메시지는 Rhai에서 send_line.
    engine.register_fn(
        "require_admin",
        |ob: &mut rhai::Map, min_level: i64| -> bool {
            let adm = ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0i64);
            adm >= min_level
        },
    );

    // Text formatting functions for item actions
    engine.register_fn(
        "format_item_action_self",
        |name: &str, action: &str, count: i64| -> String {
            if count > 1 {
                format!("{} {} {}개를 {}.", name, action, count, han_iga(name))
            } else {
                format!("{} {} {}.", name, han_iga(name), action)
            }
        },
    );
    engine.register_fn(
        "format_item_action_target",
        |name: &str, target: &str, action: &str, count: i64| -> String {
            if count > 1 {
                format!(
                    "{} {} {}개를 {} {}.",
                    name,
                    action,
                    count,
                    han_eun(target),
                    target
                )
            } else {
                format!("{} {} {} {}.", name, han_eun(target), target, action)
            }
        },
    );

    // Note: format_hp_bar, format_time, format_item_name, format_mob_name are now implemented
    // in lib/format.rhai for hot-reload capability. They are loaded as library scripts.

    // ============================================================
    // DATA LOADING (get_item_data, get_mob_data, get_room_data, get_skill_data)
    // ============================================================

    engine.register_fn("get_item_data", |name: &str| -> Dynamic {
        let full_path = format!("data/item/{}.json", name);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("아이템정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_mob_data", |name: &str| -> Dynamic {
        // Support both "zone:filename" and plain "filename" formats
        let full_path = if name.contains(':') {
            let parts: Vec<&str> = name.splitn(2, ':').collect();
            if parts.len() == 2 {
                format!("data/mob/{}/{}.json", parts[0], parts[1])
            } else {
                format!("data/mob/{}.json", name)
            }
        } else {
            format!("data/mob/{}.json", name)
        };
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("몹정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_room_data", |name: &str| -> Dynamic {
        let full_path = format!("data/map/{}.json", name.replace(":", "/"));
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("맵정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_skill_data", |name: &str| -> Dynamic {
        match std::fs::read_to_string("data/config/skill.json") {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
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
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    // find_mobs(search_term): 몹 검색. 관리자 명령어용.
    // Returns Array of [zone, room, mob_name, display_name, hp, max_hp]
    engine.register_fn("find_mobs", |search_term: &str| -> rhai::Array {
        use crate::world::get_world_state;
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Array::new(),
        };

        let results = w.search_mobs_by_name(search_term);
        let mut arr = rhai::Array::new();

        for (zone, room, mob_name, display_name, hp, max_hp) in results {
            let mut m = rhai::Map::new();
            m.insert("zone".into(), Dynamic::from(zone));
            m.insert("room".into(), Dynamic::from(room));
            m.insert("mob_name".into(), Dynamic::from(mob_name));
            m.insert("display_name".into(), Dynamic::from(display_name));
            m.insert("hp".into(), Dynamic::from(hp));
            m.insert("max_hp".into(), Dynamic::from(max_hp));
            arr.push(Dynamic::from(m));
        }

        arr
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

    // get_timestamp(): Unix timestamp (초). 값값 등.
    engine.register_fn("get_timestamp", || -> i64 {
        chrono::Utc::now().timestamp()
    });

    // read_text_file(path): 텍스트 파일 내용. 없으면 "".
    engine.register_fn("read_text_file", |path: &str| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    });

    // ============================================================
    // PLAYER DATA ACCESS
    // ============================================================

    engine.register_fn(
        "get_player_data",
        |player_data: &mut rhai::Map, key: &str| -> Dynamic {
            player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
        },
    );

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
#[allow(clippy::too_many_arguments)]
pub fn create_engine_with_body_and_output(
    body: &mut Body,
    output_collector: Arc<Mutex<Vec<String>>>,
    get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
    get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
    special_collector: Arc<Mutex<Option<CommandResult>>>,
    user_sends: Arc<Mutex<Vec<(String, String)>>>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
    script_name: Option<&str>,
    global_data: Option<SharedGlobalData>,
) -> Engine {
    let oc = output_collector.clone();
    let mut engine = create_engine_with_output(output_collector);
    let body_ptr = body as *mut Body;
    let spec = special_collector.clone();

    engine.register_fn(
        "get_bool",
        |player_data: &mut rhai::Map, key: &str| -> bool {
            player_data
                .get(key)
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false)
        },
    );

    engine.register_fn(
        "item_create",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &mut *body_ptr };
            if let Some((arc, name)) = object_from_item_json(key) {
                body.object.append(arc);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
                name
            } else {
                String::new()
            }
        },
    );

    engine.register_fn(
        "item_drop",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
            if name.is_empty() {
                return 0; // 빈 name이 "".contains("")로 전부 매칭되는 것 방지
            }
            let body = unsafe { &mut *body_ptr };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            let mut w = get_world_state().write().unwrap();
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
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
                        let room_stack = w.get_room_objs_stack_mut(&zone, &room);
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
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name));
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
            let room_objs = w.get_room_objs_mut(&zone, &room);
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
        },
    );

    engine.register_fn(
        "item_get",
        move |_ob: &mut rhai::Map, name: &str, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr };
            let count = count.clamp(1, 100) as usize;
            let mut w = get_world_state().write().unwrap();
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return 0,
            };
            let mut taken = 0usize;
            // 스택: room_inv_stack에서 가져와 body.inv_stack에
            if let Some(ref key) = find_item_key_by_name(name) {
                if is_stackable(key) {
                    let room_stack = w.get_room_objs_stack_mut(&zone, &room);
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
            let room_list = w.get_room_objs_mut(&zone, &room);
            let mut i = 0;
            while i < room_list.len() && taken < count {
                let matches = {
                    let o = room_list[i].lock().unwrap();
                    o.getName() == name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name))
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
        },
    );

    engine.register_fn(
        "item_destroy",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            // 스택: inv_stack에서 제거
            if order == 1 {
                if let Some(ref key) = find_item_key_by_name(name) {
                    if is_stackable(key) {
                        let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                        let destroy_cnt = (count as i64).clamp(0, have);
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
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name));
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
        },
    );

    // item_destroy_busha: like item_destroy but skips 부수지못함. Returns -1 if first candidate has it.
    engine.register_fn(
        "item_destroy_busha",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            let mut n = 0usize;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            let mut hit_unbreakable = false;
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let ok = o.getName() == name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name));
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
        },
    );

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
            let pair = vec![rhai::Dynamic::from(k), rhai::Dynamic::from(v)];
            arr.push(rhai::Dynamic::from(pair));
        }
        arr
    });

    // list_inventory_of_player(ob, target_name): 관리자가 같은 방 다른 플레이어의 소지품 확인.
    // {ok, items, err}. ok=true면 items=[["이름",개수],...]. err="not_admin"|"not_found"|"not_same_room"|"no_permission".
    let body_ptr_inv_other = body_ptr;
    engine.register_fn(
        "list_inventory_of_player",
        move |ob: &mut rhai::Map, target_name: &str| -> Dynamic {
            let admin = ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0i64);
            if admin < 1000 {
                let mut m = rhai::Map::new();
                m.insert("ok".into(), Dynamic::from(false));
                m.insert("err".into(), Dynamic::from("not_admin"));
                return Dynamic::from(m);
            }

            let body = unsafe { &*body_ptr_inv_other };
            let viewer_name = body.get_name();

            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => {
                    let mut m = rhai::Map::new();
                    m.insert("ok".into(), Dynamic::from(false));
                    m.insert("err".into(), Dynamic::from("no_position"));
                    return Dynamic::from(m);
                }
            };

            // 같은 방에 있는지 확인
            let viewer_pos = match w.get_player_position(viewer_name.as_str()) {
                Some(p) => p,
                None => {
                    let mut m = rhai::Map::new();
                    m.insert("ok".into(), Dynamic::from(false));
                    m.insert("err".into(), Dynamic::from("no_position"));
                    return Dynamic::from(m);
                }
            };

            let players_in_room = w.get_players_in_room(&viewer_pos.zone, &viewer_pos.room);
            let target_found = players_in_room.iter().any(|n| n == target_name);
            if !target_found {
                let mut m = rhai::Map::new();
                m.insert("ok".into(), Dynamic::from(false));
                m.insert("err".into(), Dynamic::from("not_same_room"));
                return Dynamic::from(m);
            }

            // 타겟 플레이어의 데이터 가져오기
            // TODO: 플레이어 간 소지품 보기 기능은 복잡하므로 추후 구현
            // 일단 빈 결과 반환 (관리자 기능은 아직 미지원)

            let mut m = rhai::Map::new();
            m.insert("ok".into(), Dynamic::from(true));
            m.insert("err".into(), Dynamic::from(""));
            m.insert("items".into(), Dynamic::from(rhai::Array::new()));
            Dynamic::from(m)
        },
    );

    // get_merchant_script(ob): 현재 방의 상인(물건판매) 몹의 물건판매스크립을 "\r\n"으로 이어서 반환. 없으면 "".
    let body_ptr_merchant = body_ptr;
    engine.register_fn(
        "get_merchant_script",
        move |_ob: &mut rhai::Map| -> String {
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
        },
    );

    // get_merchant_buy_percent(ob): 현재 방의 물건구입 상인 몹의 구입 비율(1–100 등). 없으면 0.
    let body_ptr_buy = body_ptr;
    engine.register_fn(
        "get_merchant_buy_percent",
        move |_ob: &mut rhai::Map| -> i64 {
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
        },
    );

    // merchant_buy(ob, name, count): 기능만. {err: ""|"usage"|"no_merchant"|"not_for_sale"|"inv_full"|"too_heavy"|"no_money", bought, display_name, total_cost}. 오류 메시지는 Rhai에서.
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
                m.insert("err".into(), Dynamic::from("usage".to_string()));
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
                    m.insert("err".into(), Dynamic::from("no_merchant".to_string()));
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
                    let Some((iname, rn, price, wg)) = get_item_info(key) else {
                        continue;
                    };
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
                m.insert("err".into(), Dynamic::from("not_for_sale".to_string()));
                m.insert("bought".into(), Dynamic::from(0i64));
                m.insert("display_name".into(), Dynamic::from(display_name));
                m.insert("total_cost".into(), Dynamic::from(0i64));
                return Dynamic::from(m);
            }
            let cnt = count.clamp(1, 50);
            const MAX_ITEMS: usize = 50;
            let is_admin = body.get_int("관리자등급") >= 1000;
            for _ in 0..cnt {
                // 관리자는 무게/수량 제한 없음
                if !is_admin {
                    if body.get_item_count() >= MAX_ITEMS {
                        if bought == 0 {
                            err = "inv_full".to_string();
                        }
                        break;
                    }
                    if body.get_item_weight() + weight > body.get_str() * 10 {
                        if bought == 0 {
                            err = "too_heavy".to_string();
                        }
                        break;
                    }
                }
                if body.get_int("은전") < unit_price {
                    if bought == 0 {
                        err = "no_money".to_string();
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
        move |_ob: &mut rhai::Map,
              name: &str,
              order: i64,
              count: i64,
              percent: i64|
              -> rhai::Array {
            use rhai::Dynamic;
            let body = unsafe { &mut *body_ptr_sell };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            let percent = percent.max(0);
            // 스택: order==1일 때 inv_stack에서 판매
            if order == 1 {
                if let Some(ref key) = find_item_key_by_name(name) {
                    if is_stackable(key) {
                        let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                        let sell_cnt = (count as i64).clamp(0, have);
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
                                return vec![
                                    Dynamic::from(sell_cnt),
                                    Dynamic::from(total),
                                    Dynamic::from(iname),
                                    Dynamic::from(""),
                                ];
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
                    if !match_ || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함")
                    {
                        continue;
                    }
                    n += 1;
                    if n < order {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "팔지못함") {
                        return vec![
                            Dynamic::from(0i64),
                            Dynamic::from(0i64),
                            Dynamic::from(String::new()),
                            Dynamic::from("cant_sell".to_string()),
                        ];
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
            vec![
                Dynamic::from(to_remove.len() as i64),
                Dynamic::from(total),
                Dynamic::from(display_name),
                Dynamic::from(err),
            ]
        },
    );

    // view_map_data(ob): 기능만. {ok, text, err, zone, room}. ok=true면 text에 방 문자열. err="no_position"|"room_error"|"unknown_room". 출력은 Rhai에서 send_line.
    let get_other = get_other_players_desc;
    engine.register_fn("view_map_data", move |ob: &mut rhai::Map| -> Dynamic {
        let name: String = ob
            .get("이름")
            .or_else(|| ob.get("name"))
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let others = get_other.as_ref().map(|f| f(&name)).unwrap_or_default();
        let mut m = rhai::Map::new();
        match build_room_lines(&name, &others) {
            Ok(text) => {
                m.insert("ok".into(), Dynamic::from(true));
                m.insert("text".into(), Dynamic::from(text));
                m.insert("err".into(), Dynamic::from(String::new()));
                m.insert("zone".into(), Dynamic::from(String::new()));
                m.insert("room".into(), Dynamic::from(""));
            }
            Err((err, zone, room)) => {
                m.insert("ok".into(), Dynamic::from(false));
                m.insert("text".into(), Dynamic::from(String::new()));
                m.insert("err".into(), Dynamic::from(err));
                m.insert("zone".into(), Dynamic::from(zone));
                m.insert("room".into(), Dynamic::from(room));
            }
        }
        Dynamic::from(m)
    });

    // find_target(ob, line): [대상] 봐. {found, lines, to_target, err}. err=""|"no_position"|"not_found". 오류 메시지는 Rhai에서.
    let body_ptr_ft = body_ptr;
    let get_other_map_ft = get_other_players_map.clone();
    engine.register_fn(
        "find_target",
        move |ob: &mut rhai::Map, line: &str| -> Dynamic {
            let viewer_name: String = ob
                .get("이름")
                .or_else(|| ob.get("name"))
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let world = get_world_state().read().unwrap();
            let other = get_other_map_ft.as_ref().map(|f| f()).unwrap_or_default();
            let (lines, to_target) =
                look_at_target(unsafe { &*body_ptr_ft }, &world, &viewer_name, line, &other);
            let (err, lines_out) = if lines.len() == 1 {
                if lines[0].contains("위치 정보가 없습니다") {
                    ("no_position".to_string(), vec![])
                } else if lines[0].contains("안광으로는 그런것을 볼수 없다") {
                    ("not_found".to_string(), vec![])
                } else {
                    (String::new(), lines)
                }
            } else {
                (String::new(), lines)
            };
            let found = to_target.is_some() || (!lines_out.is_empty() && err.is_empty());
            let mut m = rhai::Map::new();
            m.insert("found".into(), Dynamic::from(found));
            m.insert("err".into(), Dynamic::from(err));
            m.insert(
                "lines".into(),
                Dynamic::from(rhai::Array::from_iter(
                    lines_out.into_iter().map(Dynamic::from),
                )),
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
        },
    );

    // get_all_online_players(): 전 접속자 목록. [{"이름","무림별호","성격","레벨초기화","소속","설정상태"}, ...]. 누구 스크립트용.
    engine.register_fn("get_all_online_players", get_precomputed_all_online);
    engine.register_fn("get_online_names", get_online_names);
    engine.register_fn("user_refuses_shout", user_refuses_shout);

    // get_user_config(ob, 키), set_user_config(ob, 키, 값): 영구 저장 사용자 설정. ob["설정"][키]=값. 설정상태 파싱/저장.
    let body_ptr_cfg = body_ptr;
    engine.register_fn(
        "get_user_config",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_cfg };
            parse_config_string(&body.get_string("설정상태"))
                .get(key)
                .cloned()
                .unwrap_or_default()
        },
    );
    let body_ptr_cfg2 = body_ptr;
    engine.register_fn(
        "set_user_config",
        move |_ob: &mut rhai::Map, key: &str, value: &str| {
            let body = unsafe { &mut *body_ptr_cfg2 };
            let mut m = parse_config_string(&body.get_string("설정상태"));
            m.insert(key.to_string(), value.to_string());
            body.object.attr.insert(
                "설정상태".to_string(),
                Value::String(format_config_string(&m)),
            );
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        },
    );

    // get_user_event(ob, 키), set_user_event(ob, 키, 값), del_user_event(ob, 키): 임무 등 이벤트. ob["이벤트"][키]=값. 이벤트설정리스트.
    let body_ptr_ev = body_ptr;
    engine.register_fn(
        "get_user_event",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_ev };
            parse_event_string(&body.get_string("이벤트설정리스트"))
                .get(key)
                .cloned()
                .unwrap_or_default()
        },
    );
    let body_ptr_ev2 = body_ptr;
    engine.register_fn(
        "set_user_event",
        move |_ob: &mut rhai::Map, key: &str, value: &str| {
            let body = unsafe { &mut *body_ptr_ev2 };
            let mut m = parse_event_string(&body.get_string("이벤트설정리스트"));
            if value.is_empty() {
                m.remove(key);
            } else {
                m.insert(key.to_string(), value.to_string());
            }
            body.object.attr.insert(
                "이벤트설정리스트".to_string(),
                Value::String(format_event_string(&m)),
            );
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        },
    );
    let body_ptr_ev3 = body_ptr;
    engine.register_fn("del_user_event", move |_ob: &mut rhai::Map, key: &str| {
        let body = unsafe { &mut *body_ptr_ev3 };
        let mut m = parse_event_string(&body.get_string("이벤트설정리스트"));
        m.remove(key);
        body.object.attr.insert(
            "이벤트설정리스트".to_string(),
            Value::String(format_event_string(&m)),
        );
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);
    });

    // check_mob_event(mob_key, event_key) - Check if mob has event (Python: target.checkEvent)
    engine.register_fn("check_mob_event", |mob_key: &str, event_key: &str| -> bool {
        let cache = crate::world::mob::get_mob_cache();
        if let Ok(cache_guard) = cache.read() {
            cache_guard.check_mob_event(mob_key, event_key)
        } else {
            false
        }
    });

    // set_mob_event(mob_key, event_key) - Set event on mob (Python: target.setEvent)
    engine.register_fn("set_mob_event", |mob_key: &str, event_key: &str| -> bool {
        let cache = crate::world::mob::get_mob_cache();
        if let Ok(mut cache_guard) = cache.write() {
            cache_guard.set_mob_event(mob_key, event_key)
        } else {
            false
        }
    });

    // del_mob_event(mob_key, event_key) - Delete event from mob (Python: target.delEvent)
    engine.register_fn("del_mob_event", |mob_key: &str, event_key: &str| -> bool {
        let cache = crate::world::mob::get_mob_cache();
        if let Ok(mut cache_guard) = cache.write() {
            cache_guard.del_mob_event(mob_key, event_key)
        } else {
            false
        }
    });

    // get_admin_level(ob) - Get player's admin level (관리자등급)
    let body_ptr_admin = body_ptr;
    engine.register_fn("get_admin_level", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_admin };
        crate::command::handler::helpers::get_admin_level(body)
    });

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
            m.insert("room".into(), Dynamic::from(p.room.clone()));
        } else {
            m.insert("zone".into(), Dynamic::from(""));
            m.insert("room".into(), Dynamic::from(0i64));
        }
        Dynamic::from(m)
    });

    // get_room_name(zone, room) -> 방 이름 문자열. 어디 등.
    // i64 버전
    engine.register_fn("get_room_name", |zone: &str, room: i64| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return format!("{}:{}", zone, room),
        };
        let r = w.room_cache.get_room_cached(zone, &room.to_string());
        match r {
            Some(arc) => {
                let guard = arc.read().unwrap();
                if guard.display_name.is_empty() {
                    guard.name.clone()
                } else {
                    guard.display_name.clone()
                }
            }
            None => format!("{}:{}", zone, room),
        }
    });

    // get_room_name(zone, room) -> 방 이름 문자열. 어디 등.
    // &str 버전 (room이 문자열인 경우)
    engine.register_fn("get_room_name", |zone: &str, room: &str| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return format!("{}:{}", zone, room),
        };
        let r = w.room_cache.get_room_cached(zone, room);
        match r {
            Some(arc) => {
                let guard = arc.read().unwrap();
                if guard.display_name.is_empty() {
                    guard.name.clone()
                } else {
                    guard.display_name.clone()
                }
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
                if !o.getBool("inUse") {
                    continue;
                }
                let slot = o.getString("계층");
                if slot.is_empty() {
                    continue;
                }
                pairs.push((slot, o.getName()));
            }
        }
        pairs.sort_by_cached_key(|(s, _)| {
            ITEM_EQUIP_LEVELS
                .iter()
                .position(|&l| l == s.as_str())
                .unwrap_or(999)
        });
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
    engine.register_fn(
        "set_act",
        move |_ob: &mut rhai::Map, state: rhai::Dynamic| {
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
        },
    );

    // has_room_property(zone, room, prop): 방 맵속성에 prop 포함 여부. 쉬어(쉼금지) 등.
    engine.register_fn(
        "has_room_property",
        |zone: &str, room: i64, prop: &str| -> bool {
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
        },
    );

    // has_room_property(zone, room, prop): &str 버전 (room이 문자열인 경우)
    engine.register_fn(
        "has_room_property",
        |zone: &str, room: &str, prop: &str| -> bool {
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return false,
            };
            if let Some(arc) = w.room_cache.get_room_cached(zone, room) {
                if let Ok(r) = arc.read() {
                    return r.properties.iter().any(|p| p == prop);
                }
            }
            false
        },
    );

    // get_exits_string(zone, room): 출구 나침반 문자열. 지도/맵 등.
    // i64 버전
    engine.register_fn("get_exits_string", |zone: &str, room: i64| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return String::new(),
        };
        if let Some(arc) = w.room_cache.get_room_cached(zone, &room.to_string()) {
            if let Ok(r) = arc.read() {
                return format_exits_long(&r);
            }
        }
        String::new()
    });

    // get_exits_string(zone, room): 출구 나침반 문자열. 지도/맵 등.
    // &str 버전 (room이 문자열인 경우)
    engine.register_fn("get_exits_string", |zone: &str, room: &str| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return String::new(),
        };
        if let Some(arc) = w.room_cache.get_room_cached(zone, room) {
            if let Ok(r) = arc.read() {
                return format_exits_long(&r);
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
            m.insert("room".into(), Dynamic::from(p.room.clone()));
        } else {
            m.insert("zone".into(), Dynamic::from(String::new()));
            m.insert("room".into(), Dynamic::from(0i64));
        }
        Dynamic::from(m)
    });

    // set_my_position(ob, zone, room): 기능만. ""=성공, "fail"|"same_place". 오류 메시지는 Rhai에서.
    let body_ptr_setpos = body_ptr;
    engine.register_fn(
        "set_my_position",
        move |_ob: &mut rhai::Map, zone: &str, room: rhai::Dynamic| -> String {
            let body = unsafe { &*body_ptr_setpos };
            let name = body.get_name();
            if name.is_empty() {
                return "fail".to_string();
            }
            let room_s = room.to_string();
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "fail".to_string(),
            };
            let cur = w.get_player_position(&name).cloned();
            let (cz, cr) = cur
                .as_ref()
                .map(|p| (p.zone.as_str(), p.room.as_str()))
                .unwrap_or(("", "0"));
            if cz == zone && cr == room_s {
                return "same_place".to_string();
            }
            if w.room_cache.get_room(zone, &room_s).is_err() {
                return "fail".to_string();
            }
            w.set_player_position(&name, PlayerPosition::new(zone.to_string(), room_s.clone()));
            w.spawn_mobs_for_room(zone, &room_s);
            String::new()
        },
    );

    // set_value(ob, key, val): Body에 키-값 저장. 점프(cooltime) 등. val은 정수 또는 문자열.
    let body_ptr_setv = body_ptr;
    engine.register_fn(
        "set_value",
        move |_ob: &mut rhai::Map, key: &str, val: rhai::Dynamic| {
            let body = unsafe { &mut *body_ptr_setv };
            if val.is_int() {
                body.set(key, val.as_int().unwrap_or(0));
            } else {
                body.set(key, val.to_string());
            }
        },
    );

    // set_obj_attr(ob, target, key, val): 기능만. 대상에 속성 설정. 성공 true. 오류 메시지는 Rhai에서 send_line.
    let body_ptr_soa = body_ptr;
    engine.register_fn(
        "set_obj_attr",
        move |ob: &mut rhai::Map, target: &str, key: &str, val: rhai::Dynamic| -> bool {
            let body = unsafe { &mut *body_ptr_soa };
            let my_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let val_str = if val.is_int() {
                val.as_int().unwrap_or(0).to_string()
            } else {
                val.to_string()
            };
            let v: crate::object::Value = if val.is_int() {
                (val.as_int().unwrap_or(0)).into()
            } else {
                val_str.as_str().into()
            };
            if target == "방" {
                let pos = match get_world_state()
                    .read()
                    .ok()
                    .and_then(|w| w.get_player_position(&my_name).cloned())
                {
                    Some(p) => p,
                    None => return false,
                };
                get_world_state()
                    .write()
                    .unwrap()
                    .get_room_attrs_mut(&pos.zone, &pos.room)
                    .insert(key.to_string(), val_str);
                return true;
            }
            if target == "나" || target == my_name {
                body.set(key, v);
                return true;
            }
            for arc in &body.object.objs {
                if let Ok(o) = arc.lock() {
                    if o.getName() == target
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(target))
                    {
                        drop(o);
                        if let Ok(mut obj) = arc.lock() {
                            obj.set(key, v);
                        }
                        return true;
                    }
                }
            }
            if let Some((zone, room)) = get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&my_name)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                let mut w = get_world_state().write().unwrap();
                let room_list = w.get_room_objs_mut(&zone, &room);
                for arc in room_list.iter_mut() {
                    if let Ok(o) = arc.lock() {
                        if o.getName() == target
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(target))
                        {
                            drop(o);
                            if let Ok(mut obj) = arc.lock() {
                                obj.set(key, v);
                            }
                            return true;
                        }
                    }
                }
            }
            false
        },
    );

    // del_obj_attr(ob, target, key): 기능만. 대상에서 속성 삭제. 성공 true. 오류 메시지는 Rhai에서 send_line.
    let body_ptr_doa = body_ptr;
    engine.register_fn(
        "del_obj_attr",
        move |ob: &mut rhai::Map, target: &str, key: &str| -> bool {
            let body = unsafe { &mut *body_ptr_doa };
            let my_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            if target == "방" {
                let (zone, room) = match get_world_state().read().ok().and_then(|w| {
                    w.get_player_position(&my_name)
                        .map(|p| (p.zone.clone(), p.room.clone()))
                }) {
                    Some(x) => x,
                    None => return false,
                };
                let mut w = get_world_state().write().unwrap();
                let attrs = w.get_room_attrs_mut(&zone, &room);
                return attrs.remove(key).is_some();
            }
            if target == "나" || target == my_name {
                return body.attr_mut().remove(key).is_some();
            }
            for arc in &body.object.objs {
                if let Ok(o) = arc.lock() {
                    if o.getName() == target
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(target))
                    {
                        drop(o);
                        if let Ok(mut obj) = arc.lock() {
                            return obj.attr.remove(key).is_some();
                        }
                    }
                }
            }
            if let Some((zone, room)) = get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&my_name)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                let mut w = get_world_state().write().unwrap();
                let room_list = w.get_room_objs_mut(&zone, &room);
                for arc in room_list.iter_mut() {
                    if let Ok(o) = arc.lock() {
                        if o.getName() == target
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(target))
                        {
                            drop(o);
                            if let Ok(mut obj) = arc.lock() {
                                return obj.attr.remove(key).is_some();
                            }
                        }
                    }
                }
            }
            false
        },
    );

    // exit_hide(ob, name): 기능만. 출구숨김. 성공 true. 오류 메시지는 Rhai에서 send_line.
    engine.register_fn("exit_hide", move |ob: &mut rhai::Map, name: &str| -> bool {
        let name_body = ob
            .get("이름")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let (zone, room) = match get_world_state().read().ok().and_then(|w| {
            w.get_player_position(&name_body)
                .map(|p| (p.zone.clone(), p.room.clone()))
        }) {
            Some(x) => x,
            None => return false,
        };
        let mut w = match get_world_state().write() {
            Ok(g) => g,
            Err(_) => return false,
        };
        let room_arc = match w.room_cache.get_room(&zone, &room.to_string()) {
            Ok(r) => r,
            Err(_) => return false,
        };
        let ok = room_arc.write().unwrap().set_exit_hidden(name, true);
        ok
    });

    // exit_show(ob, name): 출구 드러냄. 성공 true.
    let _oc_es = oc.clone();
    engine.register_fn("exit_show", move |ob: &mut rhai::Map, name: &str| -> bool {
        let name_body = ob
            .get("이름")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let (zone, room) = match get_world_state().read().ok().and_then(|w| {
            w.get_player_position(&name_body)
                .map(|p| (p.zone.clone(), p.room.clone()))
        }) {
            Some(x) => x,
            None => return false,
        };
        let mut w = match get_world_state().write() {
            Ok(g) => g,
            Err(_) => return false,
        };
        let room_arc = match w.room_cache.get_room(&zone, &room.to_string()) {
            Ok(r) => r,
            Err(_) => return false,
        };
        let ok = room_arc.write().unwrap().set_exit_hidden(name, false);
        ok
    });

    // exit_remove(ob, name): 기능만. 출구제거. 성공 true. 오류 메시지는 Rhai에서 send_line.
    engine.register_fn(
        "exit_remove",
        move |ob: &mut rhai::Map, name: &str| -> bool {
            let name_body = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let (zone, room) = match get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&name_body)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                Some(x) => x,
                None => return false,
            };
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let room_arc = match w.room_cache.get_room(&zone, &room) {
                Ok(r) => r,
                Err(_) => return false,
            };
            let ok = room_arc.write().unwrap().remove_exit(name);
            ok
        },
    );

    // exit_set_wander(ob, name): 기능만. 맴돌이. 출구 목적지를 자기 방으로. 성공 true. 오류 메시지는 Rhai에서 send_line.
    engine.register_fn(
        "exit_set_wander",
        move |ob: &mut rhai::Map, name: &str| -> bool {
            let name_body = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let (zone, room) = match get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&name_body)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                Some(x) => x,
                None => return false,
            };
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let room_arc = match w.room_cache.get_room(&zone, &room) {
                Ok(r) => r,
                Err(_) => return false,
            };
            let ok = room_arc
                .write()
                .unwrap()
                .set_exit_destination(name, &zone, &room);
            ok
        },
    );

    // mob_regen(ob, name): 리젠. 시체만 가능. 성공 true.
    engine.register_fn("mob_regen", move |ob: &mut rhai::Map, name: &str| -> bool {
        let name_body = ob
            .get("이름")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let (zone, room) = match get_world_state().read().ok().and_then(|w| {
            w.get_player_position(&name_body)
                .map(|p| (p.zone.clone(), p.room.clone()))
        }) {
            Some(x) => x,
            None => return false,
        };
        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .do_regen(&zone, &room, name)
    });

    // guild_get(id, key), guild_set(id, key, value), guild_attr_keys(id), guild_list(), guild_has(id), guild_remove(id), guild_save()
    // guild_list는 Vec<String> 대신 rhai::Array 반환 (len, [] 연산 호환)
    engine.register_fn("guild_get", guild_get);
    engine.register_fn("guild_set", guild_set);
    engine.register_fn("guild_attr_keys", guild_attr_keys);
    engine.register_fn("guild_list", || -> rhai::Array {
        guild_list().into_iter().map(rhai::Dynamic::from).collect()
    });
    engine.register_fn("guild_has", guild_has);
    engine.register_fn("guild_remove", guild_remove);
    engine.register_fn("guild_save", guild_save);

    // rank_write(ty, name, value, level), rank_read(ty, name), rank_get_num(ty, rank), rank_get_all(ty), rank_clear(ty). ty 빈 문자열이면 전체.
    engine.register_fn("rank_write", rank_write);
    engine.register_fn("rank_read", rank_read);
    engine.register_fn("rank_get_num", rank_get_num);
    engine.register_fn("rank_get_all", rank_get_all);
    engine.register_fn("rank_clear", rank_clear);

    // password_hash(plain): 평문을 SHA-512 해시 16진수 문자열로. 암호 저장/암호변경용.
    engine.register_fn("password_hash", |plain: &str| -> String {
        password_hash(plain)
    });
    // password_verify(stored, plain): 저장된 해시(또는 레거시 평문)와 평문 일치 여부. 암호변경 검증용.
    engine.register_fn("password_verify", |stored: &str, plain: &str| -> bool {
        password_verify(stored, plain)
    });
    // verify_password(ob, plain): Body 암호와 평문 일치 여부. 해시를 스크립트에 노출하지 않고 검증.
    let body_ptr_vp = body_ptr;
    engine.register_fn(
        "verify_password",
        move |_ob: &mut rhai::Map, plain: &str| -> bool {
            let body = unsafe { &*body_ptr_vp };
            let stored = body.get_string("암호");
            password_verify(&stored, plain)
        },
    );
    // parse_two_args(s): 첫 공백 기준 [앞, 뒤]. "a b c" -> ["a","b c"]. "a" -> ["a",""].
    engine.register_fn("parse_two_args", |s: &str| -> rhai::Array {
        let parts: Vec<&str> = s.splitn(2, ' ').collect();
        vec![
            rhai::Dynamic::from(parts.first().copied().unwrap_or("").to_string()),
            rhai::Dynamic::from(parts.get(1).copied().unwrap_or("").to_string()),
        ]
    });

    // get_body_int(ob, key): Body에서 정수 읽기. Map에 없는 런타임 키(예: cooltime)용.
    let body_ptr_getbi = body_ptr;
    engine.register_fn(
        "get_body_int",
        move |_ob: &mut rhai::Map, key: &str| -> i64 {
            let body = unsafe { &*body_ptr_getbi };
            body.get_int(key)
        },
    );

    // get_body_string(ob, key): Body에서 문자열 읽기. set_value로 넣은 키(예: 위치각인, 꼬리말)용.
    let body_ptr_getbs = body_ptr;
    engine.register_fn(
        "get_body_string",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_getbs };
            body.get_string(key)
        },
    );

    // ---- 외쳐/전음/표현/주다: special_collector에 CommandResult 설정, handler에서 Shout/Tell/EmotionToRoom/GiveToPlayer 처리 ----
    // send_to_user(name, msg): 해당 접속자에게 msg 전송. 스크립트에서 포맷·조건(외침거부 등) 정한 뒤 호출.

    let user_sends_clone = user_sends.clone();
    engine.register_fn("send_to_user", move |name: &str, msg: &str| {
        if !name.is_empty() && !msg.is_empty() {
            if let Ok(mut u) = user_sends_clone.lock() {
                u.push((name.to_string(), msg.to_string()));
            }
        }
    });

    // send_notice(ob, msg): 기능만. [공지] 이름 : 메시지 형식으로 접속 전원 전송. ""=성공, "usage"=빈 msg. 오류 메시지는 Rhai에서.
    let spec_not = spec.clone();
    let body_ptr_not = body_ptr;
    engine.register_fn(
        "send_notice",
        move |_ob: &mut rhai::Map, msg: &str| -> String {
            if msg.trim().is_empty() {
                return "usage".to_string();
            }
            let body = unsafe { &*body_ptr_not };
            let name = body.get_string("이름");
            let n = if name.is_empty() {
                "관리자"
            } else {
                name.as_str()
            };
            let formatted = format!("[공지] {} : {}", n, msg);
            if let Ok(mut s) = spec_not.lock() {
                *s = Some(CommandResult::Notice(formatted));
            }
            "".to_string()
        },
    );

    // send_broadcast_to_guild(ob, msg): 기능만. [방파] 이름 : 메시지. ""=성공, "usage"=빈 msg, "no_guild"=소속 없음. 오류 메시지는 Rhai에서.
    let spec_bg = spec.clone();
    engine.register_fn(
        "send_broadcast_to_guild",
        move |ob: &mut rhai::Map, msg: &str| -> String {
            if msg.trim().is_empty() {
                return "usage".to_string();
            }
            let my_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let guild = ob
                .get("소속")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            if guild.is_empty() {
                return "no_guild".to_string();
            }
            let arr = get_precomputed_all_online();
            let mut names: Vec<String> = Vec::new();
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    let s: String = m
                        .get("소속")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    if s == guild {
                        if let Some(n) = m
                            .get("이름")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        {
                            if !n.is_empty() {
                                names.push(n);
                            }
                        }
                    }
                }
            }
            let formatted = format!("\x1b[0;35m[방파]\x1b[0;37m {} : {}", my_name, msg);
            if let Ok(mut s) = spec_bg.lock() {
                *s = Some(CommandResult::BroadcastToPlayers(names, formatted));
            }
            "".to_string()
        },
    );

    // send_tell(ob, target, msg): 기능만. ""=성공, "usage"|"tell_refuse"|"self_tell". 오류 메시지는 Rhai에서.
    let spec_te = spec.clone();
    let body_ptr_te = body_ptr;
    engine.register_fn(
        "send_tell",
        move |_ob: &mut rhai::Map, target: &str, msg: &str| -> String {
            let body = unsafe { &*body_ptr_te };
            if target.trim().is_empty() || msg.trim().is_empty() {
                return "usage".to_string();
            }
            let config = body.get_string("설정상태");
            if config.contains("전음거부 1") {
                return "tell_refuse".to_string();
            }
            if target == body.get_name() {
                return "self_tell".to_string();
            }
            if let Ok(mut s) = spec_te.lock() {
                *s = Some(CommandResult::Tell(target.to_string(), msg.to_string()));
            }
            "".to_string()
        },
    );

    // send_emotion(ob, action): 기능만. to_self/to_room 설정. ""=성공, "usage"=빈 action. 오류 메시지는 Rhai에서.
    let spec_em = spec.clone();
    let body_ptr_em = body_ptr;
    engine.register_fn(
        "send_emotion",
        move |_ob: &mut rhai::Map, action: &str| -> String {
            let body = unsafe { &*body_ptr_em };
            if action.trim().is_empty() {
                return "usage".to_string();
            }
            let name = body.get_name();
            let iga = han_iga(&name);
            let to_self = format!("당신이 {}", action);
            let to_room = format!("{}{} {}", name, iga, action);
            if let Ok(mut s) = spec_em.lock() {
                *s = Some(CommandResult::EmotionToRoom(to_self, to_room, None));
            }
            "".to_string()
        },
    );

    // request_give_silver(ob, target, amt): 기능만. ""=성공, "usage"|"no_money". 오류 메시지는 Rhai에서.
    let spec_gs = spec.clone();
    let body_ptr_gs = body_ptr;
    engine.register_fn(
        "request_give_silver",
        move |_ob: &mut rhai::Map, target: &str, amt: i64| -> String {
            let body = unsafe { &*body_ptr_gs };
            if amt < 1 {
                return "usage".to_string();
            }
            let have = body.get_int("은전");
            let give = amt.min(have.max(0));
            if give < 1 {
                return "no_money".to_string();
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
        },
    );

    // request_give_gold(ob, target, amt): 기능만. ""=성공, "usage"|"no_money". 오류 메시지는 Rhai에서.
    let spec_gg = spec.clone();
    let body_ptr_gg = body_ptr;
    engine.register_fn(
        "request_give_gold",
        move |_ob: &mut rhai::Map, target: &str, amt: i64| -> String {
            let body = unsafe { &*body_ptr_gg };
            if amt < 1 {
                return "usage".to_string();
            }
            let have = body.get_int("금전");
            let give = amt.min(have.max(0));
            if give < 1 {
                return "no_money".to_string();
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
        },
    );

    // request_give_item(ob, target, name, order, count): 기능만. ""=성공, "no_item". 오류 메시지는 Rhai에서.
    let spec_gi = spec.clone();
    let body_ptr_gi = body_ptr;
    engine.register_fn(
        "request_give_item",
        move |_ob: &mut rhai::Map,
              target: &str,
              item_name: &str,
              order: i64,
              count: i64|
              -> String {
            let body = unsafe { &*body_ptr_gi };
            let order = order.max(1) as usize;
            let cnt = if order > 1 {
                1i64
            } else {
                count.clamp(1, 50)
            };
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
                return "no_item".to_string();
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
        },
    );

    // item_equip(ob, name, order): 기능만. ""=성공, "usage"|"no_item"|"not_equippable"|"slot_used". 오류 메시지는 Rhai에서.
    // 아이템 착용 시 모든 속성 보너스가 플레이어에게 적용됨
    let body_ptr_equip = body_ptr;
    engine.register_fn(
        "item_equip",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> String {
            if name.is_empty() {
                return "usage".to_string();
            }
            let order = order.max(1) as usize;
            let body = unsafe { &mut *body_ptr_equip };
            let arc = match body.object.findObjInven(name, order) {
                Some(a) => a,
                None => return "no_item".to_string(),
            };
            // 아이템의 모든 속성 수집
            let (kind, slot, stats) = {
                let o = arc.lock().unwrap();
                let k = o.getString("종류");
                let s = o.getString("계층");
                if k != "방어구" && k != "무기" {
                    return "not_equippable".to_string();
                }
                let stats = EquipStats {
                    attack: o.getInt("공격력") as i32,
                    defense: o.getInt("방어력") as i32,
                    strength: o.getInt("힘") as i32,
                    dexterity: o.getInt("민첩") as i32,
                    armor: o.getInt("맷집") as i32,
                    max_hp: o.getInt("체력") as i32,
                    max_mp: o.getInt("내공") as i32,
                    hit: o.getInt("명중") as i32,
                    miss: o.getInt("회피") as i32,
                    critical: o.getInt("치명") as i32,
                    luck: o.getInt("운") as i32,
                };
                (k, s, stats)
            };
            let slot_used = body.object.objs.iter().any(|obj| {
                if std::sync::Arc::ptr_eq(obj, &arc) {
                    return false;
                }
                obj.lock()
                    .map(|x| x.getBool("inUse") && x.getString("계층") == slot)
                    .unwrap_or(false)
            });
            if slot_used && !slot.is_empty() {
                return "slot_used".to_string();
            }
            {
                let mut o = arc.lock().unwrap();
                o.set("inUse", 1i64);
            }
            // 모든 속성 보너스 적용
            body.attpower += stats.attack;
            body.armor += stats.defense;
            body._str += stats.strength;
            body._dex += stats.dexterity;
            body._arm += stats.armor;
            body._maxhp += stats.max_hp;
            body._maxmp += stats.max_mp;
            body._hit += stats.hit;
            body._miss += stats.miss;
            body._critical += stats.critical;
            body._critical_chance += stats.luck;
            if kind == "무기" {
                body.weapon_item = Some(std::sync::Arc::downgrade(&arc));
            }
            String::new()
        },
    );

    // item_unequip(ob, name, order): 기능만. ""=성공, "usage"|"no_item". 오류 메시지는 Rhai에서.
    // 아이템 해제 시 모든 속성 보너스 제거
    let body_ptr_ue = body_ptr;
    engine.register_fn(
        "item_unequip",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> String {
            if name.is_empty() {
                return "usage".to_string();
            }
            let order = order.max(1) as usize;
            let body = unsafe { &mut *body_ptr_ue };
            let arc = match body.object.findObjInUse(name, order) {
                Some(a) => a,
                None => return "no_item".to_string(),
            };
            // 아이템의 모든 속성 수집 및 해제 처리
            let (is_weapon, stats) = {
                let mut o = arc.lock().unwrap();
                o.set("inUse", 0i64);
                let w = o.getString("종류") == "무기";
                let stats = EquipStats {
                    attack: o.getInt("공격력") as i32,
                    defense: o.getInt("방어력") as i32,
                    strength: o.getInt("힘") as i32,
                    dexterity: o.getInt("민첩") as i32,
                    armor: o.getInt("맷집") as i32,
                    max_hp: o.getInt("체력") as i32,
                    max_mp: o.getInt("내공") as i32,
                    hit: o.getInt("명중") as i32,
                    miss: o.getInt("회피") as i32,
                    critical: o.getInt("치명") as i32,
                    luck: o.getInt("운") as i32,
                };
                (w, stats)
            };
            // 모든 속성 보너스 제거 (음수 방지)
            body.attpower = (body.attpower - stats.attack).max(0);
            body.armor = (body.armor - stats.defense).max(0);
            body._str = (body._str - stats.strength).max(0);
            body._dex = (body._dex - stats.dexterity).max(0);
            body._arm = (body._arm - stats.armor).max(0);
            body._maxhp = (body._maxhp - stats.max_hp).max(0);
            body._maxmp = (body._maxmp - stats.max_mp).max(0);
            body._hit = (body._hit - stats.hit).max(0);
            body._miss = (body._miss - stats.miss).max(0);
            body._critical = (body._critical - stats.critical).max(0);
            body._critical_chance = (body._critical_chance - stats.luck).max(0);
            if is_weapon {
                body.weapon_item = None;
            }
            String::new()
        },
    );

    // item_unequip_all(ob): 착용 중인 전부 해제. 해제한 개수 반환.
    let body_ptr_ua = body_ptr;
    engine.register_fn("item_unequip_all", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &mut *body_ptr_ua };
        let n = body
            .object
            .objs
            .iter()
            .filter(|o| o.lock().map(|x| x.getBool("inUse")).unwrap_or(false))
            .count();
        body.unwear_all();
        n as i64
    });

    // item_use_consumable(ob, name, order): 소비성 아이템 사용.
    // 먼저 inv_stack에서 찾고(개수 관리), 없으면 objs에서 찾음.
    // {err: ""|"usage"|"bad_state"|"no_item"|"not_consumable", name}. 오류 메시지는 Rhai에서.
    let body_ptr_cons = body_ptr;
    engine.register_fn(
        "item_use_consumable",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> Dynamic {
            let mut m = rhai::Map::new();
            if name.is_empty() {
                m.insert("err".into(), Dynamic::from("usage".to_string()));
                m.insert("name".into(), Dynamic::from(String::new()));
                return Dynamic::from(m);
            }
            let body = unsafe { &mut *body_ptr_cons };
            if body.act == crate::player::ActState::Rest {
                m.insert("err".into(), Dynamic::from("bad_state".to_string()));
                m.insert("name".into(), Dynamic::from(String::new()));
                return Dynamic::from(m);
            }

            // 1단계: inv_stack에서 아이템 찾기 (개수로 관리되는 소비성 아이템)
            if let Some(key) = find_item_key_by_name(name) {
                if is_stackable(&key) {
                    let have = *body.object.inv_stack.get(&key).unwrap_or(&0);
                    if have > 0 {
                        // 아이템 정보 가져오기
                        let (item_name, hp, mp) = get_consumable_info(&key);
                        if hp == 0 && mp == 0 {
                            // 소비성 아이템이 아님
                            m.insert("err".into(), Dynamic::from("not_consumable".to_string()));
                            m.insert("name".into(), Dynamic::from(String::new()));
                            return Dynamic::from(m);
                        }

                        // HP/MP 회복 적용
                        let max_hp = body.get_max_hp();
                        let max_mp = body.get_max_mp();
                        let cur_hp = body.get_hp();
                        let cur_mp = body.get_mp();
                        let new_hp = (cur_hp + hp).min(max_hp).max(0);
                        let new_mp = (cur_mp + mp).min(max_mp).max(0);
                        body.set("체력", new_hp);
                        body.set("내공", new_mp);

                        // 개수 차감
                        if have <= 1 {
                            body.object.inv_stack.remove(&key);
                        } else {
                            *body.object.inv_stack.get_mut(&key).unwrap() -= 1;
                        }

                        // 저장
                        let path = format!("data/user/{}.json", body.get_name());
                        let _ = save_body_to_json(body, &path);

                        m.insert("err".into(), Dynamic::from(String::new()));
                        m.insert("name".into(), Dynamic::from(item_name));
                        return Dynamic::from(m);
                    }
                }
            }

            // 2단계: objs에서 아이템 찾기 (기존 방식 - 개별 인스턴스)
            let order = order.max(1) as usize;
            let arc = match body.object.findObjInven(name, order) {
                Some(a) => a,
                None => {
                    m.insert("err".into(), Dynamic::from("no_item".to_string()));
                    m.insert("name".into(), Dynamic::from(String::new()));
                    return Dynamic::from(m);
                }
            };
            let (item_name, hp, mp) = {
                let o = arc.lock().unwrap();
                if o.getString("종류") != "먹는것" {
                    m.insert("err".into(), Dynamic::from("not_consumable".to_string()));
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
            body.object
                .objs
                .retain(|x| !std::sync::Arc::ptr_eq(x, &arc));
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            m.insert("err".into(), Dynamic::from(String::new()));
            m.insert("name".into(), Dynamic::from(item_name));
            Dynamic::from(m)
        },
    );

    // body_save(ob): 캐릭터 저장. data/user/{이름}.json 에 저장.
    engine.register_fn("body_save", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr };
        let path = format!("data/user/{}.json", body.get_name());
        save_body_to_json(body, &path)
    });

    // add_stack_item(ob, item_key, count) - 스택 아이템을 inv_stack에 추가
    // 성공 시 true 반환, 실패 시 false
    let body_ptr_stack = body_ptr;
    engine.register_fn(
        "add_stack_item",
        move |_ob: &mut rhai::Map, item_key: &str, count: i64| -> bool {
            if item_key.is_empty() || count <= 0 {
                return false;
            }
            let body = unsafe { &mut *body_ptr_stack };

            // 스택 가능한 아이템인지 확인
            if !is_stackable(item_key) {
                return false;
            }

            // inv_stack에 추가
            *body
                .object
                .inv_stack
                .entry(item_key.to_string())
                .or_insert(0) += count;

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            save_body_to_json(body, &path)
        },
    );

    // get_stack_count(ob, item_key) - inv_stack에서 아이템 개수 조회
    let body_ptr_gs = body_ptr;
    engine.register_fn(
        "get_stack_count",
        move |_ob: &mut rhai::Map, item_key: &str| -> i64 {
            let body = unsafe { &*body_ptr_gs };
            *body.object.inv_stack.get(item_key).unwrap_or(&0)
        },
    );

    // remove_stack_item(ob, item_key, count) - inv_stack에서 아이템 제거
    // 성공 시 true, 실패(부족) 시 false
    let body_ptr_rs = body_ptr;
    engine.register_fn(
        "remove_stack_item",
        move |_ob: &mut rhai::Map, item_key: &str, count: i64| -> bool {
            if item_key.is_empty() || count <= 0 {
                return false;
            }
            let body = unsafe { &mut *body_ptr_rs };

            let have = *body.object.inv_stack.get(item_key).unwrap_or(&0);
            if have < count {
                return false;
            }

            if have == count {
                body.object.inv_stack.remove(item_key);
            } else {
                *body.object.inv_stack.get_mut(item_key).unwrap() -= count;
            }

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            save_body_to_json(body, &path)
        },
    );

    // ONEITEM (단일아이템/기연) 시스템. Python ONEITEM과 동일.
    engine.register_fn("oneitem_get_name", crate::oneitem::oneitem_get_name);
    engine.register_fn("oneitem_get", crate::oneitem::oneitem_get);
    engine.register_fn("oneitem_have", crate::oneitem::oneitem_have);
    engine.register_fn("oneitem_drop", crate::oneitem::oneitem_drop);
    engine.register_fn("oneitem_drop2", crate::oneitem::oneitem_drop2);
    engine.register_fn("oneitem_keep", crate::oneitem::oneitem_keep);
    engine.register_fn("oneitem_destroy", crate::oneitem::oneitem_destroy);
    engine.register_fn("oneitem_check_name", crate::oneitem::oneitem_check_name);
    engine.register_fn("oneitem_check_index", crate::oneitem::oneitem_check_index);
    engine.register_fn("oneitem_list", crate::oneitem::oneitem_list);
    engine.register_fn("oneitem_clear", crate::oneitem::oneitem_clear);
    engine.register_fn("oneitem_attr_keys", crate::oneitem::oneitem_attr_keys);
    engine.register_fn(
        "oneitem_get_index_by_name",
        crate::oneitem::oneitem_get_index_by_name,
    );
    engine.register_fn(
        "oneitem_list_index_entries",
        crate::oneitem::oneitem_list_index_entries,
    );

    // call_out / call_later / remove_call_out — 점프 2초 후 착지 등. script_name이 있을 때만 등록(지연 시 스크립트 함수 실행).
    if let (Some(sched), Some(sn)) = (call_out_scheduler, script_name) {
        let s = sched.clone();
        let script_owned = sn.to_string();
        engine.register_fn(
            "call_out",
            move |target: &str, function: &str, delay: i64| {
                let d = Duration::from_secs(delay.max(0) as u64);
                s.call_out(target, function, d, vec![], Some(script_owned.clone()));
            },
        );
        let s2 = sched.clone();
        let script_owned2 = sn.to_string();
        engine.register_fn(
            "call_later",
            move |target: &str, function: &str, delay: i64| {
                let d = Duration::from_secs(delay.max(0) as u64);
                s2.call_out(target, function, d, vec![], Some(script_owned2.clone()));
            },
        );
        let s3 = sched.clone();
        engine.register_fn(
            "remove_call_out",
            move |target: &str, function: &str| -> bool {
                s3.remove_call_out_by_name(target, function)
            },
        );
    }

    // ============================================================
    // TALK HISTORY FUNCTIONS (대화 기록)
    // ============================================================

    // get_talk_history(ob) -> 배열
    // NPC와의 대화 기록을 가져옵니다.
    engine.register_fn(
        "get_talk_history",
        move |_obj: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &mut *body_ptr };
            let arr: rhai::Array = body
                .talk_history
                .iter()
                .map(|s| rhai::Dynamic::from(s.as_str()))
                .collect();
            arr
        },
    );

    // add_talk_history(ob, key) - 대화 기록 추가
    engine.register_fn(
        "add_talk_history",
        move |_obj: &mut rhai::Map, key: &str| {
            let body = unsafe { &mut *body_ptr };
            if !body.talk_history.contains(&key.to_string()) {
                body.talk_history.push(key.to_string());
            }
        },
    );

    // clear_talk_history(ob) - 대화 기록 초기화
    engine.register_fn("clear_talk_history", move |_obj: &mut rhai::Map| {
        let body = unsafe { &mut *body_ptr };
        body.talk_history.clear();
    });

    // ============================================================
    // BOX / STORAGE FUNCTIONS (보관함)
    // ============================================================

    // is_box(obj) -> bool - 오브젝트가 보관함인지 확인
    // obj는 방에서 찾은 오브젝트의 Map 형태
    engine.register_fn("is_box", move |obj: &mut rhai::Map| -> bool {
        let obj_type = obj.get("종류");
        if let Some(t) = obj_type {
            if let Ok(type_str) = t.clone().into_string() {
                return type_str == "보관함" || type_str == "상자" || type_str == "가방";
            }
        }
        false
    });

    // find_box_in_room(ob) -> String - 방에서 보관함 찾기 (이름 반환)
    engine.register_fn("find_box_in_room", move |_obj: &mut rhai::Map| -> String {
        let body = unsafe { &mut *body_ptr };
        let (zone, room) = if let Ok(w) = crate::world::get_world_state().try_read() {
            if let Some(pos) = w.get_player_position(body.get_name().as_str()) {
                (pos.zone.clone(), pos.room.clone())
            } else {
                return String::new();
            }
        } else {
            return String::new();
        };

        if let Ok(w) = crate::world::get_world_state().try_read() {
            let room_objs = w.get_room_objs(&zone, &room);
            for arc in &room_objs {
                if let Ok(item) = arc.lock() {
                    let item_type = item.getString("종류");
                    if item_type == "보관함" || item_type == "상자" || item_type == "가방" {
                        return item.getName();
                    }
                }
            }
        }
        String::new()
    });

    // get_box_capacity(box_obj) -> int - 보관함 용량 가져오기
    engine.register_fn("get_box_capacity", move |box_obj: &mut rhai::Map| -> i64 {
        let capacity = box_obj.get("보관용량");
        if let Some(c) = capacity {
            if let Ok(n) = c.clone().as_int() {
                return n;
            }
        }
        0
    });

    // box_deposit_money(ob, box_name, amount) -> bool - 보관함에 돈 입금
    engine.register_fn(
        "box_deposit_money",
        move |_ob: &mut rhai::Map, box_name: &str, amount: i64| -> bool {
            if amount <= 0 || box_name.is_empty() {
                return false;
            }

            let body = unsafe { &mut *body_ptr };

            let player_money = body.get_int("은전");
            if player_money < amount {
                return false;
            }

            // 보관함 돈 추가 - 방에서 오브젝트 찾아서 수정
            let (zone, room) = if let Ok(w) = crate::world::get_world_state().try_read() {
                if let Some(pos) = w.get_player_position(body.get_name().as_str()) {
                    (pos.zone.clone(), pos.room.clone())
                } else {
                    return false;
                }
            } else {
                return false;
            };

            let mut found = false;
            if let Ok(w) = crate::world::get_world_state().try_read() {
                let room_objs = w.get_room_objs(&zone, &room);
                for arc in &room_objs {
                    if let Ok(item) = arc.lock() {
                        let item_name = item.getName();
                        if item_name == box_name {
                            drop(item);
                            if let Ok(mut box_lock) = arc.lock() {
                                let current_money = box_lock.getInt("은전");
                                box_lock
                                    .set("은전", crate::object::Value::Int(current_money + amount));
                            }
                            found = true;
                            break;
                        }
                    }
                }
            }

            if !found {
                return false;
            }

            // 플레이어 돈 차감
            body.set("은전", player_money - amount);

            true
        },
    );

    // box_withdraw_money(ob, box_name, amount) -> bool - 보관함에서 돈 출금
    engine.register_fn(
        "box_withdraw_money",
        move |_ob: &mut rhai::Map, box_name: &str, amount: i64| -> bool {
            if amount <= 0 || box_name.is_empty() {
                return false;
            }

            let body = unsafe { &mut *body_ptr };

            // 보관함 돈 확인
            let (zone, room) = if let Ok(w) = crate::world::get_world_state().try_read() {
                if let Some(pos) = w.get_player_position(body.get_name().as_str()) {
                    (pos.zone.clone(), pos.room.clone())
                } else {
                    return false;
                }
            } else {
                return false;
            };

            let mut box_money = 0i64;
            let mut found_arc: Option<std::sync::Arc<std::sync::Mutex<crate::object::Object>>> =
                None;

            if let Ok(w) = crate::world::get_world_state().try_read() {
                let room_objs = w.get_room_objs(&zone, &room);
                for arc in &room_objs {
                    if let Ok(item) = arc.lock() {
                        let item_name = item.getName();
                        if item_name == box_name {
                            box_money = item.getInt("은전");
                            found_arc = Some(arc.clone());
                            break;
                        }
                    }
                }
            }

            if found_arc.is_none() {
                return false;
            }

            if box_money < amount {
                return false;
            }

            // 보관함 돈 차감
            if let Some(arc) = found_arc {
                if let Ok(mut box_lock) = arc.lock() {
                    let new_money = box_money - amount;
                    box_lock.set("은전", crate::object::Value::Int(new_money));
                }
            }

            // 플레이어 돈 추가
            let player_money = body.get_int("은전");
            body.set("은전", player_money + amount);

            true
        },
    );

    // get_box_money(box_obj) -> int - 보관함의 돈 확인
    engine.register_fn("get_box_money", move |box_obj: &mut rhai::Map| -> i64 {
        let money = box_obj.get("은전");
        if let Some(m) = money {
            if let Ok(n) = m.clone().as_int() {
                return n;
            }
        }
        0
    });

    // ============================================================
    // 몹/오브젝트 관련 efun (스크립트용)
    // ============================================================

    // find_mob_in_room(ob, mob_name) - 현재 방에서 몹 찾기
    // 몹이 있으면 몹 데이터를 반환, 없으면 UNIT
    let body_ptr_mob = body_ptr;
    engine.register_fn(
        "find_mob_in_room",
        move |ob: &mut rhai::Map, mob_name: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_mob };

            // 플레이어 이름과 위치 가져오기
            let player_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();

            if player_name.is_empty() {
                return Dynamic::UNIT;
            }

            // 위치 정보 파싱 (zone/room 형식)
            let location = body.get_string("위치");
            let parts: Vec<&str> = location.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Dynamic::UNIT;
            }
            let zone = parts[0];
            let room = parts[1];

            // WorldState에서 현재 방의 몹 검색
            if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(zone, room);

                // mob_name으로 검색 (이름 또는 반응 이름 일치)
                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    // 몹 데이터로 표시 이름 확인
                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let mob_name_lower = mob_name.to_lowercase();
                        let display_name_lower = display_name.to_lowercase();

                        // 정확히 일치하거나 포함
                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            // 몹 데이터 반환
                            let mut mob_info = rhai::Map::new();
                            mob_info.insert("이름".into(), Dynamic::from(mob_data.name.clone()));
                            mob_info.insert("표시".into(), Dynamic::from(display_name.clone()));
                            mob_info.insert("hp".into(), Dynamic::from(mob.hp));
                            mob_info.insert("max_hp".into(), Dynamic::from(mob.max_hp));
                            mob_info.insert("level".into(), Dynamic::from(mob_data.level));
                            mob_info.insert("zone".into(), Dynamic::from(mob.zone.clone()));
                            mob_info.insert("room".into(), Dynamic::from(mob.room.clone()));
                            mob_info.insert("mob_key".into(), Dynamic::from(mob.mob_key.clone()));
                            return Dynamic::from(mob_info);
                        }

                        // 반응 이름들도 확인
                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                let mut mob_info = rhai::Map::new();
                                mob_info
                                    .insert("이름".into(), Dynamic::from(mob_data.name.clone()));
                                mob_info.insert("표시".into(), Dynamic::from(display_name.clone()));
                                mob_info.insert("hp".into(), Dynamic::from(mob.hp));
                                mob_info.insert("max_hp".into(), Dynamic::from(mob.max_hp));
                                mob_info.insert("level".into(), Dynamic::from(mob_data.level));
                                mob_info.insert("zone".into(), Dynamic::from(mob.zone.clone()));
                                mob_info.insert("room".into(), Dynamic::from(mob.room.clone()));
                                mob_info
                                    .insert("mob_key".into(), Dynamic::from(mob.mob_key.clone()));
                                return Dynamic::from(mob_info);
                            }
                        }
                    }
                }
            }

            Dynamic::UNIT
        },
    );

    // get_mob_by_name(ob, mob_name) - 데이터베이스에서 몹 정보 조회
    // 몹 데이터베이스(Mobs)에서 몹 정보를 가져옴
    let body_ptr_get_mob = body_ptr;
    engine.register_fn(
        "get_mob_by_name",
        move |_ob: &mut rhai::Map, mob_name: &str| -> Dynamic {
            let _body = unsafe { &*body_ptr_get_mob };
            // 기존 get_mob_data 함수와 동일하게 동작
            let full_path = format!("data/mob/{}.json", mob_name);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(obj) = value.get("몹정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                },
                Err(_) => Dynamic::UNIT,
            }
        },
    );

    // kill_mob(ob, mob_name) - 몹 처치
    let body_ptr_kill = body_ptr;
    engine.register_fn(
        "kill_mob",
        move |ob: &mut rhai::Map, mob_name: &str| -> bool {
            let body = unsafe { &*body_ptr_kill };

            // 플레이어 이름과 위치 가져오기
            let player_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();

            if player_name.is_empty() {
                return false;
            }

            // 위치 정보 파싱 (zone/room 형식)
            let location = body.get_string("위치");
            let parts: Vec<&str> = location.splitn(2, '/').collect();
            if parts.len() != 2 {
                return false;
            }
            let zone = parts[0];
            let room = parts[1];

            // WorldState에서 현재 방의 몹 검색 후 처치
            // 먼저 읽기 lock으로 몹 찾기
            let mob_key_to_kill = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(zone, room);
                let mob_name_lower = mob_name.to_lowercase();
                let mut found_key = None;

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    // 몹 데이터로 표시 이름 확인
                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let display_name_lower = display_name.to_lowercase();

                        // 정확히 일치하거나 포함
                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_key = Some(mob.mob_key.clone());
                            break;
                        }

                        // 반응 이름들도 확인
                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_key = Some(mob.mob_key.clone());
                                break;
                            }
                        }
                    }
                }
                found_key
            } else {
                None
            };

            // 찾은 몹 처치 (쓰기 lock)
            if let Some(mob_key) = mob_key_to_kill {
                if let Ok(mut world) = get_world_state().write() {
                    world.kill_mob(zone, room, &mob_key);
                    return true;
                }
            }

            false
        },
    );

    // create_mob(ob, mob_name, zone, room) - 새 몹 생성
    let body_ptr_create = body_ptr;
    engine.register_fn(
        "create_mob",
        move |_ob: &mut rhai::Map, mob_name: &str, zone: &str, room: &str| -> String {
            let _body = unsafe { &*body_ptr_create };

            // 몹 데이터 로드 - WorldState를 통해 로드
            let mob_data = if let Ok(mut world) = get_world_state().write() {
                match world.mob_cache.load_mob(zone, mob_name) {
                    Ok(data) => data,
                    Err(_) => {
                        // zone 폴더에 없으면 시도
                        match world.mob_cache.load_mob(zone, mob_name) {
                            Ok(data) => data,
                            Err(_) => return format!("몹 데이터를 찾을 수 없습니다: {}", mob_name),
                        }
                    }
                }
            } else {
                return "월드 상태 접근 실패".to_string();
            };

            // 몹 생성
            if let Ok(mut world) = get_world_state().write() {
                // Use with_difficulty constructor for proper stat initialization
                let mob_instance = MobInstance::with_difficulty(
                    format!("{}:{}", zone, mob_name),
                    zone.to_string(),
                    room.to_string(),
                    &mob_data,
                    0, // difficulty 0 for spawned mobs
                );

                world.mob_cache.add_mob_instance(mob_instance);
                String::new() // 성공 시 빈 문자열 반환
            } else {
                "월드 상태 접근 실패".to_string()
            }
        },
    );

    // mob_say(mob_name, message) - 몹이 말하기
    let body_ptr_say = body_ptr;
    engine.register_fn(
        "mob_say",
        move |_ob: &mut rhai::Map, mob_name: &str, message: &str| -> bool {
            let body = unsafe { &*body_ptr_say };

            // 플레이어 위치 가져오기
            let location = body.get_string("위치");
            let parts: Vec<&str> = location.splitn(2, '/').collect();
            if parts.len() != 2 {
                return false;
            }
            let zone = parts[0].to_string();
            let room = parts[1].to_string();

            // WorldState에서 몹 찾기 (display_name을 소유하여 반환)
            let found_display_name = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();

                let mut found_name = None;
                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name_lower = mob_data.desc1.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_name = Some(mob_data.desc1.clone());
                            break;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_name = Some(mob_data.desc1.clone());
                                break;
                            }
                        }
                    }
                }
                found_name
            } else {
                None
            };

            if let Some(display_name) = found_display_name {
                // 메시지 전송 - 브로드캐스터에 메시지 보내기
                // 현재는 로그로 출력 (실제로는 broadcaster를 통해 방에 있는 모든 플레이어에게 전송)
                println!("[MOB_SAY] {}: {}", display_name, message);
                true
            } else {
                false
            }
        },
    );

    // mob_follow(mob_name, target_name) - 몹이 대상 따라가기
    let body_ptr_follow = body_ptr;
    engine.register_fn(
        "mob_follow",
        move |_ob: &mut rhai::Map, mob_name: &str, target_name: &str| -> bool {
            let body = unsafe { &*body_ptr_follow };

            // 플레이어 위치 가져오기
            let location = body.get_string("위치");
            let parts: Vec<&str> = location.splitn(2, '/').collect();
            if parts.len() != 2 {
                return false;
            }
            let zone = parts[0].to_string();
            let room = parts[1].to_string();

            // 먼저 읽기 lock으로 몹 찾기
            let mob_key_to_follow = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();
                let mut found_key = None;
                let mut found_name = None;

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let display_name_lower = display_name.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_key = Some(mob.mob_key.clone());
                            found_name = Some(display_name.clone());
                            break;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_key = Some(mob.mob_key.clone());
                                found_name = Some(display_name.clone());
                                break;
                            }
                        }
                    }
                }
                (found_key, found_name)
            } else {
                (None, None)
            };

            // 찾은 몹의 타겟 설정
            if let (Some(mob_key), Some(display_name)) = mob_key_to_follow {
                if let Ok(mut world) = get_world_state().write() {
                    if let Some(mob_instance) =
                        world.mob_cache.get_mob_instance_mut(&zone, &room, &mob_key)
                    {
                        if !mob_instance.targets.contains(&target_name.to_string()) {
                            mob_instance.targets.push(target_name.to_string());
                        }
                    }
                    println!(
                        "[MOB_FOLLOW] {} now following {}",
                        display_name, target_name
                    );
                    return true;
                }
            }

            false
        },
    );

    // get_mob_hp(ob, mob_name) - 몹의 현재 HP 조회
    let body_ptr_get_hp = body_ptr;
    engine.register_fn(
        "get_mob_hp",
        move |_ob: &mut rhai::Map, mob_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_hp };

            // 플레이어 위치 가져오기
            let location = body.get_string("위치");
            let parts: Vec<&str> = location.splitn(2, '/').collect();
            if parts.len() != 2 {
                return 0;
            }
            let zone = parts[0];
            let room = parts[1];

            // WorldState에서 몹 찾기
            if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(zone, room);
                let mob_name_lower = mob_name.to_lowercase();

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let display_name_lower = display_name.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            return mob.hp;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                return mob.hp;
                            }
                        }
                    }
                }
            }

            0
        },
    );

    // set_mob_hp(ob, mob_name, hp) - 몹의 HP 설정
    let body_ptr_set_hp = body_ptr;
    engine.register_fn(
        "set_mob_hp",
        move |_ob: &mut rhai::Map, mob_name: &str, hp: i64| -> bool {
            let body = unsafe { &*body_ptr_set_hp };

            // 플레이어 위치 가져오기
            let location = body.get_string("위치");
            let parts: Vec<&str> = location.splitn(2, '/').collect();
            if parts.len() != 2 {
                return false;
            }
            let zone = parts[0].to_string();
            let room = parts[1].to_string();

            // 먼저 읽기 lock으로 몹 찾기
            let mob_key_to_set = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();
                let mut found_key = None;

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name_lower = mob_data.desc1.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_key = Some(mob.mob_key.clone());
                            break;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_key = Some(mob.mob_key.clone());
                                break;
                            }
                        }
                    }
                }
                found_key
            } else {
                None
            };

            // 찾은 몹의 HP 설정
            if let Some(mob_key) = mob_key_to_set {
                if let Ok(mut world) = get_world_state().write() {
                    if let Some(mob_instance) =
                        world.mob_cache.get_mob_instance_mut(&zone, &room, &mob_key)
                    {
                        mob_instance.hp = hp.max(0).min(mob_instance.max_hp);
                        if mob_instance.hp <= 0 {
                            world.kill_mob(&zone, &room, &mob_key);
                        }
                        return true;
                    }
                }
            }

            false
        },
    );

    // ============================================================
    // Room/Zone 관련 efun
    // ============================================================

    // get_room(ob, zone:room_id) - 특정 zone:room의 방 데이터 조회
    let body_ptr_get_room = body_ptr;
    engine.register_fn(
        "get_room",
        move |_ob: &mut rhai::Map, zone: &str, room_id: &str| -> Dynamic {
            let _body = unsafe { &*body_ptr_get_room };
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return Dynamic::UNIT,
            };
            let _room_key = format!("{}:{}", zone, room_id);
            if let Some(arc) = w.room_cache.get_room_cached(zone, room_id) {
                if let Ok(room_ref) = arc.read() {
                    let mut m = rhai::Map::new();
                    m.insert("zone".into(), Dynamic::from(room_ref.zone.clone()));
                    m.insert("room".into(), Dynamic::from(room_ref.name.clone()));
                    m.insert("name".into(), Dynamic::from(room_ref.display_name.clone()));
                    m.insert(
                        "desc".into(),
                        Dynamic::from(room_ref.description.join("\n")),
                    );
                    // 출구 배열: [{direction, display_name, destination_zone, destination_room}, ...]
                    let mut exits_arr = rhai::Array::new();
                    for (display_name, exit) in &room_ref.exits {
                        let mut exit_map = rhai::Map::new();
                        exit_map.insert("display_name".into(), Dynamic::from(display_name.clone()));
                        if let Some(dir) = &exit.direction {
                            exit_map.insert("direction".into(), Dynamic::from(dir.korean_name()));
                        } else {
                            exit_map.insert("direction".into(), Dynamic::from(""));
                        }
                        if let Some((dest_zone, dest_room)) = &exit.destination {
                            exit_map.insert(
                                "destination_zone".into(),
                                Dynamic::from(dest_zone.clone()),
                            );
                            exit_map.insert(
                                "destination_room".into(),
                                Dynamic::from(dest_room.clone()),
                            );
                        }
                        exit_map.insert("hidden".into(), Dynamic::from(exit.hidden));
                        exits_arr.push(Dynamic::from(exit_map));
                    }
                    m.insert("exits".into(), Dynamic::from(exits_arr));
                    // 맵속성 배열
                    let mut props_arr = rhai::Array::new();
                    for prop in &room_ref.properties {
                        props_arr.push(Dynamic::from(prop.clone()));
                    }
                    m.insert("properties".into(), Dynamic::from(props_arr));
                    return Dynamic::from(m);
                }
            }
            Dynamic::UNIT
        },
    );

    // get_current_room(ob) - 현재 플레이어의 방 데이터 조회
    let body_ptr_cur_room = body_ptr;
    engine.register_fn("get_current_room", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &*body_ptr_cur_room };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return Dynamic::UNIT,
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return Dynamic::UNIT,
        };
        if let Some(arc) = w.room_cache.get_room_cached(&pos.zone, &pos.room) {
            if let Ok(room_ref) = arc.read() {
                let mut m = rhai::Map::new();
                m.insert("zone".into(), Dynamic::from(room_ref.zone.clone()));
                m.insert("room".into(), Dynamic::from(room_ref.name.clone()));
                m.insert("name".into(), Dynamic::from(room_ref.display_name.clone()));
                m.insert(
                    "desc".into(),
                    Dynamic::from(room_ref.description.join("\n")),
                );
                // 출구 배열
                let mut exits_arr = rhai::Array::new();
                for (display_name, exit) in &room_ref.exits {
                    let mut exit_map = rhai::Map::new();
                    exit_map.insert("display_name".into(), Dynamic::from(display_name.clone()));
                    if let Some(dir) = &exit.direction {
                        exit_map.insert("direction".into(), Dynamic::from(dir.korean_name()));
                    } else {
                        exit_map.insert("direction".into(), Dynamic::from(""));
                    }
                    if let Some((dest_zone, dest_room)) = &exit.destination {
                        exit_map
                            .insert("destination_zone".into(), Dynamic::from(dest_zone.clone()));
                        exit_map
                            .insert("destination_room".into(), Dynamic::from(dest_room.clone()));
                    }
                    exit_map.insert("hidden".into(), Dynamic::from(exit.hidden));
                    exits_arr.push(Dynamic::from(exit_map));
                }
                m.insert("exits".into(), Dynamic::from(exits_arr));
                return Dynamic::from(m);
            }
        }
        Dynamic::UNIT
    });

    // find_obj_in_room(ob, obj_name) - 현재 방에서 아이템으로 이름 찾기
    let body_ptr_find_obj = body_ptr;
    engine.register_fn(
        "find_obj_in_room",
        move |_ob: &mut rhai::Map, obj_name: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_find_obj };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return Dynamic::UNIT,
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return Dynamic::UNIT,
            };
            // 바닥 아이템 검색
            let room_objs = w.get_room_objs(&pos.zone, &pos.room);
            for arc in room_objs {
                if let Ok(o) = arc.lock() {
                    let item_name = o.getName();
                    // 정확히 일치하거나 접두사 일치
                    if item_name == obj_name || item_name.starts_with(obj_name) {
                        let mut m = rhai::Map::new();
                        m.insert("name".into(), Dynamic::from(item_name));
                        m.insert("name_a".into(), Dynamic::from(o.getNameA()));
                        m.insert("desc1".into(), Dynamic::from(o.getString("설명1")));
                        m.insert("count".into(), Dynamic::from(1i64));
                        return Dynamic::from(m);
                    }
                }
            }
            // 쌓을 수 있는 아이템 검색
            let room_stack = w.get_room_objs_stack(&pos.zone, &pos.room);
            for (key, count) in room_stack {
                if count > 0 {
                    if let Some((item_name, _, _, _)) = get_item_info(&key) {
                        let obj_name_str = obj_name.to_string();
                        if item_name == obj_name_str || item_name.starts_with(&obj_name_str) {
                            let mut m = rhai::Map::new();
                            m.insert("name".into(), Dynamic::from(item_name.clone()));
                            m.insert("desc1".into(), Dynamic::from(get_item_desc1(&key)));
                            m.insert("count".into(), Dynamic::from(count));
                            m.insert("key".into(), Dynamic::from(key));
                            return Dynamic::from(m);
                        }
                    }
                }
            }
            Dynamic::UNIT
        },
    );

    // get_room_exits(ob) - 현재 방의 출구 방향 배열
    let body_ptr_exits = body_ptr;
    engine.register_fn(
        "get_room_exits",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_exits };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let mut exits = rhai::Array::new();
            if let Some(arc) = w.room_cache.get_room_cached(&pos.zone, &pos.room) {
                if let Ok(room_ref) = arc.read() {
                    // 방향이 있는 출구만 (방향 이동용)
                    for exit in room_ref.exits.values() {
                        if let Some(dir) = &exit.direction {
                            exits.push(Dynamic::from(dir.korean_name()));
                        }
                    }
                }
            }
            exits
        },
    );

    // get_room_players(ob) - 현재 방의 플레이어 목록 (실제 구현)
    let body_ptr_players = body_ptr;
    engine.register_fn(
        "get_room_players",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_players };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let players = w.get_players_in_room(&pos.zone, &pos.room);
            let mut arr = rhai::Array::new();
            for player_name in players {
                arr.push(Dynamic::from(player_name));
            }
            arr
        },
    );

    // get_room_mobs(ob) - 현재 방의 몹 목록 (실제 구현)
    let body_ptr_room_mobs = body_ptr;
    engine.register_fn("get_room_mobs", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_room_mobs };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Array::new(),
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return rhai::Array::new(),
        };
        let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
        let mut arr = rhai::Array::new();
        for mob in mobs {
            if let Some(mob_data) = w.mob_cache.get_mob(&mob.mob_key) {
                let mut m = rhai::Map::new();
                m.insert("name".into(), Dynamic::from(mob_data.name.clone()));
                m.insert("desc1".into(), Dynamic::from(mob_data.desc1.clone()));
                m.insert("alive".into(), Dynamic::from(mob.alive));
                m.insert("hp".into(), Dynamic::from(mob.hp));
                m.insert("max_hp".into(), Dynamic::from(mob.max_hp));
                m.insert("mob_key".into(), Dynamic::from(mob.mob_key.clone()));
                arr.push(Dynamic::from(m));
            }
        }
        arr
    });

    // get_room_mobs_admin(ob) - 관리자용 몹 상세 정보 (infoMob 대응)
    // 레벨, 체력, 내공, 힘, 민첩, 맷집, 타겟 등 상세 정보 반환
    let body_ptr_room_mobs_admin = body_ptr;
    engine.register_fn(
        "get_room_mobs_admin",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_room_mobs_admin };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(name.as_str()) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut arr = rhai::Array::new();
            for mob in mobs {
                if let Some(mob_data) = w.mob_cache.get_mob(&mob.mob_key) {
                    let mut m = rhai::Map::new();
                    m.insert("name".into(), Dynamic::from(mob_data.name.clone()));
                    m.insert("level".into(), Dynamic::from(mob_data.level));
                    m.insert("hp".into(), Dynamic::from(mob.hp));
                    m.insert("max_hp".into(), Dynamic::from(mob.max_hp));
                    m.insert("inner_power".into(), Dynamic::from(mob_data.inner_power));
                    m.insert("strength".into(), Dynamic::from(mob_data.strength));
                    m.insert("agility".into(), Dynamic::from(mob_data.agility));
                    m.insert("alive".into(), Dynamic::from(mob.alive));
                    // 타겟 목록
                    let mut targets_arr = rhai::Array::new();
                    for target_name in &mob.targets {
                        targets_arr.push(Dynamic::from(target_name.clone()));
                    }
                    m.insert("targets".into(), Dynamic::from(targets_arr));
                    // 상태 (alive/dead)
                    let state = if mob.alive { "활동" } else { "사망" };
                    m.insert("state".into(), Dynamic::from(state));
                    arr.push(Dynamic::from(m));
                }
            }
            arr
        },
    );

    // get_room_players_admin(ob) - 관리자용 플레이어 상세 정보 (infoPlayer 대응)
    // 레벨, 체력, 내공, 힘, 민첩, 맷집, 타겟 등 상세 정보 반환
    let body_ptr_room_players_admin = body_ptr;
    engine.register_fn(
        "get_room_players_admin",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_room_players_admin };
            let viewer_name = body.get_name();

            // 같은 방의 다른 플레이어 목록
            let mut arr = rhai::Array::new();

            // TODO: broadcaster를 통한 플레이어 데이터 접근 구현
            // 현재는 간단하게 이름만 반환
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return arr,
            };
            let pos = match w.get_player_position(viewer_name.as_str()) {
                Some(p) => p,
                None => return arr,
            };
            let players = w.get_players_in_room(&pos.zone, &pos.room);
            for player_name in players {
                if player_name != viewer_name {
                    let mut m = rhai::Map::new();
                    m.insert("name".into(), Dynamic::from(player_name.clone()));
                    m.insert("level".into(), Dynamic::from(1i64)); // TODO: 실제 레벨
                    m.insert("hp".into(), Dynamic::from(100i64)); // TODO: 실제 HP
                    m.insert("max_hp".into(), Dynamic::from(100i64)); // TODO: 실제 최대 HP
                    arr.push(Dynamic::from(m));
                }
            }
            arr
        },
    );

    // look_room(ob) - 현재 방 설명 (look 명령용)
    let body_ptr_look = body_ptr;
    engine.register_fn("look_room", move |_ob: &mut rhai::Map| -> String {
        let body = unsafe { &*body_ptr_look };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return "방 정보를 가져올 수 없습니다.".to_string(),
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return "위치 정보가 없습니다.".to_string(),
        };
        if let Some(arc) = w.room_cache.get_room_cached(&pos.zone, &pos.room) {
            if let Ok(room_ref) = arc.read() {
                let room_name_formatted = format_room_header(&room_ref.display_name);
                let exits_str = format_exits_long(&room_ref);
                let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
                let mob_str = if mobs.is_empty() {
                    String::new()
                } else {
                    let mut mob_msgs = Vec::new();
                    for mob in mobs {
                        if let Some(mob_data) = w.mob_cache.get_mob(&mob.mob_key) {
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
                let room_objs = w.get_room_objs(&pos.zone, &pos.room);
                let room_stack = w.get_room_objs_stack(&pos.zone, &pos.room);
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
                return out;
            }
        }
        "방 정보를 가져올 수 없습니다.".to_string()
    });

    // move_player(ob, direction) - 플레이어 이동
    let body_ptr_move = body_ptr;
    engine.register_fn(
        "move_player",
        move |_ob: &mut rhai::Map, direction: &str| -> String {
            let body = unsafe { &*body_ptr_move };
            let name = body.get_name();
            if name.is_empty() {
                return "플레이어 정보가 없습니다.".to_string();
            }
            // 방향 문자열을 Direction으로 변환
            let dir = match crate::world::Direction::from_korean(direction) {
                Some(d) => d,
                None => return format!("{}쪽은 없습니다.", direction),
            };
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "이동할 수 없습니다.".to_string(),
            };
            match w.move_player(&name, dir) {
                Ok(_) => String::new(), // 성공 시 빈 문자열
                Err(e) => e,
            }
        },
    );

    // ============================================================
    // 플레이어 간 상호작용 efun (관리자용)
    // ============================================================

    // get_player_by_name(name) - 이름으로 플레이어 데이터 조회
    // 다른 플레이어의 데이터를 조회할 때 사용 (관리자 기능)
    // 현재는 제한적 구현 - 자기 자신만 가능
    let body_ptr_get = body_ptr;
    engine.register_fn("get_player_by_name", move |name: &str| -> Dynamic {
        let body = unsafe { &*body_ptr_get };
        if body.get_name() == name {
            // 자기 자신의 데이터 반환 (확장)
            let mut m = rhai::Map::new();
            m.insert("이름".into(), Dynamic::from(body.get_name()));
            m.insert("레벨".into(), Dynamic::from(body.get_int("레벨")));
            m.insert("hp".into(), Dynamic::from(body.get_hp()));
            m.insert("max_hp".into(), Dynamic::from(body.get_max_hp()));
            m.insert("은전".into(), Dynamic::from(body.get_int("은전")));
            m.insert("금전".into(), Dynamic::from(body.get_int("금전")));
            m.insert(
                "무림별호".into(),
                Dynamic::from(body.get_string("무림별호")),
            );
            m.insert("소속".into(), Dynamic::from(body.get_string("소속")));

            // 스킬 목록
            let skills: rhai::Array = body
                .skill_list
                .iter()
                .map(|s: &String| Dynamic::from(s.clone()))
                .collect();
            m.insert("스킬".into(), Dynamic::from(skills));

            // 인벤토리 (비스택 아이템)
            let mut inv_items: rhai::Array = rhai::Array::new();
            for arc in &body.object.objs {
                if let Ok(o) = arc.lock() {
                    let mut item = rhai::Map::new();
                    item.insert("이름".into(), Dynamic::from(o.getName()));
                    item.insert("인덱스".into(), Dynamic::from(o.getString("인덱스")));
                    inv_items.push(Dynamic::from(item));
                }
            }
            m.insert("인벤토리".into(), Dynamic::from(inv_items));

            // 스택 아이템
            let mut stack_items = rhai::Map::new();
            for (key, count) in &body.object.inv_stack {
                stack_items.insert(key.clone().into(), Dynamic::from(*count));
            }
            m.insert("스택아이템".into(), Dynamic::from(stack_items));

            Dynamic::from(m)
        } else {
            // 다른 플레이어는 현재 조회 불가
            Dynamic::UNIT
        }
    });

    // give_silver_to_player(from_ob, to_name, amount) - 은전 전송
    let body_ptr_give = body_ptr;
    engine.register_fn(
        "give_silver_to_player",
        move |_from_ob: &mut rhai::Map, to_name: &str, _amount: i64| -> String {
            let body = unsafe { &*body_ptr_give };
            if body.get_name() == to_name {
                return "self".to_string();
            }
            // TODO: 다른 플레이어 찾아서 전송
            // 현재는 "not_found" 반환
            "not_found".to_string()
        },
    );

    // teach_skill_to_player(teacher_ob, student_name, skill_name) - 무공 전수
    let body_ptr_teach = body_ptr;
    engine.register_fn(
        "teach_skill_to_player",
        move |_teacher_ob: &mut rhai::Map, student_name: &str, skill_name: &str| -> String {
            let _body = unsafe { &*body_ptr_teach };
            // TODO: 학생 찾아서 스킬 추가
            // 현재는 "not_found" 또는 "not_implemented" 반환
            println!("[SCRIPT] teach_skill: {} -> {}", student_name, skill_name);
            if _body.get_name() == student_name {
                return "self".to_string();
            }
            "not_implemented".to_string()
        },
    );

    // check_player_skill(player_name, skill_name) - 플레이어 스킬 보유 확인
    let body_ptr_check = body_ptr;
    engine.register_fn(
        "check_player_skill",
        move |player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &*body_ptr_check };
            if body.get_name() == player_name {
                // 자기 자신의 스킬만 확인 가능
                return body.skill_list.contains(&skill_name.to_string());
            }
            false
        },
    );

    // ============================================================
    // 플레이어 상호작용 관련 추가 efun
    // ============================================================

    // find_player_online(name) - 플레이어 접속 중인지 확인
    // 접속 중이면 true 반환
    engine.register_fn("find_player_online", move |name: &str| -> bool {
        if let Ok(w) = get_world_state().try_read() {
            w.player_positions.contains_key(name)
        } else {
            false
        }
    });

    // send_to_player(player_name, message) - 특정 플레이어에게 메시지 전송
    // 성공 시 true 반환
    let user_sends_clone = user_sends.clone();
    engine.register_fn(
        "send_to_player",
        move |player_name: &str, message: &str| -> bool {
            if player_name.is_empty() || message.is_empty() {
                return false;
            }
            // 플레이어가 접속 중인지 확인
            let online = if let Ok(w) = get_world_state().try_read() {
                w.player_positions.contains_key(player_name)
            } else {
                false
            };
            if !online {
                return false;
            }
            // user_sends에 메시지 추가
            if let Ok(mut sends) = user_sends_clone.lock() {
                sends.push((player_name.to_string(), message.to_string()));
                true
            } else {
                false
            }
        },
    );

    // give_money_to_player(from_ob, to_name, amount) - 돈 전송
    // 성공 시 "", 실패 시 에러 코드 반환
    let spec_money = spec.clone();
    let body_ptr_money = body_ptr;
    engine.register_fn(
        "give_money_to_player",
        move |_from_ob: &mut rhai::Map, to_name: &str, amount: i64| -> String {
            let body = unsafe { &*body_ptr_money };

            // 파라미터 검증
            if amount < 1 {
                return "usage".to_string(); // 잘못된 금액
            }

            let my_name = body.get_name();

            // 자기 자신에게는 줄 수 없음
            if my_name == to_name {
                return "self".to_string();
            }

            // 상대방이 접속 중인지 확인
            let target_online = if let Ok(w) = get_world_state().try_read() {
                w.player_positions.contains_key(to_name)
            } else {
                false
            };
            if !target_online {
                return "not_online".to_string();
            }

            // 보내는 사람의 돈 확인 (은전)
            let have = body.get_int("은전");
            if have < amount {
                return "no_money".to_string();
            }

            // CommandResult에 GiveToPlayer 설정 (실제 전송은 핸들러에서)
            if let Ok(mut s) = spec_money.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: to_name.to_string(),
                    giver_name: my_name,
                    give_silver: Some(amount),
                    give_gold: None,
                    give_item: None,
                    give_item_stack: None,
                });
            }

            String::new() // 성공
        },
    );

    // give_item_to_player(from_ob, to_name, item_name) - 아이템 전송
    // 성공 시 "", 실패 시 에러 코드 반환
    let spec_item = spec.clone();
    let body_ptr_item = body_ptr;
    engine.register_fn(
        "give_item_to_player",
        move |_from_ob: &mut rhai::Map, to_name: &str, item_name: &str| -> String {
            let body = unsafe { &*body_ptr_item };

            // 파라미터 검증
            if item_name.is_empty() {
                return "usage".to_string();
            }

            let my_name = body.get_name();

            // 자기 자신에게는 줄 수 없음
            if my_name == to_name {
                return "self".to_string();
            }

            // 상대방이 접속 중인지 확인
            let target_online = if let Ok(w) = get_world_state().try_read() {
                w.player_positions.contains_key(to_name)
            } else {
                false
            };
            if !target_online {
                return "not_online".to_string();
            }

            // 아이템이 있는지 확인 (스택 아이템 우선)
            let mut found_item = false;
            let mut give_stack: Option<(String, i64)> = None;
            let mut give_non_stack: Option<(String, usize, usize)> = None;

            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    if have > 0 {
                        found_item = true;
                        give_stack = Some((key.clone(), 1)); // 기본 1개
                    }
                }
            }

            // 비스택 아이템 확인
            if !found_item {
                if let Some(_arc) = body.object.findObjInven(item_name, 1) {
                    found_item = true;
                    give_non_stack = Some((item_name.to_string(), 1, 1));
                }
            }

            if !found_item {
                return "no_item".to_string();
            }

            // CommandResult에 GiveToPlayer 설정
            if let Ok(mut s) = spec_item.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: to_name.to_string(),
                    giver_name: my_name,
                    give_silver: None,
                    give_gold: None,
                    give_item: give_non_stack,
                    give_item_stack: give_stack,
                });
            }

            String::new() // 성공
        },
    );

    // add_skill_to_player(ob, player_name, skill_name) - 스킬 추가
    // 성공 시 true 반환
    let body_ptr_add_skill = body_ptr;
    engine.register_fn(
        "add_skill_to_player",
        move |_ob: &mut rhai::Map, player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_add_skill };
            if skill_name.is_empty() {
                return false;
            }
            // 현재는 자기 자신에게만 추가 가능
            if body.get_name() != player_name {
                return false;
            }
            // 이미 있는지 확인
            if body.skill_list.contains(&skill_name.to_string()) {
                return true; // 이미 있음
            }

            // 스킬 추가
            body.skill_list.push(skill_name.to_string());
            body.skill_map.insert(
                skill_name.to_string(),
                crate::player::SkillTraining::new(1, 0),
            );
            true
        },
    );

    // ============================================================
    // SKILL/ABILITY 관련 efun
    // ============================================================

    // Helper function to parse MP cost from skill 속성
    fn parse_mp_cost(skill_data: &serde_json::Value) -> i64 {
        if let Some(attrs) = skill_data.get("속성") {
            let attr_str: String = if attrs.is_string() {
                attrs.as_str().unwrap_or("").to_string()
            } else if attrs.is_array() {
                attrs
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            // "내공소모 240" -> 240
            for part in attr_str.split_whitespace() {
                if part == "내공소모" {
                    if let Ok(val) = attr_str
                        .split("내공소모")
                        .nth(1)
                        .unwrap_or("")
                        .split_whitespace()
                        .next()
                        .unwrap_or("0")
                        .parse::<i64>()
                    {
                        return val;
                    }
                }
            }
        }
        0
    }

    // Helper function to parse skill bonuses from 속성
    // Returns (hp_bonus, mp_bonus, str_bonus, dex_bonus, arm_bonus, skill_bonus)
    fn parse_skill_bonuses(skill_data: &serde_json::Value) -> (i64, i64, i64, i64, i64, i64) {
        let mut hp_bonus = 0i64;
        let mut mp_bonus = 0i64;
        let mut str_bonus = 0i64;
        let mut dex_bonus = 0i64;
        let mut arm_bonus = 0i64;
        let mut skill_bonus = 0i64;

        if let Some(attrs) = skill_data.get("속성") {
            let attr_str: String = if attrs.is_string() {
                attrs.as_str().unwrap_or("").to_string()
            } else if attrs.is_array() {
                attrs
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // Parse each bonus type
            let parts: Vec<&str> = attr_str.split_whitespace().collect();
            let mut i = 0;
            while i < parts.len() {
                match parts[i] {
                    "체력증가" | "체력회복" => {
                        if i + 1 < parts.len() {
                            hp_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "내공증가" | "내공회복" => {
                        if i + 1 < parts.len() {
                            mp_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "힘증가" => {
                        if i + 1 < parts.len() {
                            str_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "민첩증가" => {
                        if i + 1 < parts.len() {
                            dex_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "맷집증가" => {
                        if i + 1 < parts.len() {
                            arm_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "위력" | "보너스" => {
                        if i + 1 < parts.len() {
                            skill_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }

        (hp_bonus, mp_bonus, str_bonus, dex_bonus, arm_bonus, skill_bonus)
    }

    // Helper function to get skill description from 속성
    fn get_skill_description(skill_data: &serde_json::Value) -> String {
        let mut desc_parts = Vec::new();

        if let Some(kind) = skill_data.get("종류") {
            if let Some(s) = kind.as_str() {
                desc_parts.push(format!("종류: {}", s));
            }
        }

        if let Some(attrs) = skill_data.get("속성") {
            let attr_str = if attrs.is_string() {
                attrs.as_str().unwrap_or("").to_string()
            } else if attrs.is_array() {
                attrs
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default()
            } else {
                "".to_string()
            };
            if !attr_str.is_empty() {
                desc_parts.push(format!("속성: {}", attr_str));
            }
        }

        if let Some(prob) = skill_data.get("확률") {
            if let Some(n) = prob.as_i64() {
                desc_parts.push(format!("확률: {}", n));
            }
        }

        desc_parts.join(" | ")
    }

    // use_skill(ob, skill_name, target) - 무공 스킬 사용
    // Returns "" on success, error string on failure
    let body_ptr_use_skill = body_ptr;
    engine.register_fn(
        "use_skill",
        move |_ob: &mut rhai::Map, skill_name: &str, _target: &str| -> String {
            let body = unsafe { &mut *body_ptr_use_skill };

            // Check if player has the skill
            if !body.skill_list.contains(&skill_name.to_string()) {
                return format!("배우지 않은 무공입니다: {}", skill_name);
            }

            // Check cooldown
            let cooldown_remaining = body.get_skill_cooldown_remaining(skill_name);
            if cooldown_remaining > 0 {
                return format!("쿨다운 중입니다. {}초 남음.", cooldown_remaining);
            }

            // Load skill data
            let skill_path = "data/config/skill.json";
            let skill_data = match std::fs::read_to_string(skill_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => value,
                    Err(_) => return "스킬 데이터를 읽을 수 없습니다.".to_string(),
                },
                Err(_) => return "스킬 데이터를 찾을 수 없습니다.".to_string(),
            };

            let skill_info = match skill_data.get(skill_name) {
                Some(s) => s,
                None => return format!("스킬을 찾을 수 없습니다: {}", skill_name),
            };

            // Check MP cost
            let mp_cost = parse_mp_cost(skill_info);
            if mp_cost > 0 {
                let current_mp = body.get_mp();
                if current_mp < mp_cost {
                    return format!("내공이 부족합니다. 필요: {}, 현재: {}", mp_cost, current_mp);
                }
                // Deduct MP
                let new_mp = current_mp - mp_cost;
                body.set("내공", new_mp);
            }

            // Set skill cast time (mark as used)
            body.set_skill_cast_time(skill_name);

            // Get skill level
            let skill_level = body
                .skill_map
                .get(skill_name)
                .map(|t| t.level as i32)
                .unwrap_or(1);

            // Parse skill bonuses from 속성
            let (hp_bonus, mp_bonus, str_bonus, dex_bonus, arm_bonus, skill_bonus) =
                parse_skill_bonuses(skill_info);

            // Apply skill effects to player (healing, stat boosts, etc.)
            let effects = crate::combat::apply_skill_effects(
                body,
                skill_name,
                hp_bonus,
                mp_bonus,
                str_bonus,
                dex_bonus,
                arm_bonus,
            );

            // Log effects
            for effect in &effects {
                if !effect.message.is_empty() {
                    println!("[SKILL] {}", effect.message);
                }
            }

            // Calculate skill damage if there's a target
            if !_target.is_empty() {
                let damage_result = crate::combat::calculate_skill_damage(
                    body,
                    skill_name,
                    skill_level,
                    skill_bonus,
                    _target,
                );

                // Log damage
                if damage_result.hit {
                    println!(
                        "[SCRIPT] use_skill: {} used by {} on {} for {} damage",
                        skill_name,
                        body.get_name(),
                        _target,
                        damage_result.final_damage
                    );
                } else {
                    println!(
                        "[SCRIPT] use_skill: {} used by {} on {} (missed)",
                        skill_name,
                        body.get_name(),
                        _target
                    );
                }
            } else {
                println!(
                    "[SCRIPT] use_skill: {} used by {} (self-buff)",
                    skill_name,
                    body.get_name()
                );
            }

            // Success - return empty string
            "".to_string()
        },
    );

    // learn_skill(ob, skill_name) - 새 스킬 학습
    // Returns "" on success, error string on failure
    let body_ptr_learn = body_ptr;
    engine.register_fn(
        "learn_skill",
        move |_ob: &mut rhai::Map, skill_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_learn };

            // Check if already learned
            if body.skill_list.contains(&skill_name.to_string()) {
                return format!("이미 배운 무공입니다: {}", skill_name);
            }

            // Validate skill exists
            let skill_path = "data/config/skill.json";
            let skill_data = match std::fs::read_to_string(skill_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => value,
                    Err(_) => return "스킬 데이터를 읽을 수 없습니다.".to_string(),
                },
                Err(_) => return "스킬 데이터를 찾을 수 없습니다.".to_string(),
            };

            if skill_data.get(skill_name).is_none() {
                return format!("존재하지 않는 무공입니다: {}", skill_name);
            }

            // Add to skill_list
            body.skill_list.push(skill_name.to_string());

            // Initialize skill_map with level 1, exp 0
            body.skill_map.insert(
                skill_name.to_string(),
                crate::player::SkillTraining::new(1, 0),
            );

            println!(
                "[SCRIPT] learn_skill: {} learned by {}",
                skill_name,
                body.get_name()
            );

            "".to_string()
        },
    );

    // forget_skill(ob, skill_name) - 스킬 잊기
    // Returns true on success
    let body_ptr_forget = body_ptr;
    engine.register_fn(
        "forget_skill",
        move |_ob: &mut rhai::Map, skill_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_forget };

            // Check if has the skill
            if !body.skill_list.contains(&skill_name.to_string()) {
                return false;
            }

            // Remove from skill_list
            body.skill_list.retain(|s| s != skill_name);

            // Remove from skill_map
            body.skill_map.remove(skill_name);

            println!(
                "[SCRIPT] forget_skill: {} forgotten by {}",
                skill_name,
                body.get_name()
            );

            true
        },
    );

    // get_skill_list(ob) - 배운 무공 목록 가져오기
    // Returns Array of skill names
    let body_ptr_get_skills = body_ptr;
    engine.register_fn(
        "get_skill_list",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_get_skills };

            let mut result = rhai::Array::new();
            for skill_name in &body.skill_list {
                result.push(Dynamic::from(skill_name.clone()));
            }
            result
        },
    );

    // get_skill_level(ob, skill_name) - 무공 수련 레벨 가져오기
    // Returns level as i64, 0 if not trained
    let body_ptr_get_level = body_ptr;
    engine.register_fn(
        "get_skill_level",
        move |_ob: &mut rhai::Map, skill_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_level };

            if let Some(training) = body.skill_map.get(skill_name) {
                training.level as i64
            } else {
                0
            }
        },
    );

    // train_skill(ob, skill_name, exp) - 무공 수련 (경험치 추가)
    // Returns new level after training
    let body_ptr_train = body_ptr;
    engine.register_fn(
        "train_skill",
        move |_ob: &mut rhai::Map, skill_name: &str, exp_add: i64| -> i64 {
            let body = unsafe { &mut *body_ptr_train };

            // Get current training or initialize new
            let current = body
                .skill_map
                .get(skill_name)
                .copied()
                .unwrap_or_else(|| crate::player::SkillTraining::new(1, 0));

            let mut new_exp = current.exp as i64 + exp_add;
            let mut new_level = current.level;

            // Simple level up logic: every 100 exp = 1 level, max 12
            while new_exp >= 100 && new_level < 12 {
                new_exp -= 100;
                new_level += 1;
            }

            // Update skill_map
            body.skill_map.insert(
                skill_name.to_string(),
                crate::player::SkillTraining::new(new_level, new_exp as u32),
            );

            println!(
                "[SCRIPT] train_skill: {} trained by {}, exp+{}, now level {}",
                skill_name,
                body.get_name(),
                exp_add,
                new_level
            );

            new_level as i64
        },
    );

    // get_skill_desc(skill_name) - 무공 설명 가져오기
    // Returns description string from MUGONG data
    engine.register_fn("get_skill_desc", move |skill_name: &str| -> String {
        let skill_path = "data/config/skill.json";
        match std::fs::read_to_string(skill_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(skill_info) = value.get(skill_name) {
                        get_skill_description(skill_info)
                    } else {
                        "".to_string()
                    }
                }
                Err(_) => "".to_string(),
            },
            Err(_) => "".to_string(),
        }
    });

    // cast_spell(ob, spell_name, target) - 주문 시전
    // Similar to use_skill but for spells (could use spell.json in future)
    let body_ptr_cast = body_ptr;
    engine.register_fn(
        "cast_spell",
        move |_ob: &mut rhai::Map, spell_name: &str, _target: &str| -> String {
            let body = unsafe { &mut *body_ptr_cast };

            // Check if player has the spell
            if !body.skill_list.contains(&spell_name.to_string()) {
                return format!("배우지 않은 주문입니다: {}", spell_name);
            }

            // For now, spells use the same skill.json data
            let skill_path = "data/config/skill.json";
            let skill_data = match std::fs::read_to_string(skill_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => value,
                    Err(_) => return "주문 데이터를 읽을 수 없습니다.".to_string(),
                },
                Err(_) => return "주문 데이터를 찾을 수 없습니다.".to_string(),
            };

            let spell_info = match skill_data.get(spell_name) {
                Some(s) => s,
                None => return format!("주문을 찾을 수 없습니다: {}", spell_name),
            };

            // Check MP cost
            let mp_cost = parse_mp_cost(spell_info);
            if mp_cost > 0 {
                let current_mp = body.get_mp();
                if current_mp < mp_cost {
                    return format!("내공이 부족합니다. 필요: {}, 현재: {}", mp_cost, current_mp);
                }
                body.set("내공", current_mp - mp_cost);
            }

            // TODO: Implement spell-specific effects
            println!(
                "[SCRIPT] cast_spell: {} cast by {}",
                spell_name,
                body.get_name()
            );

            "".to_string()
        },
    );

    // has_skill(ob, skill_name) - 스킬 보유 여부 확인
    // Returns true if player has the skill
    let body_ptr_has_skill2 = body_ptr;
    engine.register_fn(
        "has_skill",
        move |_ob: &mut rhai::Map, skill_name: &str| -> bool {
            let body = unsafe { &*body_ptr_has_skill2 };
            body.skill_list.contains(&skill_name.to_string())
        },
    );

    // remove_skill_from_player(ob, player_name, skill_name) - 스킬 제거
    // 성공 시 true 반환
    let body_ptr_remove_skill = body_ptr;
    engine.register_fn(
        "remove_skill_from_player",
        move |_ob: &mut rhai::Map, player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_remove_skill };
            if skill_name.is_empty() {
                return false;
            }
            // 현재는 자기 자신의 스킬만 제거 가능
            if body.get_name() != player_name {
                return false;
            }
            // 스킬 제거
            let original_len = body.skill_list.len();
            body.skill_list.retain(|s| s != skill_name);
            let removed = body.skill_list.len() < original_len;
            if removed {
                // 저장
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            removed
        },
    );

    // player_has_skill(ob, player_name, skill_name) - 플레이어 스킬 보유 확인
    // 스킬이 있으면 true 반환
    let body_ptr_has_skill = body_ptr;
    engine.register_fn(
        "player_has_skill",
        move |_ob: &mut rhai::Map, player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &*body_ptr_has_skill };
            if skill_name.is_empty() {
                return false;
            }
            // 현재는 자기 자신의 스킬만 확인 가능
            if body.get_name() == player_name {
                body.skill_list.contains(&skill_name.to_string())
            } else {
                false
            }
        },
    );

    // ============================================================
    // 파티/팔로우 시스템 efun
    // ============================================================

    // add_follower(ob, leader_name) - 팔로우 시작. 성공 시 "", 실패 시 에러 문자열
    let body_ptr_af = body_ptr;
    engine.register_fn(
        "add_follower",
        move |_ob: &mut rhai::Map, leader_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_af };
            let my_name = body.get_name();

            // 자기 자신은 팔로우 불가
            if my_name == leader_name {
                return "자기 자신을 팔로우할 수 없습니다.".to_string();
            }

            // 이미 팔로우 중인지 확인
            let current_leader = body.get_string("팔로우_리더");
            if !current_leader.is_empty() {
                return format!("이미 {}을(를) 팔로우 중입니다.", current_leader);
            }

            // 리더 존재 확인 (접속 중인 플레이어)
            let leader_exists = if let Ok(w) = get_world_state().try_read() {
                w.player_positions.contains_key(leader_name)
            } else {
                false
            };

            if !leader_exists {
                return "대상이 접속 중이 아닙니다.".to_string();
            }

            // 팔로우 관계 저장
            body.set("팔로우_리더", leader_name.to_string());

            "".to_string()
        },
    );

    // remove_follower(ob) - 팔로우 중지. 성공 시 true
    let body_ptr_rf = body_ptr;
    engine.register_fn("remove_follower", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr_rf };

        let current_leader = body.get_string("팔로우_리더");
        if current_leader.is_empty() {
            return false;
        }

        // 팔로우 관계 제거
        body.set("팔로우_리더", "");
        true
    });

    // get_followers(ob) - 내 팔로워 목록 반환 (Array of names)
    let body_ptr_gf = body_ptr;
    engine.register_fn("get_followers", move |_ob: &mut rhai::Map| -> rhai::Array {
        let _body = unsafe { &*body_ptr_gf };

        // TODO: 전역 팔로워 맵 구현 필요
        // 현재는 다른 플레이어의 Body 데이터에 접근할 방법이 없음
        rhai::Array::new()
    });

    // get_leader(ob) - 내가 팔로우 중인 리더 이름 반환
    let body_ptr_gl = body_ptr;
    engine.register_fn("get_leader", move |_ob: &mut rhai::Map| -> String {
        let body = unsafe { &*body_ptr_gl };
        body.get_string("팔로우_리더")
    });

    // create_party(ob, party_name) - 파티 생성. 성공 시 "", 실패 시 에러 문자열
    let body_ptr_cp = body_ptr;
    engine.register_fn(
        "create_party",
        move |_ob: &mut rhai::Map, party_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_cp };
            let my_name = body.get_name();

            if party_name.trim().is_empty() {
                return "파티 이름을 입력해주세요.".to_string();
            }

            // 이미 파티에 소속되어 있는지 확인
            let current_party = body.get_string("소속파티");
            if !current_party.is_empty() {
                return format!("이미 {} 파티에 소속되어 있습니다.", current_party);
            }

            // 파티 이름 생성 (플레이어명_파티명 형식으로 고유화)
            let full_party_name = format!("{}_{}", my_name, party_name);

            // 파티 소속 설정 (Body)
            body.set("소속파티", full_party_name.clone());
            body.set("파티장", 1i64);
            body.set("파티이름", party_name.to_string());

            // WorldState에 파티 멤버십 등록
            if let Ok(mut w) = get_world_state().try_write() {
                w.join_party(&my_name, &full_party_name);
            }

            "".to_string()
        },
    );

    // join_party(ob, party_name) - 파티 가입. 성공 시 "", 실패 시 에러 문자열
    let body_ptr_jp = body_ptr;
    engine.register_fn(
        "join_party",
        move |_ob: &mut rhai::Map, party_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_jp };
            let my_name = body.get_name();

            if party_name.trim().is_empty() {
                return "파티 이름을 입력해주세요.".to_string();
            }

            // 이미 파티에 소속되어 있는지 확인
            let current_party = body.get_string("소속파티");
            if !current_party.is_empty() {
                return format!("이미 {} 파티에 소속되어 있습니다.", current_party);
            }

            // 파티장(생성자) 확인
            // party_name 형식: "파티장명_파티명" 또는 그냥 "파티명"
            let leader_name = if party_name.contains('_') {
                party_name.split('_').next().unwrap_or("").to_string()
            } else {
                return "존재하지 않는 파티입니다.".to_string();
            };

            let leader_exists = if let Ok(w) = get_world_state().try_read() {
                w.player_positions.contains_key(&leader_name)
            } else {
                false
            };

            if !leader_exists {
                return "파티장이 접속 중이 아닙니다.".to_string();
            }

            // 파티 소속 설정 (Body)
            body.set("소속파티", party_name.to_string());
            body.set("파티장", 0i64);

            // WorldState에 파티 멤버십 등록
            if let Ok(mut w) = get_world_state().try_write() {
                w.join_party(&my_name, party_name);
            }

            "".to_string()
        },
    );

    // leave_party(ob) - 파티 탈퇴. 성공 시 true
    let body_ptr_lp = body_ptr;
    engine.register_fn("leave_party", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr_lp };
        let my_name = body.get_name();

        let current_party = body.get_string("소속파티");
        if current_party.is_empty() {
            return false;
        }

        // 파티 정보 제거 (Body)
        body.set("소속파티", "");
        body.set("파티장", 0i64);
        body.set("파티이름", "");

        // WorldState에서 파티 멤버십 제거
        if let Ok(mut w) = get_world_state().try_write() {
            w.leave_party(&my_name);
        }

        true
    });

    // get_party_members(ob) - 파티 멤버 목록 반환 (Array of names)
    let body_ptr_gpm = body_ptr;
    engine.register_fn(
        "get_party_members",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_gpm };
            let mut members = rhai::Array::new();

            let my_party = body.get_string("소속파티");
            if my_party.is_empty() {
                return members;
            }

            // WorldState에서 파티 멤버 목록 조회
            if let Ok(w) = get_world_state().try_read() {
                let party_members = w.get_party_members(&my_party);
                for member_name in party_members {
                    members.push(Dynamic::from(member_name));
                }
            }

            members
        },
    );

    // send_party_message(ob, message) - 파티 메시지 전송. 성공 시 true
    let spec_pm = spec.clone();
    let body_ptr_pm = body_ptr;
    engine.register_fn(
        "send_party_message",
        move |_ob: &mut rhai::Map, msg: &str| -> bool {
            let body = unsafe { &*body_ptr_pm };
            let my_name = body.get_name();
            let my_party = body.get_string("소속파티");

            if my_party.is_empty() || msg.trim().is_empty() {
                return false;
            }

            // WorldState에서 파티 멤버 목록 조회
            let member_names: Vec<String> = if let Ok(w) = get_world_state().try_read() {
                w.get_party_members(&my_party)
            } else {
                Vec::new()
            };

            let formatted = format!("\x1b[0;36m[파티]\x1b[0;37m {} : {}", my_name, msg);
            if let Ok(mut s) = spec_pm.lock() {
                *s = Some(CommandResult::BroadcastToPlayers(member_names, formatted));
            }

            true
        },
    );

    // is_same_party(ob1_name, ob2_name) - 두 플레이어가 같은 파티인지 확인
    engine.register_fn(
        "is_same_party",
        move |ob1_name: &str, ob2_name: &str| -> bool {
            // WorldState에서 파티 멤버십 확인
            if let Ok(w) = get_world_state().try_read() {
                w.is_same_party(ob1_name, ob2_name)
            } else {
                false
            }
        },
    );

    // ============================================================
    // 오브젝트/아이템 조작 관련 efun
    // ============================================================

    // find_obj_in_inventory(ob, obj_name) - 플레이어 인벤토리에서 오브젝트 찾기
    // 오브젝트를 찾으면 오브젝트 데이터를 반환, 없으면 UNIT 반환
    let body_ptr_fii = body_ptr;
    engine.register_fn(
        "find_obj_in_inventory",
        move |_ob: &mut rhai::Map, obj_name: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_fii };
            for obj_arc in &body.object.objs {
                if let Ok(obj) = obj_arc.lock() {
                    let matches = obj.getName() == obj_name
                        || (!obj.getString("반응이름").is_empty()
                            && obj.getString("반응이름").contains(obj_name));
                    if matches {
                        // 오브젝트 데이터를 Map으로 변환하여 반환
                        let mut obj_data = rhai::Map::new();
                        obj_data.insert("이름".into(), Dynamic::from(obj.getName()));
                        obj_data.insert("표시".into(), Dynamic::from(obj.getNameA())); // getNameA를 표시로 사용
                        obj_data.insert("종류".into(), Dynamic::from(obj.getString("종류")));
                        drop(obj);
                        return Dynamic::from(obj_data);
                    }
                }
            }
            Dynamic::UNIT
        },
    );

    // drop_item(ob, item_name, count) - 아이템을 바닥에 버리기
    // 성공 시 빈 문자열 "", 실패 시 오류 메시지 반환
    let body_ptr_di = body_ptr;
    engine.register_fn(
        "drop_item",
        move |_ob: &mut rhai::Map, item_name: &str, count: i64| -> String {
            if item_name.is_empty() {
                return "아이템 이름을 입력해주세요.".to_string();
            }
            let body = unsafe { &mut *body_ptr_di };
            let count = count.clamp(1, 100) as usize;
            let mut w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return "플레이어 위치를 찾을 수 없습니다.".to_string(),
            };

            // 스택 아이템 처리
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    let drop_cnt = (count as i64).min(have).max(0);
                    if drop_cnt <= 0 {
                        return format!("{}을(를) 가지고 있지 않습니다.", item_name);
                    }
                    let should_remove = {
                        let r = body.object.inv_stack.get_mut(key).unwrap();
                        *r -= drop_cnt;
                        *r <= 0
                    };
                    if should_remove {
                        body.object.inv_stack.remove(key);
                    }
                    let room_stack = w.get_room_objs_stack_mut(&zone, &room);
                    *room_stack.entry(key.clone()).or_insert(0) += drop_cnt;
                    drop(w);
                    let path = format!("data/user/{}.json", body.get_name());
                    let _ = save_body_to_json(body, &path);
                    return String::new();
                }
            }

            // 비스택 아이템 처리
            let mut dropped = 0usize;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let ok = o.getName() == item_name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(item_name));
                    if !ok || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함") {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "버리지못함") {
                        drop(o);
                        continue;
                    }
                    drop(o);
                    to_remove.push(obj.clone());
                    dropped += 1;
                    if dropped >= count {
                        break;
                    }
                }
            }

            if dropped == 0 {
                return format!("{}을(를) 가지고 있지 않습니다.", item_name);
            }

            let room_objs = w.get_room_objs_mut(&zone, &room);
            for arc in to_remove {
                body.object.remove(&arc);
                room_objs.push(arc);
            }
            drop(w);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            String::new()
        },
    );

    // pick_up_item(ob, item_name, count) - 바닥에서 아이템 줍기
    // 성공 시 빈 문자열 "", 실패 시 오류 메시지 반환
    // 관리자(등급>=1000)는 무게/수량 제한 없음
    const MAX_ITEMS_PICKUP: usize = 50;
    let body_ptr_pui = body_ptr;
    engine.register_fn(
        "pick_up_item",
        move |_ob: &mut rhai::Map, item_name: &str, count: i64| -> String {
            if item_name.is_empty() {
                return "아이템 이름을 입력해주세요.".to_string();
            }
            let body = unsafe { &mut *body_ptr_pui };
            let admin_level = body.get_int("관리자등급");
            let is_admin = admin_level >= 1000;
            let count = count.clamp(1, 100) as usize;
            let mut w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return "플레이어 위치를 찾을 수 없습니다.".to_string(),
            };

            let mut taken = 0usize;

            // 스택 아이템 처리
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let room_stack = w.get_room_objs_stack_mut(&zone, &room);
                    let have = *room_stack.get(key).unwrap_or(&0);
                    let take_cnt = (count as i64).min(have).max(0) as usize;
                    if take_cnt > 0 {
                        // 관리자가 아니면 무게/수량 체크
                        if !is_admin {
                            // get_item_info returns (name, rn, price, weight)
                            let item_weight = get_item_info(key).map(|(_, _, _, w)| w).unwrap_or(0);
                            let total_weight = item_weight * take_cnt as i64;
                            if body.get_item_weight() + total_weight > body.get_str() * 10 {
                                return "무거워서 더 이상 들 수 없습니다.".to_string();
                            }
                            if body.get_item_count() + take_cnt > MAX_ITEMS_PICKUP {
                                return "소지품이 가득 찼습니다.".to_string();
                            }
                        }
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

            // 바닥 Object에서 가져오기 (비스택 또는 예전 드랍)
            let room_list = w.get_room_objs_mut(&zone, &room);
            let mut i = 0;
            while i < room_list.len() && taken < count {
                let (matches, item_weight) = {
                    let o = room_list[i].lock().unwrap();
                    let m = o.getName() == item_name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(item_name));
                    (m, o.getInt("무게"))
                };
                if matches {
                    // 관리자가 아니면 무게/수량 체크
                    if !is_admin {
                        if body.get_item_weight() + item_weight > body.get_str() * 10 {
                            if taken == 0 {
                                return "무거워서 더 이상 들 수 없습니다.".to_string();
                            }
                            break;
                        }
                        if body.get_item_count() + 1 > MAX_ITEMS_PICKUP {
                            if taken == 0 {
                                return "소지품이 가득 찼습니다.".to_string();
                            }
                            break;
                        }
                    }
                    let arc = room_list.remove(i);
                    body.object.append(arc);
                    taken += 1;
                } else {
                    i += 1;
                }
            }

            if taken == 0 {
                return format!("여기에는 {}이(가) 없습니다.", item_name);
            }

            drop(w);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            String::new()
        },
    );

    // move_item_to_room(ob, item_name, room) - 특정 방으로 아이템 이동
    // room 형식: "zone:room_id" (예: "낙양성:1")
    // 성공 시 true 반환
    let body_ptr_mitr = body_ptr;
    engine.register_fn(
        "move_item_to_room",
        move |_ob: &mut rhai::Map, item_name: &str, room: &str| -> bool {
            if item_name.is_empty() || room.is_empty() {
                return false;
            }
            let body = unsafe { &mut *body_ptr_mitr };

            // room 파싱: "zone:room_id"
            let parts: Vec<&str> = room.split(':').collect();
            if parts.len() != 2 {
                return false;
            }
            let target_zone = parts[0];
            let target_room = parts[1];

            let mut w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return false,
            };

            let mut moved = false;

            // 스택 아이템 처리 (전체 수량 이동)
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    if let Some(&have) = body.object.inv_stack.get(key) {
                        if have > 0 {
                            body.object.inv_stack.remove(key);
                            let target_stack = w.get_room_objs_stack_mut(target_zone, target_room);
                            *target_stack.entry(key.clone()).or_insert(0) += have;
                            moved = true;
                        }
                    }
                }
            }

            // 비스택 아이템 처리 (첫 번째 매칭 아이템만 이동)
            if !moved {
                let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
                for obj in &body.object.objs {
                    if let Ok(o) = obj.lock() {
                        let ok = o.getName() == item_name
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(item_name));
                        if ok {
                            drop(o);
                            to_remove.push(obj.clone());
                            break; // 첫 번째 아이템만 이동
                        }
                    }
                }

                if !to_remove.is_empty() {
                    let target_room_objs = w.get_room_objs_mut(target_zone, target_room);
                    for arc in to_remove {
                        body.object.remove(&arc);
                        target_room_objs.push(arc);
                    }
                    moved = true;
                }
            }

            if moved {
                drop(w);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }

            moved
        },
    );

    // get_obj_attr(ob, obj_name, attr) - 오브젝트 속성 가져오기
    // 속성 값 반환, 없으면 UNIT 반환
    let body_ptr_goa = body_ptr;
    engine.register_fn(
        "get_obj_attr",
        move |_ob: &mut rhai::Map, obj_name: &str, attr: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_goa };

            // 인벤토리에서 검색
            for obj_arc in &body.object.objs {
                if let Ok(obj) = obj_arc.lock() {
                    let matches = obj.getName() == obj_name
                        || (!obj.getString("반응이름").is_empty()
                            && obj.getString("반응이름").contains(obj_name));
                    if matches {
                        let value = obj.get(attr);
                        drop(obj);
                        // Value 타입을 Dynamic으로 변환
                        match value {
                            crate::object::Value::Int(n) => return Dynamic::from_int(n),
                            crate::object::Value::String(s) => return Dynamic::from(s),
                            crate::object::Value::Float(f) => return Dynamic::from(f),
                        }
                    }
                }
            }

            // 현재 방의 바닥에서 검색
            if let Ok(w) = get_world_state().read() {
                if let Some(pos) = w.get_player_position(body.get_name().as_str()) {
                    let room_objs = w.get_room_objs(&pos.zone, &pos.room);
                    for obj_arc in room_objs {
                        if let Ok(obj) = obj_arc.lock() {
                            let matches = obj.getName() == obj_name
                                || (!obj.getString("반응이름").is_empty()
                                    && obj.getString("반응이름").contains(obj_name));
                            if matches {
                                let value = obj.get(attr);
                                drop(obj);
                                match value {
                                    crate::object::Value::Int(n) => return Dynamic::from_int(n),
                                    crate::object::Value::String(s) => return Dynamic::from(s),
                                    crate::object::Value::Float(f) => return Dynamic::from(f),
                                }
                            }
                        }
                    }
                }
            }

            Dynamic::UNIT
        },
    );

    // destroy_item(ob, item_name, count) - 아이템 완전히 파괴
    // 파괴된 아이템 수 반환
    let body_ptr_dest = body_ptr;
    engine.register_fn(
        "destroy_item",
        move |_ob: &mut rhai::Map, item_name: &str, count: i64| -> i64 {
            if item_name.is_empty() {
                return 0;
            }
            let body = unsafe { &mut *body_ptr_dest };
            let count = count.clamp(1, 100) as usize;

            let mut destroyed = 0i64;

            // 스택 아이템 파괴
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    let destroy_cnt = (count as i64).min(have).max(0);
                    if destroy_cnt > 0 {
                        let should_remove = {
                            let r = body.object.inv_stack.get_mut(key).unwrap();
                            *r -= destroy_cnt;
                            *r <= 0
                        };
                        if should_remove {
                            body.object.inv_stack.remove(key);
                        }
                        destroyed += destroy_cnt;
                    }
                }
            }

            // 비스택 아이템 파괴
            if destroyed < count as i64 {
                let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
                for obj in &body.object.objs {
                    if let Ok(o) = obj.lock() {
                        let ok = o.getName() == item_name
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(item_name));
                        if !ok || o.getBool("inUse") {
                            continue;
                        }
                        drop(o);
                        to_remove.push(obj.clone());
                        destroyed += 1;
                        if destroyed >= count as i64 {
                            break;
                        }
                    }
                }

                for arc in to_remove {
                    body.object.remove(&arc);
                }
            }

            if destroyed > 0 {
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }

            destroyed
        },
    );

    // give_item_to_mob(ob, mob_name, item_name) - 몹에게 아이템 주기
    // 성공 시 true 반환
    let body_ptr_gitm = body_ptr;
    engine.register_fn(
        "give_item_to_mob",
        move |_ob: &mut rhai::Map, mob_name: &str, item_name: &str| -> bool {
            if item_name.is_empty() {
                return false;
            }
            let body = unsafe { &mut *body_ptr_gitm };

            let w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return false,
            };

            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return false,
            };

            // 방에 있는 몹 찾기
            let mobs = w.mob_cache.get_mobs_in_room(&zone, &room);
            let mob_found = mobs.iter().any(|m| m.name == mob_name);

            if !mob_found {
                return false;
            }

            let mut given = false;

            // 스택 아이템 처리 (1개만 주기)
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    if have > 0 {
                        let should_remove = {
                            let r = body.object.inv_stack.get_mut(key).unwrap();
                            *r -= 1;
                            *r <= 0
                        };
                        if should_remove {
                            body.object.inv_stack.remove(key);
                        }
                        // TODO: 몹 인벤토리에 추가 로직 필요 (현재는 삭제만)
                        given = true;
                    }
                }
            }

            // 비스택 아이템 처리
            if !given {
                let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
                for obj in &body.object.objs {
                    if let Ok(o) = obj.lock() {
                        let ok = o.getName() == item_name
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(item_name));
                        if ok && !o.getBool("inUse") {
                            drop(o);
                            to_remove.push(obj.clone());
                            break;
                        }
                    }
                }
                for arc in to_remove {
                    body.object.remove(&arc);
                    // TODO: 몹 인벤토리에 추가 로직 필요 (현재는 삭제만)
                    given = true;
                }
            }

            if given {
                drop(w);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }

            given
        },
    );

    // ============================================================
    // Admin command efun (관리자 명령)
    // ============================================================

    // summon_player(admin_ob, target_name) - 대상 플레이어를 관리자의 현재 위치로 소환
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let body_ptr_summon = body_ptr;
    engine.register_fn(
        "summon_player",
        move |admin_ob: &mut rhai::Map, target_name: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_summon };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 자기 자신 소환 불가
            if target_name == admin_name {
                return "자기 자신을 소환할 수 없습니다.".to_string();
            }

            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            // 관리자의 현재 위치 확인
            let admin_pos = match w.get_player_position(&admin_name).cloned() {
                Some(p) => p,
                None => return "관리자의 위치를 찾을 수 없습니다.".to_string(),
            };

            // 대상 플레이어의 현재 위치 확인
            let target_pos = match w.get_player_position(target_name) {
                Some(p) => p.clone(),
                None => return "대상을 찾을 수 없습니다.".to_string(),
            };

            // 이미 같은 위치에 있는지 확인
            if target_pos.zone == admin_pos.zone && target_pos.room == admin_pos.room {
                return "이미 같은 위치에 있습니다.".to_string();
            }

            // 대상을 관리자의 위치로 이동
            w.set_player_position(target_name, admin_pos.clone());
            w.spawn_mobs_for_room(&admin_pos.zone, &admin_pos.room);

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // goto_player(admin_ob, target_name) - 관리자가 대상 플레이어의 위치로 이동
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let body_ptr_goto = body_ptr;
    engine.register_fn(
        "goto_player",
        move |admin_ob: &mut rhai::Map, target_name: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_goto };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 자기 자신에게 이동 불가
            if target_name == admin_name {
                return "자기 자신에게 이동할 수 없습니다.".to_string();
            }

            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            // 대상 플레이어의 현재 위치 확인
            let target_pos = match w.get_player_position(target_name).cloned() {
                Some(p) => p,
                None => return "대상을 찾을 수 없습니다.".to_string(),
            };

            // 관리자의 현재 위치 확인
            let admin_pos = match w.get_player_position(&admin_name) {
                Some(p) => p.clone(),
                None => return "관리자의 위치를 찾을 수 없습니다.".to_string(),
            };

            // 이미 같은 위치에 있는지 확인
            if admin_pos.zone == target_pos.zone && admin_pos.room == target_pos.room {
                return "이미 같은 위치에 있습니다.".to_string();
            }

            // 관리자를 대상의 위치로 이동
            w.set_player_position(&admin_name, target_pos.clone());
            w.spawn_mobs_for_room(&target_pos.zone, &target_pos.room);

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // kick_player(admin_ob, target_name) - 플레이어 강제 로그아웃
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let spec_kick = spec.clone();
    let body_ptr_kick = body_ptr;
    engine.register_fn(
        "kick_player",
        move |admin_ob: &mut rhai::Map, target_name: &str| -> String {
            use crate::command::handler::CommandResult;

            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_kick };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 자기 자신 킥 불가
            if target_name == admin_name {
                return "자기 자신을 킥할 수 없습니다.".to_string();
            }

            // 대상이 접속 중인지 확인
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            if w.get_player_position(target_name).is_none() {
                return "대상을 찾을 수 없습니다.".to_string();
            }

            // CommandResult::Kick 설정 (핸들러에서 실제 처리)
            if let Ok(mut s) = spec_kick.lock() {
                *s = Some(CommandResult::Kick {
                    target_name: target_name.to_string(),
                    reason: "관리자에 의해 강제 로그아웃되었습니다.".to_string(),
                });
            }

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // ban_player(admin_ob, target_name, duration) - 플레이어 접속 차단
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let spec_ban = spec.clone();
    let body_ptr_ban = body_ptr;
    engine.register_fn(
        "ban_player",
        move |admin_ob: &mut rhai::Map, target_name: &str, duration: i64| -> String {
            use crate::command::handler::CommandResult;

            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_ban };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 기간 체크
            if duration <= 0 {
                return "차단 기간은 0보다 커야 합니다.".to_string();
            }

            // 자기 자신 밴 불가
            if target_name == admin_name {
                return "자기 자신을 밴할 수 없습니다.".to_string();
            }

            // CommandResult::Ban 설정 (핸들러에서 실제 처리)
            if let Ok(mut s) = spec_ban.lock() {
                *s = Some(CommandResult::Ban {
                    target_name: target_name.to_string(),
                    duration,
                    reason: format!("관리자에 의해 {}초간 접속이 차단되었습니다.", duration),
                });
            }

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // set_player_level(admin_ob, target_name, level) - 플레이어 레벨 설정
    // Returns true on success
    // Admin level 2000 required
    let _body_ptr_set_lvl = body_ptr;
    engine.register_fn(
        "set_player_level",
        move |admin_ob: &mut rhai::Map, target_name: &str, level: i64| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return false;
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return false;
            }

            // 레벨 범위 체크
            if !(1..=1000).contains(&level) {
                return false;
            }

            // TODO: 대상 플레이어의 Body에 접근하여 레벨 설정
            // 현재는 로그만 출력하고 true 반환
            println!(
                "[SCRIPT] set_player_level: {} -> level {}",
                target_name, level
            );
            true
        },
    );

    // set_player_money(admin_ob, target_name, amount) - 플레이어 돈 설정
    // Returns true on success
    // Admin level 2000 required
    let _body_ptr_set_money = body_ptr;
    engine.register_fn(
        "set_player_money",
        move |admin_ob: &mut rhai::Map, target_name: &str, amount: i64| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return false;
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return false;
            }

            // 금액 범위 체크
            if !(0..=1_000_000_000).contains(&amount) {
                return false;
            }

            // TODO: 대상 플레이어의 Body에 접근하여 돈 설정
            // 현재는 로그만 출력하고 true 반환
            println!(
                "[SCRIPT] set_player_money: {} -> {} 은전",
                target_name, amount
            );
            true
        },
    );

    // set_player_hp(admin_ob, target_name, hp) - 플레이어 HP 설정
    // Returns true on success
    // Admin level 2000 required
    let _body_ptr_set_hp_player = body_ptr;
    engine.register_fn(
        "set_player_hp",
        move |admin_ob: &mut rhai::Map, target_name: &str, hp: i64| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return false;
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return false;
            }

            // HP 범위 체크
            if !(0..=1_000_000).contains(&hp) {
                return false;
            }

            // TODO: 대상 플레이어의 Body에 접근하여 HP 설정
            // 현재는 로그만 출력하고 true 반환
            println!("[SCRIPT] set_player_hp: {} -> {} HP", target_name, hp);
            true
        },
    );

    // create_user_mob(admin_ob, mob_name) - 몹 생성 (관리자)
    // mob_name should be "zone:filename" format (e.g., "낙양성:밍밍")
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let body_ptr_create_mob = body_ptr;
    engine.register_fn(
        "create_user_mob",
        move |admin_ob: &mut rhai::Map, mob_name: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_create_mob };
            let admin_name = body.get_name();

            // 빈 몹 이름 체크
            if mob_name.trim().is_empty() {
                return "몹 이름을 입력해주세요.".to_string();
            }

            // 관리자 현재 위치 확인
            let (zone, room) = {
                let w = match get_world_state().read() {
                    Ok(g) => g,
                    Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
                };
                let pos = match w.get_player_position(&admin_name) {
                    Some(p) => p.clone(),
                    None => return "위치 정보를 찾을 수 없습니다.".to_string(),
                };
                (pos.zone, pos.room)
            };

            // 몹 생성 (mob_name은 "zone:filename" 형식)
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            match w.spawn_mob_at(mob_name, &zone, &room) {
                Ok(()) => {
                    println!("[SCRIPT] create_user_mob: {} spawned at {}:{} by {}", mob_name, zone, room, admin_name);
                    String::new() // 성공 시 빈 문자열 반환
                }
                Err(e) => e,
            }
        },
    );

    // remove_user_mob(admin_ob, mob_name) - 사용자 정의 몹 제거
    // Returns true on success
    // Admin level 2000 required
    let body_ptr_remove_mob = body_ptr;
    engine.register_fn(
        "remove_user_mob",
        move |admin_ob: &mut rhai::Map, mob_name: &str| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return false;
            }

            let body = unsafe { &*body_ptr_remove_mob };
            let admin_name = body.get_name();

            // 빈 몹 이름 체크
            if mob_name.trim().is_empty() {
                return false;
            }

            // TODO: 실제 사용자 정의 몹 제거 로직 구현
            // 몹 데이터 파일 삭제 및 등록 해제
            println!("[SCRIPT] remove_user_mob: {} by {}", mob_name, admin_name);
            true
        },
    );

    // warp_player(admin_ob, target_name, zone, room) - 플레이어를 특정 위치로 이동
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let _body_ptr_warp = body_ptr;
    engine.register_fn(
        "warp_player",
        move |admin_ob: &mut rhai::Map, target_name: &str, zone: &str, room: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 빈 zone/room 체크
            if zone.trim().is_empty() || room.trim().is_empty() {
                return "위치를 입력해주세요 (zone:room).".to_string();
            }

            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            // 대상 플레이어의 현재 위치 확인
            let current_pos = w.get_player_position(target_name);
            if current_pos.is_none() {
                return "대상을 찾을 수 없습니다.".to_string();
            }

            let room_s = room.to_string();

            // 방 존재 확인
            if w.room_cache.get_room(zone, &room_s).is_err() {
                return "해당 위치를 찾을 수 없습니다.".to_string();
            }

            // 플레이어 이동
            w.set_player_position(
                target_name,
                PlayerPosition::new(zone.to_string(), room_s.clone()),
            );
            w.spawn_mobs_for_room(zone, &room_s);

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // admin_force_command(admin_ob, target_name, command) - 대상 플레이어에게 명령 강제 실행
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    // Note: This adds the command to user_sends which will be processed as if the player typed it
    let user_sends_force = user_sends.clone();
    engine.register_fn(
        "admin_force_command",
        move |admin_ob: &mut rhai::Map, target_name: &str, command: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 빈 명령어 체크
            if command.trim().is_empty() {
                return "명령어를 입력해주세요.".to_string();
            }

            // 플레이어가 접속 중인지 확인
            let online = if let Ok(w) = get_world_state().try_read() {
                w.player_positions.contains_key(target_name)
            } else {
                return "월드 상태를 확인할 수 없습니다.".to_string();
            };

            if !online {
                return "대상 플레이어가 접속 중이 아닙니다.".to_string();
            }

            // 명령어를 플레이어의 큐에 추가
            // user_sends에 (target_name, command) 형태로 추가
            // 이것은 나중에 플레이어가 입력한 것처럼 처리됨
            if let Ok(mut sends) = user_sends_force.lock() {
                sends.push((target_name.to_string(), command.to_string()));
                String::new() // 성공 시 빈 문자열 반환
            } else {
                "명령어 큐에 추가할 수 없습니다.".to_string()
            }
        },
    );

    // ============================================================
    // HELPER/UTILITY FUNCTIONS (Display formatting)
    // ============================================================
    // Note: Text formatting functions (format_bar, format_money, format_number,
    // get_item_display, get_mob_display, time_to_string) are now implemented
    // in lib/format.rhai for hot-reload capability.
    //
    // However, format_item_name and format_mob_name are frequently used and
    // kept in Rust for performance.

    // format_item_name - Item name with color (frequently used, kept in Rust)
    engine.register_fn("format_item_name", |display_name: &str| -> String {
        format!("\x1b[1;37m{}\x1b[0;37m", display_name)
    });

    // format_mob_name - Mob name with color (frequently used, kept in Rust)
    engine.register_fn("format_mob_name", |display_name: &str| -> String {
        format!("\x1b[1;33m{}\x1b[0;37m", display_name)
    });

    // ============================================================
    // 호위 (Guard/Protection) 시스템 efun
    // ============================================================

    // add_guard(ob, mob_name) - 몹을 호위로 추가
    // Returns "" on success, error string on failure
    let body_ptr_add_guard = body_ptr;
    engine.register_fn(
        "add_guard",
        move |_ob: &mut rhai::Map, mob_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_add_guard };

            if mob_name.trim().is_empty() {
                return "몹 이름을 입력해주세요.".to_string();
            }

            // 몹 데이터 확인
            let mob_data = match get_mob_by_name_impl(mob_name) {
                Some(data) => data,
                None => return format!("몹 '{}'을(를) 찾을 수 없습니다.", mob_name),
            };

            let guard_name = mob_data
                .get("이름")
                .and_then(|v| v.as_str())
                .unwrap_or(mob_name);

            let max_hp = mob_data.get("체력").and_then(|v| v.as_i64()).unwrap_or(100);

            let desc = mob_data.get("설명2").and_then(|v| v.as_str()).unwrap_or("");

            // 현재 호위 목록 가져오기
            let mut guards = parse_guards_list(&body.get_string("호위_리스트"));

            // 이미 있는 호위인지 확인
            if guards.iter().any(|g| g.name == guard_name) {
                return format!("{}은(는) 이미 호위로 있습니다.", guard_name);
            }

            // 호위 추가
            guards.push(crate::script::GuardData {
                name: guard_name.to_string(),
                hp: max_hp,
                max_hp,
                description: desc.to_string(),
            });

            // 호위 목록 저장
            body.set("호위_리스트", format_guards_list(&guards));

            String::new()
        },
    );

    // remove_guard(ob, mob_name) - 호위 제거
    // Returns true on success
    let body_ptr_remove_guard = body_ptr;
    engine.register_fn(
        "remove_guard",
        move |_ob: &mut rhai::Map, mob_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_remove_guard };

            if mob_name.trim().is_empty() {
                return false;
            }

            let mut guards = parse_guards_list(&body.get_string("호위_리스트"));
            let original_len = guards.len();

            guards.retain(|g| g.name != mob_name);

            if guards.len() < original_len {
                body.set("호위_리스트", format_guards_list(&guards));
                true
            } else {
                false
            }
        },
    );

    // get_guards(ob) - 호위 목록 가져오기
    // Returns Array of guard data (이름, 체력, max_체력, 설명)
    let body_ptr_get_guards = body_ptr;
    engine.register_fn("get_guards", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_get_guards };
        let guards = parse_guards_list(&body.get_string("호위_리스트"));

        let mut result = rhai::Array::new();
        for guard in guards {
            let mut guard_map = rhai::Map::new();
            guard_map.insert("이름".into(), Dynamic::from(guard.name.clone()));
            guard_map.insert("체力".into(), Dynamic::from(guard.hp));
            guard_map.insert("max_체력".into(), Dynamic::from(guard.max_hp));
            guard_map.insert("설명".into(), Dynamic::from(guard.description));
            result.push(Dynamic::from(guard_map));
        }
        result
    });

    // get_guard_count(ob) - 호위 수 가져오기
    // Returns count as i64
    let body_ptr_guard_count = body_ptr;
    engine.register_fn("get_guard_count", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_guard_count };
        let guards = parse_guards_list(&body.get_string("호위_리스트"));
        guards.len() as i64
    });

    // get_anger(ob) - 분노 (anger) 점수 가져오기
    let body_ptr_get_anger = body_ptr;
    engine.register_fn("get_anger", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_get_anger };
        body.get_int("분노")
    });

    // set_anger(ob, value) - 분노 점수 설정
    // Returns true on success
    let body_ptr_set_anger = body_ptr;
    engine.register_fn(
        "set_anger",
        move |_ob: &mut rhai::Map, value: i64| -> bool {
            let body = unsafe { &mut *body_ptr_set_anger };
            let clamped = value.clamp(0, 10000); // 분노 값 범위 제한
            body.set("분노", clamped);
            true
        },
    );

    // guard_fight(ob) - 호위가 싸우게 하기
    // Returns true if any guard attacked
    let body_ptr_guard_fight = body_ptr;
    engine.register_fn("guard_fight", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &*body_ptr_guard_fight };
        let guards = parse_guards_list(&body.get_string("호위_리스트"));

        if guards.is_empty() {
            return false;
        }

        // TODO: 실제 전투 로직 구현
        // 현재는 호위가 있으면 true 반환
        println!(
            "[SCRIPT] guard_fight: {} guards attacking for {}",
            guards.len(),
            body.get_name()
        );
        true
    });

    // find_guard_in_room(ob, mob_name) - 방의 몹이 플레이어의 호위인지 확인
    // Returns true if mob is player's guard
    let body_ptr_find_guard = body_ptr;
    engine.register_fn(
        "find_guard_in_room",
        move |_ob: &mut rhai::Map, mob_name: &str| -> bool {
            let body = unsafe { &*body_ptr_find_guard };
            let guards = parse_guards_list(&body.get_string("호위_리스트"));

            guards.iter().any(|g| g.name == mob_name)
        },
    );

    // ============================================================
    // SHOP/MERCHANT SYSTEM EFUNS
    // ============================================================

    // get_shop_mobs(ob) - 현재 방의 상인(상점) 몹 목록 반환
    // Returns: Array of mob names that are merchants (have items_for_sale or buy_percent > 0)
    let body_ptr_shop_mobs = body_ptr;
    engine.register_fn("get_shop_mobs", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_shop_mobs };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Array::new(),
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return rhai::Array::new(),
        };
        let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
        let mut arr = rhai::Array::new();
        for mob in mobs {
            if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                // 상인 확인: 물건판매 있거나 물건구입 비율이 0보다 큰 경우
                if !data.items_for_sale.is_empty() || data.buy_percent > 0 {
                    arr.push(Dynamic::from(data.name.clone()));
                }
            }
        }
        arr
    });

    // get_shop_items(ob, mob_name) - 특정 상인이 판매하는 아이템 목록 반환
    // Returns: Array of {name, price, count} maps
    let body_ptr_shop_items = body_ptr;
    engine.register_fn(
        "get_shop_items",
        move |_ob: &mut rhai::Map, mob_name: &str| -> rhai::Array {
            let body = unsafe { &*body_ptr_shop_items };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut arr = rhai::Array::new();
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    // 몹 이름 매칭 (정확히 일치하거나 포함)
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    // items_for_sale 목록 반환
                    for (item_key, percent) in &data.items_for_sale {
                        if let Some((iname, _, base_price, _)) = get_item_info(item_key) {
                            let p = (*percent).max(1);
                            let price = base_price * 100 / p;
                            let mut item_map = rhai::Map::new();
                            item_map.insert("name".into(), Dynamic::from(iname.clone()));
                            item_map.insert("price".into(), Dynamic::from(price));
                            item_map.insert("count".into(), Dynamic::from(1i64)); // 기본값: 1 (무제한인 경우)
                            arr.push(Dynamic::from(item_map));
                        }
                    }
                    break;
                }
            }
            arr
        },
    );

    // buy_from_shop(ob, mob_name, item_name, count) - 상인에게 아이템 구매
    // Returns: "" on success, error code on failure
    // Error codes: "no_merchant", "not_for_sale", "no_money", "inv_full", "too_heavy"
    let body_ptr_buy_shop = body_ptr;
    engine.register_fn(
        "buy_from_shop",
        move |_ob: &mut rhai::Map, mob_name: &str, item_name: &str, count: i64| -> String {
            let body = unsafe { &mut *body_ptr_buy_shop };
            let pname = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "no_merchant".to_string(),
            };
            let pos = match w.get_player_position(&pname) {
                Some(p) => p,
                None => return "no_merchant".to_string(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut item_key = String::new();
            let mut unit_price = 0i64;
            let mut weight = 0i64;
            let mut _display_name = String::new();

            // 상인 찾기 및 아이템 확인
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    if data.items_for_sale.is_empty() {
                        return "no_merchant".to_string();
                    }
                    for (key, percent) in &data.items_for_sale {
                        let Some((iname, rn, price, wg)) = get_item_info(key) else {
                            continue;
                        };
                        let ok = iname == item_name || (!rn.is_empty() && rn.contains(item_name));
                        if !ok {
                            continue;
                        }
                        let p = (*percent).max(1);
                        unit_price = price * 100 / p;
                        weight = wg;
                        _display_name = iname;
                        item_key = key.clone();
                        break;
                    }
                    break;
                }
            }

            if item_key.is_empty() {
                return "not_for_sale".to_string();
            }

            let cnt = count.clamp(1, 50);
            const MAX_ITEMS: usize = 50;
            let is_admin = body.get_int("관리자등급") >= 1000;

            // 돈 확인
            let total_cost = unit_price * cnt;
            if body.get_int("은전") < total_cost {
                return "no_money".to_string();
            }

            // 인벤토리 공간 및 무게 확인 (관리자 제외)
            if !is_admin {
                if body.get_item_count() + cnt as usize > MAX_ITEMS {
                    return "inv_full".to_string();
                }
                if body.get_item_weight() + (weight * cnt) > body.get_str() * 10 {
                    return "too_heavy".to_string();
                }
            }

            // 아이템 추가 및 돈 차감
            for _ in 0..cnt {
                if is_stackable(&item_key) {
                    *body.object.inv_stack.entry(item_key.clone()).or_insert(0) += 1;
                } else if let Some((arc, _)) = object_from_item_json(&item_key) {
                    body.object.append(arc);
                } else {
                    return "not_for_sale".to_string();
                }
            }
            body.set("은전", body.get_int("은전") - total_cost);

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            String::new() // 성공
        },
    );

    // sell_to_shop(ob, mob_name, item_name, count) - 상인에게 아이템 판매
    // Returns: "" on success, error code on failure
    // Error codes: "no_merchant", "no_item", "cant_sell"
    let body_ptr_sell_shop = body_ptr;
    engine.register_fn(
        "sell_to_shop",
        move |_ob: &mut rhai::Map, mob_name: &str, item_name: &str, count: i64| -> String {
            let body = unsafe { &mut *body_ptr_sell_shop };
            let pname = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "no_merchant".to_string(),
            };
            let pos = match w.get_player_position(&pname) {
                Some(p) => p,
                None => return "no_merchant".to_string(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut buy_percent = 0i64;

            // 상인 찾기 및 구입 비율 확인
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    if data.buy_percent <= 0 {
                        return "no_merchant".to_string();
                    }
                    buy_percent = data.buy_percent;
                    break;
                }
            }

            if buy_percent <= 0 {
                return "no_merchant".to_string();
            }

            let count = count.clamp(1, 100) as usize;
            let _sold = 0usize;
            let mut total = 0i64;

            // 스택 아이템 먼저 확인
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    if let Some((iname, _, base_price, _)) = get_item_info(key) {
                        if iname == item_name {
                            let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                            let sell_cnt = (count as i64).clamp(0, have);
                            if sell_cnt > 0 {
                                let unit = (base_price * buy_percent) / 100;
                                total = unit * sell_cnt;
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
                                return String::new(); // 성공
                            }
                        }
                    }
                }
            }

            // 개별 아이템 확인
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let nm = o.getName();
                    let rn = o.getString("반응이름");
                    let match_ = nm == item_name || (!rn.is_empty() && rn.contains(item_name));
                    if !match_ || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함")
                    {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "팔지못함") {
                        return "cant_sell".to_string();
                    }
                    let price = (o.getInt("판매가격") * buy_percent) / 100;
                    total += price;
                    to_remove.push(obj.clone());
                    if to_remove.len() >= count {
                        break;
                    }
                }
            }

            if to_remove.is_empty() {
                return "no_item".to_string();
            }

            for arc in &to_remove {
                body.object.remove(arc);
            }
            body.set("은전", body.get_int("은전") + total);

            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            String::new() // 성공
        },
    );

    // get_shop_buy_price(mob_name) - 상인의 구입 비율 반환 (1-100)
    // get_merchant_buy_percent와 동일하지만 mob_name을 인자로 받음
    let body_ptr_get_buy_price = body_ptr;
    engine.register_fn(
        "get_shop_buy_price",
        move |_ob: &mut rhai::Map, mob_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_buy_price };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return 0,
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return 0,
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if (data.name == mob_name || data.name.contains(mob_name)) && data.buy_percent > 0 {
                        return data.buy_percent;
                    }
                }
            }
            0
        },
    );

    // get_shop_sell_price(mob_name) - 상인의 판매 비율 반환 (1-100)
    // items_for_sale에 있는 percent 값 반환 (첫 번째 아이템의 비율)
    let body_ptr_get_sell_price = body_ptr;
    engine.register_fn(
        "get_shop_sell_price",
        move |_ob: &mut rhai::Map, mob_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_sell_price };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return 0,
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return 0,
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if (data.name == mob_name || data.name.contains(mob_name))
                        && !data.items_for_sale.is_empty()
                    {
                        // 첫 번째 아이템의 판매 비율 반환
                        return data.items_for_sale[0].1.max(1);
                    }
                }
            }
            0
        },
    );

    // list_shop_inventory(ob, mob_name) - 상점 재고 목록 문자열 반환
    // Returns: 포맷된 재고 목록 문자열
    let body_ptr_list_shop = body_ptr;
    engine.register_fn(
        "list_shop_inventory",
        move |_ob: &mut rhai::Map, mob_name: &str| -> String {
            let body = unsafe { &*body_ptr_list_shop };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "상점 정보를 가져올 수 없습니다.".to_string(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return "상점 정보를 가져올 수 없습니다.".to_string(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut result = String::new();

            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    if data.items_for_sale.is_empty() {
                        result = format!("{}: 판매하는 물건이 없습니다.", data.name);
                        break;
                    }
                    result = format!("=== {} 상점 목록 ===\r\n", data.name);
                    for (item_key, percent) in &data.items_for_sale {
                        if let Some((iname, _, base_price, _)) = get_item_info(item_key) {
                            let p = (*percent).max(1);
                            let price = base_price * 100 / p;
                            result.push_str(&format!("  {} - {}은전\r\n", iname, price));
                        }
                    }
                    break;
                }
            }

            if result.is_empty() {
                "상인을 찾을 수 없습니다.".to_string()
            } else {
                result
            }
        },
    );

    // ============================================================
    // 방파(Guild) 시스템 efun
    // ============================================================

    // Helper function: 방파에 소속된 모든 멤버 이름을 data/user/*.json에서 검색
    fn get_guild_members_from_files(guild_name: &str) -> Vec<String> {
        let mut members = Vec::new();
        if let Ok(entries) = std::fs::read_dir("data/user") {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(uso) = json.get("사용자오브젝트").and_then(|v| v.as_object())
                        {
                            if let Some(attr) = uso.get("attr").and_then(|v| v.as_object()) {
                                if let Some(guild) = attr.get("소속").and_then(|v| v.as_str()) {
                                    if guild == guild_name {
                                        if let Some(name) = uso.get("이름").and_then(|v| v.as_str())
                                        {
                                            members.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        members
    }

    // Helper function: 방파 멤버의 직위를 가져옴 (data/user/*.json에서)
    fn get_guild_member_position(member_name: &str) -> String {
        let path = format!("data/user/{}.json", member_name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(uso) = json.get("사용자오브젝트").and_then(|v| v.as_object()) {
                    if let Some(attr) = uso.get("attr").and_then(|v| v.as_object()) {
                        if let Some(pos) = attr.get("직위").and_then(|v| v.as_str()) {
                            return pos.to_string();
                        }
                    }
                }
            }
        }
        String::new()
    }

    // Helper function: 방파 멤버의 직위를 설정 (data/user/*.json에 직접 저장)
    fn set_guild_member_position(member_name: &str, position: &str) -> bool {
        let path = format!("data/user/{}.json", member_name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(uso) = json
                    .get_mut("사용자오브젝트")
                    .and_then(|v| v.as_object_mut())
                {
                    if let Some(attr) = uso.get_mut("attr").and_then(|v| v.as_object_mut()) {
                        attr.insert(
                            "직위".to_string(),
                            serde_json::Value::String(position.to_string()),
                        );
                        if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                            return std::fs::write(&path, new_content).is_ok();
                        }
                    }
                }
            }
        }
        false
    }

    // guild_create(ob, guild_name) - 방파 생성
    // Returns "" on success, error string on failure
    // Admin level 1000 required
    let body_ptr_gc = body_ptr;
    engine.register_fn(
        "guild_create",
        move |ob: &mut rhai::Map, guild_name: &str| -> String {
            // 관리자 권한 확인
            let admin_level = ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 1000 {
                return "관리자 권한이 필요합니다 (등급 1000 이상).".to_string();
            }

            let body = unsafe { &mut *body_ptr_gc };

            // 빈 방파 이름 체크
            if guild_name.trim().is_empty() {
                return "방파 이름을 입력해주세요.".to_string();
            }

            // 중복 방파 이름 체크
            if crate::world::guild::guild_has(guild_name) {
                return "이미 존재하는 방파 이름입니다.".to_string();
            }

            // 현재 플레이어가 이미 다른 방파에 소속되어 있는지 확인
            let current_guild = body.get_string("소속");
            if !current_guild.is_empty() {
                return format!("이미 {}에 소속되어 있습니다.", current_guild);
            }

            // 방파 생성 (Guild 모듈 사용)
            crate::world::guild::guild_set(guild_name, "이름", guild_name);
            crate::world::guild::guild_set(guild_name, "방주", &body.get_name());
            crate::world::guild::guild_set(guild_name, "부방주", "");
            crate::world::guild::guild_set(guild_name, "장로", "");
            crate::world::guild::guild_set(guild_name, "제자", "");
            crate::world::guild::guild_set(
                guild_name,
                "설립일",
                &chrono::Utc::now().format("%Y-%m-%d").to_string(),
            );

            // 플레이어의 소속을 새 방파로 설정
            body.set("소속", guild_name.to_string());
            body.set("직위", "방주".to_string());

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            println!(
                "[SCRIPT] guild_create: {} created by {}",
                guild_name,
                body.get_name()
            );

            String::new() // 성공
        },
    );

    // guild_join(ob, guild_name) - 방파 가입
    // Returns "" on success, error string on failure
    let body_ptr_gj = body_ptr;
    engine.register_fn(
        "guild_join",
        move |_ob: &mut rhai::Map, guild_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_gj };

            // 빈 방파 이름 체크
            if guild_name.trim().is_empty() {
                return "방파 이름을 입력해주세요.".to_string();
            }

            // 방파 존재 확인
            if !crate::world::guild::guild_has(guild_name) {
                return "존재하지 않는 방파입니다.".to_string();
            }

            // 이미 다른 방파에 소속되어 있는지 확인
            let current_guild = body.get_string("소속");
            if !current_guild.is_empty() {
                return format!(
                    "이미 {}에 소속되어 있습니다. 탈퇴 후 가입해주세요.",
                    current_guild
                );
            }

            // 소속 설정
            body.set("소속", guild_name.to_string());
            body.set("직위", "제자".to_string()); // 기본 직위: 제자

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            println!(
                "[SCRIPT] guild_join: {} joined {}",
                body.get_name(),
                guild_name
            );

            String::new() // 성공
        },
    );

    // guild_leave(ob) - 방파 탈퇴
    // Returns true on success
    let body_ptr_gl = body_ptr;
    engine.register_fn("guild_leave", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr_gl };

        let current_guild = body.get_string("소속");
        if current_guild.is_empty() {
            return false; // 소속된 방파가 없음
        }

        let my_name = body.get_name();

        // 방주인지 확인 (방주는 탈퇴 불가, 해체만 가능)
        let leader = crate::world::guild::guild_get(&current_guild, "방주");
        if leader == my_name {
            return false; // 방주는 탈퇴 불가
        }

        // 소속 및 직위 제거
        body.set("소속", "".to_string());
        body.set("직위", "".to_string());

        // 저장
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);

        println!("[SCRIPT] guild_leave: {} left {}", my_name, current_guild);

        true
    });

    // guild_get_members(ob) - 방파 멤버 목록 가져오기
    // Returns Array of member names
    let body_ptr_ggm = body_ptr;
    engine.register_fn(
        "guild_get_members",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_ggm };

            let guild_name = body.get_string("소속");
            if guild_name.is_empty() {
                return rhai::Array::new();
            }

            let members = get_guild_members_from_files(&guild_name);
            let mut arr = rhai::Array::new();
            for member in members {
                arr.push(Dynamic::from(member));
            }
            arr
        },
    );

    // guild_get_leader(ob, guild_name) - 방파 방주 이름 가져오기
    // Returns leader name
    let _body_ptr_gglead = body_ptr;
    engine.register_fn(
        "guild_get_leader",
        move |_ob: &mut rhai::Map, guild_name: &str| -> String {
            if guild_name.is_empty() {
                return String::new();
            }

            // Guild 모듈에서 방주 정보 조회
            crate::world::guild::guild_get(guild_name, "방주")
        },
    );

    // guild_promote(ob, member_name, position) - 방파 멤버 승진
    // Returns "" on success, error string on failure
    // Leader only
    let body_ptr_gpr = body_ptr;
    engine.register_fn(
        "guild_promote",
        move |_ob: &mut rhai::Map, member_name: &str, position: &str| -> String {
            let body = unsafe { &*body_ptr_gpr };

            let my_name = body.get_name();
            let my_guild = body.get_string("소속");
            let my_position = body.get_string("직위");

            // 빈 인자 체크
            if member_name.trim().is_empty() || position.trim().is_empty() {
                return "사용법: guild_promote(이름, 직위)".to_string();
            }

            // 방파 소속 확인
            if my_guild.is_empty() {
                return "방파에 소속되어 있지 않습니다.".to_string();
            }

            // 방주만 승진 가능
            if my_position != "방주" {
                return "방주만 멤버의 직위를 변경할 수 있습니다.".to_string();
            }

            // 대상 멤버의 현재 소속 확인
            let member_guild = {
                let path = format!("data/user/{}.json", member_name);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(uso) = json.get("사용자오브젝트").and_then(|v| v.as_object())
                        {
                            if let Some(attr) = uso.get("attr").and_then(|v| v.as_object()) {
                                attr.get("소속")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            };

            if member_guild != my_guild {
                return "해당 플레이어가 같은 방파에 소속되어 있지 않습니다.".to_string();
            }

            // 유효한 직위인지 확인
            let valid_positions = ["방주", "부방주", "장로", "제자"];
            if !valid_positions.contains(&position) {
                return format!(
                    "유효하지 않은 직위입니다. 가능한 직위: {}",
                    valid_positions.join(", ")
                );
            }

            // 방주 직위는 다른 사람에게 넘길 수 없게 (선택적 제한)
            if position == "방주" {
                return "방주 직위는 넘길 수 없습니다. 방파 해체 후 새로 만들어주세요.".to_string();
            }

            // 직위 설정
            if set_guild_member_position(member_name, position) {
                println!(
                    "[SCRIPT] guild_promote: {} promoted to {} by {}",
                    member_name, position, my_name
                );
                String::new()
            } else {
                "직위 변경에 실패했습니다.".to_string()
            }
        },
    );

    // guild_demote(ob, member_name) - 방파 멤버 강등
    // Returns "" on success, error string on failure
    let body_ptr_gdm = body_ptr;
    engine.register_fn(
        "guild_demote",
        move |_ob: &mut rhai::Map, member_name: &str| -> String {
            let body = unsafe { &*body_ptr_gdm };

            let my_name = body.get_name();
            let my_guild = body.get_string("소속");
            let my_position = body.get_string("직위");

            // 빈 인자 체크
            if member_name.trim().is_empty() {
                return "사용법: guild_demote(이름)".to_string();
            }

            // 방파 소속 확인
            if my_guild.is_empty() {
                return "방파에 소속되어 있지 않습니다.".to_string();
            }

            // 방주만 강등 가능
            if my_position != "방주" {
                return "방주만 멤버를 강등할 수 있습니다.".to_string();
            }

            // 대상 멤버의 현재 소속 확인
            let member_guild = {
                let path = format!("data/user/{}.json", member_name);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(uso) = json.get("사용자오브젝트").and_then(|v| v.as_object())
                        {
                            if let Some(attr) = uso.get("attr").and_then(|v| v.as_object()) {
                                attr.get("소속")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            };

            if member_guild != my_guild {
                return "해당 플레이어가 같은 방파에 소속되어 있지 않습니다.".to_string();
            }

            // 현재 직위 확인
            let current_position = get_guild_member_position(member_name);
            if current_position == "방주" {
                return "방주를 강등할 수 없습니다.".to_string();
            }

            // 한 단계 강등 (부방주->장로, 장로->제자, 제자->제자)
            let new_position = match current_position.as_str() {
                "부방주" => "장로",
                "장로" => "제자",
                _ => "제자",
            };

            // 직위 설정
            if set_guild_member_position(member_name, new_position) {
                println!(
                    "[SCRIPT] guild_demote: {} demoted to {} by {}",
                    member_name, new_position, my_name
                );
                String::new()
            } else {
                "강등에 실패했습니다.".to_string()
            }
        },
    );

    // guild_chat(ob, message) - 방파 채팅
    // Already exists as send_broadcast_to_guild, but add alias
    let spec_gchat = spec.clone();
    let _body_ptr_gchat = body_ptr;
    engine.register_fn(
        "guild_chat",
        move |ob: &mut rhai::Map, msg: &str| -> String {
            if msg.trim().is_empty() {
                return "usage".to_string();
            }
            let my_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let guild = ob
                .get("소속")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            if guild.is_empty() {
                return "no_guild".to_string();
            }
            let arr = get_precomputed_all_online();
            let mut names: Vec<String> = Vec::new();
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    let s: String = m
                        .get("소속")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    if s == guild {
                        if let Some(n) = m
                            .get("이름")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        {
                            if !n.is_empty() {
                                names.push(n);
                            }
                        }
                    }
                }
            }
            let formatted = format!("\x1b[0;35m[방파]\x1b[0;37m {} : {}", my_name, msg);
            if let Ok(mut s) = spec_gchat.lock() {
                *s = Some(CommandResult::BroadcastToPlayers(names, formatted));
            }
            "".to_string()
        },
    );

    // guild_get_info(ob) - 방파 정보 가져오기
    // Returns Map with guild data
    let body_ptr_ggi = body_ptr;
    engine.register_fn("guild_get_info", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &*body_ptr_ggi };

        let guild_name = body.get_string("소속");
        if guild_name.is_empty() {
            return Dynamic::UNIT;
        }

        let mut info = rhai::Map::new();
        info.insert("이름".into(), Dynamic::from(guild_name.clone()));

        // Guild 모듈에서 정보 가져오기
        let leader = crate::world::guild::guild_get(&guild_name, "방주");
        let vice_leader = crate::world::guild::guild_get(&guild_name, "부방주");
        let elders = crate::world::guild::guild_get(&guild_name, "장로");
        let disciples = crate::world::guild::guild_get(&guild_name, "제자");
        let founded = crate::world::guild::guild_get(&guild_name, "설립일");

        info.insert("방주".into(), Dynamic::from(leader));
        info.insert("부방주".into(), Dynamic::from(vice_leader));
        info.insert("장로".into(), Dynamic::from(elders));
        info.insert("제자".into(), Dynamic::from(disciples));
        info.insert("설립일".into(), Dynamic::from(founded));

        // 현재 멤버 목록
        let members = get_guild_members_from_files(&guild_name);
        let mut member_arr = rhai::Array::new();
        for member in &members {
            member_arr.push(Dynamic::from(member.clone()));
        }
        info.insert("멤버수".into(), Dynamic::from(members.len() as i64));
        info.insert("멤버목록".into(), Dynamic::from(member_arr));

        Dynamic::from(info)
    });

    // guild_disband(ob) - 방파 해체
    // Returns true on success
    // Leader only
    let body_ptr_gdis = body_ptr;
    engine.register_fn("guild_disband", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr_gdis };

        let my_name = body.get_name();
        let my_guild = body.get_string("소속");
        let my_position = body.get_string("직위");

        // 방파 소속 확인
        if my_guild.is_empty() {
            return false;
        }

        // 방주만 해체 가능
        if my_position != "방주" {
            return false;
        }

        // 모든 멤버의 소속 및 직위 제거
        let members = get_guild_members_from_files(&my_guild);
        for member_name in &members {
            set_guild_member_position(member_name, "");
            // 직접 파일을 수정하여 소속 제거
            let path = format!("data/user/{}.json", member_name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(uso) = json
                        .get_mut("사용자오브젝트")
                        .and_then(|v| v.as_object_mut())
                    {
                        if let Some(attr) = uso.get_mut("attr").and_then(|v| v.as_object_mut()) {
                            attr.insert(
                                "소속".to_string(),
                                serde_json::Value::String("".to_string()),
                            );
                            attr.insert(
                                "직위".to_string(),
                                serde_json::Value::String("".to_string()),
                            );
                            let _ = std::fs::write(
                                &path,
                                serde_json::to_string_pretty(&json).unwrap_or_default(),
                            );
                        }
                    }
                }
            }
        }

        // 방주 본인의 소속도 제거
        body.set("소속", "".to_string());
        body.set("직위", "".to_string());

        // 방파 데이터 제거
        let _ = crate::world::guild::guild_remove(&my_guild);

        // 저장
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);

        println!(
            "[SCRIPT] guild_disband: {} disbanded by {}",
            my_guild, my_name
        );

        true
    });

    // ============================================================
    // GLOBAL DATA ACCESS FUNCTIONS (if global_data provided)
    // ============================================================
    if let Some(gd) = global_data {
        // get_global(file) - 전체 파일 데이터 가져오기
        let gd_clone = gd.clone();
        engine.register_fn("get_global", move |file: &str| -> Dynamic {
            if let Ok(data) = gd_clone.try_read() {
                if let Some(json) = data.get(file) {
                    return crate::data::json_to_dynamic(json);
                }
            }
            Dynamic::UNIT
        });

        // get_global_key(file, key) - 파일에서 특정 키의 데이터 가져오기
        let gd_clone = gd.clone();
        engine.register_fn("get_global_key", move |file: &str, key: &str| -> Dynamic {
            if let Ok(data) = gd_clone.try_read() {
                if let Some(json) = data.get_path(file, key) {
                    return crate::data::json_to_dynamic(json);
                }
            }
            Dynamic::UNIT
        });

        // get_global_keys(file) - 파일의 모든 키 목록
        let gd_clone = gd.clone();
        engine.register_fn("get_global_keys", move |file: &str| -> rhai::Array {
            if let Ok(data) = gd_clone.try_read() {
                let keys: rhai::Array = data.keys(file).into_iter().map(Dynamic::from).collect();
                keys
            } else {
                rhai::Array::new()
            }
        });

        // list_globals() - 모든 파일 이름 목록
        let gd_clone = gd.clone();
        engine.register_fn("list_globals", move || -> rhai::Array {
            if let Ok(data) = gd_clone.try_read() {
                let names: rhai::Array = data.file_names().into_iter().map(Dynamic::from).collect();
                names
            } else {
                rhai::Array::new()
            }
        });

        // has_global(file) - 파일 존재 확인
        let gd_clone = gd.clone();
        engine.register_fn("has_global", move |file: &str| -> bool {
            if let Ok(data) = gd_clone.try_read() {
                data.contains(file)
            } else {
                false
            }
        });

        // has_global_key(file, key) - 파일의 키 존재 확인
        let gd_clone = gd.clone();
        engine.register_fn("has_global_key", move |file: &str, key: &str| -> bool {
            if let Ok(data) = gd_clone.try_read() {
                data.contains_key(file, key)
            } else {
                false
            }
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
    let _gd = global_data.clone();

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
            let keys: rhai::Array = data.keys(file).into_iter().map(Dynamic::from).collect();
            keys
        } else {
            rhai::Array::new()
        }
    });

    // list_globals() - 모든 파일 이름 목록
    let gd_clone = global_data.clone();
    engine.register_fn("list_globals", move || -> rhai::Array {
        if let Ok(data) = gd_clone.try_read() {
            let names: rhai::Array = data.file_names().into_iter().map(Dynamic::from).collect();
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
    /// Library scripts loaded from lib/ directory (hot-reloadable)
    libraries: HashMap<String, String>,
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
            libraries: HashMap::new(),
            config,
            global_data: None,
        };
        storage.load_all_libraries().ok();
        storage.load_all_scripts().ok();
        storage
    }

    /// 글로벌 데이터 캐시와 함께 생성합니다.
    pub fn with_global_data(config: ScriptConfig, global_data: SharedGlobalData) -> Self {
        let mut storage = Self {
            scripts: HashMap::new(),
            libraries: HashMap::new(),
            config,
            global_data: Some(global_data),
        };
        storage.load_all_libraries().ok();
        storage.load_all_scripts().ok();
        storage
    }

    #[allow(clippy::should_implement_trait)]
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
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                self.load_script(&name, &path)?;
            }
        }

        info!("Loaded {} scripts from {:?}", self.scripts.len(), dir);
        Ok(())
    }

    /// Load all library scripts from lib/ directory (recursively) for hot-reloadable functions
    pub fn load_all_libraries(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = self.config.lib_dir.clone();
        if !dir.exists() {
            info!("Library directory does not exist: {:?}", dir);
            return Ok(());
        }

        self.load_libraries_recursive(&dir)?;

        info!(
            "Loaded {} library scripts from {:?}",
            self.libraries.len(),
            dir
        );
        Ok(())
    }

    /// Recursively load .rhai files from a directory
    fn load_libraries_recursive(&mut self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip lib/std/ and lib/doumi/ directories
                // lib/std/ files define object templates with duplicate function names
                // lib/doumi/ files are DOUMI character creation scripts, not libraries
                if let Some(file_name) = path.file_name() {
                    if file_name == "std" || file_name == "doumi" {
                        continue;
                    }
                }
                // Recursively load from subdirectories
                self.load_libraries_recursive(&path)?;
            } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rhai") {
                // Create a unique name based on relative path from lib_dir
                let rel_path = path
                    .strip_prefix(&self.config.lib_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/");

                // Skip std/ and doumi/ directory files
                if rel_path.starts_with("std/") || rel_path.starts_with("doumi/") {
                    continue;
                }

                // Remove .rhai extension from the relative path to get a unique library name
                let name = rel_path
                    .strip_suffix(".rhai")
                    .unwrap_or(&rel_path)
                    .to_string();

                let source = std::fs::read_to_string(&path)?;
                debug!("Loaded library: {} from {:?}", name, path);
                self.libraries.insert(name, source);
            }
        }
        Ok(())
    }

    /// Reload all library scripts from lib/ directory
    pub fn reload_libraries(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        self.libraries.clear();
        self.load_all_libraries()?;
        Ok(self.libraries.len())
    }

    /// Get combined library source code to prepend to scripts
    pub fn get_library_source(&self) -> String {
        let mut combined = String::new();
        for (name, source) in &self.libraries {
            combined.push_str("// Library: ");
            combined.push_str(name);
            combined.push('\n');
            combined.push_str(source);
            combined.push('\n');
        }
        combined
    }

    pub fn load_script(
        &mut self,
        name: &str,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;
        let source = std::fs::read_to_string(path)?;
        self.scripts.insert(
            name.to_string(),
            StoredScript {
                source,
                modified,
                _name: name.to_string(),
            },
        );
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
        self.scripts.insert(
            name.to_string(),
            StoredScript {
                source,
                modified,
                _name: name.to_string(),
            },
        );

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
        println!("[DEBUG] Executing script: '{}'", name);
        println!(
            "[DEBUG] Script source length: {}",
            self.scripts.get(name).map(|s| s.source.len()).unwrap_or(0)
        );
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("Script not found: {}", name))?;

        let output_collector = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output_collector.clone();
        let special_collector = Arc::new(Mutex::new(None));
        let user_sends = Arc::new(Mutex::new(Vec::new()));

        let engine = create_engine_with_body_and_output(
            player,
            output_clone,
            get_other_players_desc,
            get_other_players_map,
            special_collector.clone(),
            user_sends.clone(),
            call_out_scheduler,
            Some(name),
            self.global_data.clone(),
        );
        let mut scope = Scope::new();

        let player_data = build_ob_from_body(player);
        scope.push("player", player_data.clone());
        scope.push("me", player_data.clone());
        scope.push("ob", player_data.clone());
        scope.push("this", player_data); // For std library functions that use 'this'
        scope.push("cmdline", rhai::Dynamic::from(line.to_string()));

        // DOUMI system global variables for script suspension/resumption
        scope.push("_doumi_resume_op", "" as &str);
        scope.push("_doumi_resume_input", "" as &str);

        // Prepend library source for hot-reloadable functions
        let library_source = self.get_library_source();
        let script_with_main = format!("{}\n{}\nmain(ob, cmdline)", library_source, script.source);
        println!(
            "[DEBUG] About to run script with_main, length={}",
            script_with_main.len()
        );
        // Print first 20 lines for debugging
        let lines: Vec<&str> = script_with_main.lines().take(400).collect();
        for (i, line) in lines.iter().enumerate() {
            println!("[DEBUG] Line {}: {:?}", i + 1, line);
        }
        let result = engine.run_with_scope(&mut scope, &script_with_main);
        println!("[DEBUG] Script run result: {:?}", result);
        result.map_err(|e| format!("스크립트 실행 오류: {}", e))?;

        let outputs = output_collector.lock().unwrap().clone();
        println!("[DEBUG] Collected {} outputs", outputs.len());
        let expanded: Vec<String> = outputs
            .into_iter()
            .map(|s| expand_abbreviated_ansi(&s))
            .collect();
        let mut special = special_collector.lock().unwrap().take();
        let to_send = user_sends.lock().unwrap().drain(..).collect::<Vec<_>>();
        if special.is_none() && !to_send.is_empty() {
            special = Some(CommandResult::SendToUsers(to_send));
        }
        Ok((expanded, special))
    }

    pub fn execute_with_scope(
        &self,
        name: &str,
        scope: &mut Scope,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let script = self
            .scripts
            .get(name)
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
    m.insert("설정상태".into(), body.get_string("설정상태").into());
    m.insert(
        "운기조식".into(),
        (body.act == crate::player::ActState::Rest).into(),
    );
    m.insert("env".into(), "".into());
    m.insert("objs".into(), rhai::Dynamic::from(rhai::Array::new()));
    // 숙련도.rhai: 검/도/창/기타/맨손
    m.insert("1 숙련도".into(), body.get_int("1 숙련도").into());
    m.insert("2 숙련도".into(), body.get_int("2 숙련도").into());
    m.insert("3 숙련도".into(), body.get_int("3 숙련도").into());
    m.insert("4 숙련도".into(), body.get_int("4 숙련도").into());
    m.insert("5 숙련도".into(), body.get_int("5 숙련도").into());

    // Korean attribute keys that scripts access via get_int()
    // These are required by 능력치.rhai and other scripts
    m.insert("체력".into(), body.get_hp().into());
    m.insert("최고체력".into(), body.get_int("최고체력").into());
    m.insert("내공".into(), body.get_mp().into());
    m.insert("최고내공".into(), body.get_max_mp().into());
    m.insert("힘".into(), body.get_int("힘").into());
    m.insert("민첩성".into(), body.get_int("민첩성").into());
    m.insert("명중".into(), body.get_int("명중").into());
    m.insert("회피".into(), body.get_int("회피").into());
    m.insert("필살".into(), body.get_int("필살").into());
    m.insert("운".into(), body.get_int("운").into());
    m.insert("배우자".into(), body.get_string("배우자").into());
    m.insert("직위".into(), body.get_string("직위").into());
    m.insert("성별".into(), body.get_string("성별").into());
    m.insert("목표경험치".into(), body.get_int("목표경험치").into());
    m.insert("분노".into(), body.get_int("분노").into());
    m.insert("소지품무게".into(), body.get_int("소지품무게").into());
    m.insert("특성치".into(), body.get_int("특성치").into());
    m
}

/// call_out 만료 시 Rhai 스크립트 함수를 실행하는 runner 생성.
/// (target, script, function, args) -> Result. process_due에서 호출.
pub fn create_call_out_script_runner(
    script_storage: Arc<tokio::sync::RwLock<ScriptStorage>>,
    broadcaster: Arc<Broadcaster>,
) -> Arc<dyn Fn(&str, Option<&str>, &str, Vec<serde_json::Value>) -> Result<(), String> + Send + Sync>
{
    Arc::new(
        move |target: &str, script: Option<&str>, function: &str, _args: Vec<serde_json::Value>| {
            let script = script.ok_or_else(|| "call_out: script name required".to_string())?;
            // process_due는 tokio 워커에서 호출되므로 blocking_read 전에 block_in_place로 블로킹 허용
            let (source, global_data) = tokio::task::block_in_place(|| {
                let storage = script_storage.blocking_read();
                (storage.get_script_source(script), storage.global_data.clone())
            });
            let source = source.ok_or_else(|| format!("script not found: {}", script))?;

            // 클로저 안에서는 clients 락이 잡혀 있으므로 send_to_by_player_name(→clients.lock()) 호출 금지.
            // 메시지만 수집하고, 락 해제 후 밖에서 전송.
            let to_send = broadcaster
                .with_player_body_by_name(target, |body| {
                    let output_collector = Arc::new(Mutex::new(Vec::new()));
                    let special_collector = Arc::new(Mutex::new(None));
                    let user_sends = Arc::new(Mutex::new(Vec::new()));
                    let engine = create_engine_with_body_and_output(
                        body,
                        output_collector.clone(),
                        None,
                        None,
                        special_collector,
                        user_sends,
                        None,
                        None,
                        global_data.clone(),
                    );
                    let ast = engine
                        .compile(&source)
                        .map_err(|e| format!("compile: {}", e))?;
                    let mut scope = Scope::new();
                    let ob = Dynamic::from(build_ob_from_body(body));
                    let _ = engine
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
        },
    )
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

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(ScriptConfig::default())
    }

    pub async fn execute(
        &self,
        name: &str,
        player: &mut Body,
        line: &str,
        get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
        get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
        call_out_scheduler: Option<Arc<CallOutScheduler>>,
    ) -> Result<(Vec<String>, Option<CommandResult>), Box<dyn std::error::Error>> {
        let storage = self.inner.read().await;
        storage.execute(
            name,
            player,
            line,
            get_other_players_desc,
            get_other_players_map,
            call_out_scheduler,
        )
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
            w.set_player_position(
                "item_test_player",
                PlayerPosition::new("낙양성".to_string(), "1".to_string()),
            );
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
        assert_eq!(
            body.object.objs.len(),
            1,
            "생성 후 인벤 1개 (outputs: {:?})",
            out
        );
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "철퇴");

        // 2) 버리기 철퇴
        let res = storage.execute("버려", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "버리기 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "버린 후 인벤 비어있음");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", "1");
            assert_eq!(ro.len(), 1, "방 바닥에 1개");
            assert_eq!(ro[0].lock().unwrap().getName(), "철퇴");
        }

        // 3) 가져오기 철퇴
        let res = storage.execute("가져", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "가져오기 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 1, "가져온 후 인벤 1개");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", "1");
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
