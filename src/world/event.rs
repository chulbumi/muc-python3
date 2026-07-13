//! NPC 이벤트 스크립트 처리 (파이썬 doEvent/checkEvent 대응)
//!
//! "[대상] [명령] [인자]" (예: 왕대협 대화) 입력 시, 같은 방 NPC의 이벤트 키와 매칭되어
//! $이벤트확인, $출력, $위치이동 등 $함수 스크립트를 실행합니다.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use log::info;
use rhai::{Dynamic, Engine, EvalAltResult, Map, Position, Scope};

use crate::command::CommandResult;
use crate::hangul::han_iga;
use crate::object::{Object, Value};
use crate::player::Body;
use crate::script::{
    format_event_string, load_script_file, object_from_item_json, parse_event_string,
    save_body_to_json,
};
use crate::world::{get_world_state, EventScript, RawMobData};

/// getNextWords: 첫 토큰 제외한 나머지. "이벤트 $대화 $대" -> "$대화 $대"
fn get_next_words(key: &str) -> String {
    let mut it = key.splitn(2, |c: char| c.is_whitespace());
    it.next();
    it.next().unwrap_or("").trim().to_string()
}

/// getStrCnt: "$아이템주기 합성11 2" -> (합성11, 2). l==3: (tok[1], getInt(tok[2])); l>3: (tok[1], getInt(tok[-1])); else (tok[1], 1).
fn get_str_cnt(line: &str) -> (String, i64) {
    let tok: Vec<&str> = line.split_whitespace().collect();
    let l = tok.len();
    let cnt = if l >= 3 { parse_int(tok[l - 1]) } else { 1 };
    let index = tok.get(1).map(|s| (*s).to_string()).unwrap_or_default();
    (index, cnt)
}

fn parse_int(s: &str) -> i64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    if let Ok(n) = s.parse::<i64>() {
        return n;
    }
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<i64>().unwrap_or(0)
}

/// 몹 이벤트 키 목록에서 words(사용자 입력 토큰)와 매칭되는 키를 찾습니다.
/// last가 cmd_list에 있고, issue_list가 있으면 prev도 일치해야 함.
/// 여러 키가 매칭되면, (cmd_list+issue_list) 중 words에 등장하는 수가 가장 많은 쪽(가장 구체적)을 반환.
pub fn check_event_key(data: &RawMobData, words: &[&str]) -> Option<String> {
    if words.is_empty() {
        return None;
    }
    let last = words[words.len() - 1];

    let mut best: Option<(String, usize)> = None;

    for key in data.events.keys() {
        if !key.starts_with("이벤트") {
            continue;
        }
        let nw = get_next_words(key);
        let keywords: Vec<&str> = nw.split_whitespace().collect();

        let mut cmd_list: Vec<&str> = Vec::new();
        let mut issue_list: Vec<&str> = Vec::new();
        for kw in keywords {
            if kw.starts_with('$') {
                cmd_list.push(kw.get(1..).unwrap_or(kw));
            } else {
                issue_list.push(kw);
            }
        }

        if !cmd_list.contains(&last) {
            continue;
        }

        if !issue_list.is_empty() {
            if words.len() <= 2 {
                continue;
            }
            let prev = words[words.len() - 2];
            if !issue_list.contains(&prev) {
                continue;
            }
        }

        let all: Vec<&str> = cmd_list.iter().chain(issue_list.iter()).copied().collect();
        let score = all.iter().filter(|k| words.contains(*k)).count();
        if best.as_ref().is_none_or(|(_, s)| *s < score) {
            best = Some((key.clone(), score));
        }
    }

    best.map(|(k, _)| k)
}

/// 플레이어의 이벤트설정리스트에 키가 있는지. 파이썬 checkEvent(e).
fn get_user_event(body: &Body, key: &str) -> String {
    parse_event_string(&body.get_string("이벤트설정리스트"))
        .get(key)
        .cloned()
        .unwrap_or_default()
}

fn set_user_event(body: &mut Body, key: &str, value: &str) {
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
}

fn del_user_event(body: &mut Body, key: &str) {
    let mut m = parse_event_string(&body.get_string("이벤트설정리스트"));
    m.remove(key);
    body.object.attr.insert(
        "이벤트설정리스트".to_string(),
        Value::String(format_event_string(&m)),
    );
    let path = format!("data/user/{}.json", body.get_name());
    let _ = save_body_to_json(body, &path);
}

/// Python Player.getTendency(). 완성=무림별호 있음, 정파/사파=성격별 PK 수치 판정.
fn get_tendency(body: &Body, t: &str) -> bool {
    let t = t.trim();
    if t.is_empty() {
        return false;
    }
    let neutral = body.get_int("0 성격플킬");
    let righteous = body.get_int("1 성격플킬");
    let evil = body.get_int("2 성격플킬");
    let enough_kills = neutral.saturating_add(righteous).saturating_add(evil)
        >= crate::script::get_murim_config_int("무림별호이벤트킬수");
    match t {
        "완성" => !body.get_string("무림별호").is_empty(),
        // Python: 정파는 사파 성향(p3)이 정파 성향(p2)보다 많으면 실패한다.
        "정파" => enough_kills && evil <= righteous,
        // Python: 사파는 정파 성향(p2)이 사파 성향(p3)보다 많으면 실패한다.
        "사파" => enough_kills && righteous <= evil,
        _ => true,
    }
}

/// $출력 / 일반 줄: [공], [사용자이름], [공](이/가) 치환.
fn substitute_line(line: &str, player_name: &str) -> String {
    let r = line.replace("[사용자이름]", player_name).replace(
        "[공](이/가)",
        &format!("{}{}", player_name, han_iga(player_name)),
    );
    r.replace("[공]", player_name)
}

/// Rhai 이벤트용 output efun: [공], [사용자이름], [공](이/가) 치환.
fn event_substitute(line: &str, player_name: &str) -> String {
    substitute_line(line, player_name)
}

/// room_spec "존:방" 파싱.
fn parse_room_spec(s: &str) -> (String, String) {
    let s = s.trim();
    if let Some(i) = s.find(':') {
        (s[..i].to_string(), s[i + 1..].trim().to_string())
    } else {
        (String::new(), s.to_string())
    }
}

/// doEvent 스타일 $함수 처리. 스크립트 한 줄씩 실행.
/// mob_key: $엔터$ 재개 시 사용. start_from_line: Legacy 재개 시 1-based 이하 스킵.
/// resume_for_rhai: Rhai wait_enter 재개 시 Some("step1") 등. 이벤트 값이 문자열이면 do_event_rhai로 위임.
pub fn do_event(
    body: &mut Body,
    data: &RawMobData,
    event_key: &str,
    words: &[String],
    mob_key: &str,
    start_from_line: Option<usize>,
    resume_for_rhai: Option<String>,
) -> CommandResult {
    let script = match data.events.get(event_key) {
        None => return CommandResult::Output("(이벤트 스크립트 없음)".to_string()),
        Some(EventScript::Rhai(path)) => {
            return do_event_rhai(body, data, event_key, words, mob_key, path, resume_for_rhai);
        }
        Some(EventScript::Legacy(lines)) => lines,
    };

    let player_name = body.get_name();
    let words_ref: Vec<&str> = words.iter().map(|s| s.as_str()).collect();

    let mut output: Vec<String> = Vec::new();
    let mut set_position: Option<(String, String)> = None;
    let mut search_end = false;
    let mut tab = 0i32;

    for (idx, line) in script.iter().enumerate() {
        let line_num = idx + 1;
        if start_from_line.map(|s| line_num <= s).unwrap_or(false) {
            continue;
        }
        let sline = line.trim().to_string();
        if sline.is_empty() {
            if !search_end {
                output.push(String::new());
            }
            continue;
        }

        if search_end {
            if sline.starts_with('{') {
                tab += 1;
                continue;
            }
            if !sline.starts_with('}') {
                continue;
            }
            tab -= 1;
            if tab != 0 {
                continue;
            }
            search_end = false;
            tab = 0;
            continue;
        }

        if sline.starts_with("$종료") {
            break;
        }

        let sline = sline.replace("[사용자이름]", &player_name);

        if sline.starts_with('$') {
            let mut sline_mut = sline.clone();
            if words.len() > 2 {
                sline_mut =
                    sline_mut.replace("$변수:1", words.get(1).map(|s| s.as_str()).unwrap_or(""));
            }

            let func = sline_mut.split_whitespace().next().unwrap_or("");
            let next_words = get_next_words(&sline_mut);

            match func {
                "$엔터$" => {
                    return CommandResult::MobEventEnter {
                        output_lines: output,
                        set_position: set_position.clone(),
                        broadcast_lines: Vec::new(),
                        mob_key: mob_key.to_string(),
                        event_key: event_key.to_string(),
                        words: words.to_vec(),
                        line_num,
                        prompt: next_words,
                        resume_func: None,
                    };
                }
                "$이벤트확인!" => {
                    if !get_user_event(body, &next_words).is_empty() {
                        search_end = true;
                    }
                }
                "$이벤트확인" => {
                    if get_user_event(body, &next_words).is_empty() {
                        search_end = true;
                    }
                }
                "$이벤트설정" => {
                    let v: Vec<&str> = next_words.split_whitespace().collect();
                    if v.len() >= 2 {
                        set_user_event(body, v[0], v[1]);
                    } else if !v.is_empty() {
                        set_user_event(body, v[0], "1");
                    }
                }
                "$이벤트삭제" => {
                    if !next_words.is_empty() {
                        del_user_event(body, &next_words);
                    }
                }
                "$위치이동" => {
                    if next_words.is_empty() {
                        continue;
                    }
                    let (zone, room) = parse_room_spec(&next_words);
                    if zone.is_empty() || room.is_empty() {
                        continue;
                    }
                    output.push(String::new());
                    set_position = Some((zone, room));
                    break;
                }
                "$출력" => {
                    let buf = substitute_line(&next_words, &player_name);
                    output.push(buf);
                }
                "$무림별호조건" => {
                    if !get_tendency(body, &next_words) {
                        search_end = true;
                    }
                }
                "$변수확인" => {
                    let v: Vec<&str> = sline_mut.split_whitespace().collect();
                    if v.len() < 3 {
                        search_end = true;
                        continue;
                    }
                    let c = v[1].parse::<usize>().unwrap_or(0);
                    if words_ref.len() < 2 + c {
                        search_end = true;
                        continue;
                    }
                    if words_ref.get(c + 1).copied() != v.get(2).copied() {
                        search_end = true;
                    }
                }
                "$아이템주기" => {
                    let (index, cnt) = get_str_cnt(&sline_mut);
                    if index.is_empty() {
                        continue;
                    }
                    if index == "은전" {
                        let v = body.get_int("은전") + cnt;
                        body.set("은전", v);
                        continue;
                    }
                    if index == "금전" {
                        let v = body.get_int("금전") + cnt;
                        body.set("금전", v);
                        continue;
                    }
                    for _ in 0..cnt {
                        if let Some((arc, _)) = object_from_item_json(&index) {
                            // Python Body.addItem uses insert(), so every
                            // granted item becomes the first inventory object.
                            body.object.objs.insert(0, arc);
                        }
                    }
                }
                "$스크립트호출" => {
                    if next_words.is_empty() {
                        continue;
                    }
                    let rhai_path = Path::new("data/script").join(format!("{}.rhai", next_words));
                    if rhai_path.exists() {
                        return CommandResult::StartScript {
                            script_name: next_words,
                            lines: vec![],
                            use_rhai: true,
                        };
                    }
                    if let Some(lines) = load_script_file(&next_words) {
                        return CommandResult::StartScript {
                            script_name: next_words,
                            lines,
                            use_rhai: false,
                        };
                    }
                }
                _ => {}
            }
        } else if !sline.starts_with('{') && !sline.starts_with('}') {
            let buf = substitute_line(&sline, &player_name);
            output.push(buf);
        }
    }

    if let Some((z, r)) = set_position {
        CommandResult::MobEvent {
            output_lines: output,
            set_position: Some((z, r)),
            broadcast_lines: Vec::new(),
        }
    } else {
        CommandResult::MobEvent {
            output_lines: output,
            set_position: None,
            broadcast_lines: Vec::new(),
        }
    }
}

/// Rhai 이벤트 스크립트 실행. data/script/{존이름}/{path}.rhai.
/// resume_func: wait_enter 재개 시 Some("step1") 등. None이면 event() 호출, 없으면 top-level만 실행 후 MobEvent.
pub fn do_event_rhai(
    body: &mut Body,
    data: &RawMobData,
    event_key: &str,
    words: &[String],
    mob_key: &str,
    path: &str,
    resume_func: Option<String>,
) -> CommandResult {
    let player_name = body.get_name().to_string();
    let words_vec = words.to_vec();
    let path_trim = path.trim();
    let with_ext = if path_trim.ends_with(".rhai") {
        path_trim.to_string()
    } else {
        format!("{}.rhai", path_trim)
    };
    let script_path = Path::new("data/script")
        .join(data.zone.as_str())
        .join(&with_ext);
    let src = match std::fs::read_to_string(&script_path) {
        Ok(s) => s,
        Err(_) => return CommandResult::Output("(이벤트 스크립트 파일 없음)".to_string()),
    };

    let mut out_lines: Vec<String> = Vec::new();
    let mut out_broadcast_lines: Vec<String> = Vec::new();
    let mut out_set_position: Option<(String, String)> = None;
    let out_ptr = &mut out_lines as *mut Vec<String>;
    let broadcast_ptr = &mut out_broadcast_lines as *mut Vec<String>;
    let pos_ptr = &mut out_set_position as *mut Option<(String, String)>;
    let body_ptr = body as *mut Body;
    let player_name_out = player_name.clone();

    let mut engine = Engine::new();

    // end_event()는 Rhai에서 throw로 종료. 사용자 스크립트와 같은 컴파일 단위에 넣어야 call_fn 시 노출됨.
    const END_EVENT_PREAMBLE: &str = r#"fn end_event() { throw #{ type: "event_complete" }; }"#;
    let src_with_preamble = format!("{}\n\n{}", END_EVENT_PREAMBLE, src);

    engine.register_fn("output", move |msg: &str| {
        let line = event_substitute(msg, &player_name_out);
        unsafe {
            (*out_ptr).push(line);
        }
    });
    let player_name_broadcast = player_name.clone();
    engine.register_fn("broadcast_output", move |msg: &str| {
        let line = event_substitute(msg, &player_name_broadcast);
        unsafe {
            (*broadcast_ptr).push(line);
        }
    });
    let player_name_for_rank = player_name.clone();
    engine.register_fn("event_player_name", move || player_name_for_rank.clone());
    engine.register_fn("event_to_int", |value: &str| {
        value.parse::<i64>().unwrap_or(0)
    });
    engine.register_fn("get_stat", move |key: &str| -> i64 {
        let b = unsafe { &*body_ptr };
        b.get_int(key)
    });
    engine.register_fn("rank_write", crate::world::rank::rank_write);
    engine.register_fn("rank_read", crate::world::rank::rank_read);
    engine.register_fn("rank_get_num", |ty: &str, position: i64| {
        crate::world::rank::rank_get_num(ty, position).unwrap_or_default()
    });
    engine.register_fn("rank_get_all", crate::world::rank::rank_get_all);
    engine.register_fn("set_position", move |zone: &str, room: &str| unsafe {
        *pos_ptr = Some((zone.to_string(), room.to_string()));
    });
    engine.register_fn("check_event", move |key: &str| -> bool {
        let b = unsafe { &*body_ptr };
        !get_user_event(b, key).is_empty()
    });
    engine.register_fn("set_event", move |key: &str, val: &str| {
        let b = unsafe { &mut *body_ptr };
        if val.is_empty() || val == "1" {
            set_user_event(b, key, "1");
        } else {
            // Python setEvent stores one complete list element. Legacy
            // `$이벤트설정 오소리가죽 이벤트` was mechanically migrated
            // to two arguments, but the observable key remains the joined
            // string `오소리가죽 이벤트`.
            set_user_event(b, &format!("{key} {val}"), "1");
        }
    });
    engine.register_fn("del_event", move |key: &str| {
        let b = unsafe { &mut *body_ptr };
        del_user_event(b, key);
    });
    engine.register_fn("delete_item", move |index: &str, cnt: i64| {
        let b = unsafe { &mut *body_ptr };
        del_item_from_body(b, index, cnt);
    });
    engine.register_fn("give_item", move |index: &str, cnt: i64| {
        let b = unsafe { &mut *body_ptr };
        if index == "은전" {
            b.set("은전", b.get_int("은전") + cnt);
            return;
        }
        if index == "금전" {
            b.set("금전", b.get_int("금전") + cnt);
            return;
        }
        for _ in 0..cnt {
            if let Some((arc, _)) = object_from_item_json(index) {
                let is_one_item = arc
                    .lock()
                    .is_ok_and(|item| item.checkAttr("아이템속성", "단일아이템"));
                b.object.objs.insert(0, arc);
                // Python `$아이템주기` claims a 단일아이템 as it is handed
                // over.  Without this, later event branches observe it as
                // still unclaimed and repeatedly award the genuine item.
                if is_one_item {
                    let _ = crate::oneitem::oneitem_have(index, &b.get_name());
                }
            }
        }
    });
    engine.register_fn("get_tendency", move |t: &str| -> bool {
        let b = unsafe { &*body_ptr };
        get_tendency(b, t)
    });
    engine.register_fn("has_item", move |index: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_has_item_spec(b, index)
    });
    // Python `$무공확인 이름`: 이벤트 분기에서 습득한 무공 여부를 확인한다.
    // 아이템 조건과 달리 `skill_list`가 저장 순서와 중복 없는 실제 무공 목록이다.
    engine.register_fn("has_skill", move |name: &str| -> bool {
        let b = unsafe { &*body_ptr };
        b.skill_list.iter().any(|skill| skill == name)
    });
    // Python `$비전종류확인!` checks the exact 비전이름 array, while
    // `$무공개수확인` and `$무공전수` use the ordinary skill list.
    engine.register_fn("has_vision", move |name: &str| -> bool {
        let b = unsafe { &*body_ptr };
        b.has_secret_skill(name)
    });
    engine.register_fn("skill_count", move || -> i64 {
        let b = unsafe { &*body_ptr };
        b.skill_list.len() as i64
    });
    engine.register_fn("teach_skill", move |name: &str| {
        let b = unsafe { &mut *body_ptr };
        if !b.skill_list.iter().any(|skill| skill == name) {
            b.skill_list.push(name.to_string());
            b.skill_map
                .insert(name.to_string(), crate::player::SkillTraining::new(1, 0));
            b.sync_skill_state_to_attrs();
        }
    });
    engine.register_fn("one_item_exists", move |index: &str| -> bool {
        !crate::oneitem::oneitem_get(index).is_empty()
    });
    engine.register_fn("one_item_owner", move |index: &str| -> String {
        let owner = crate::oneitem::oneitem_get(index);
        let owner = owner.split_whitespace().next().unwrap_or_default();
        if owner.is_empty() {
            String::new()
        } else {
            format!("{}{}", owner, han_iga(owner))
        }
    });
    engine.register_fn("one_item_exists_name", move |name: &str| -> bool {
        let index = crate::oneitem::oneitem_get_index_by_name(name);
        !index.is_empty() && !crate::oneitem::oneitem_get(&index).is_empty()
    });
    engine.register_fn("selected_mob_is_corpse", move || -> bool {
        let b = unsafe { &*body_ptr };
        matches!(
            b.temp().get("_event_selected_mob_corpse"),
            Some(Value::Int(1))
        )
    });
    engine.register_fn("set_selected_mob_regen", move || {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut()
            .insert("_event_selected_mob_set_regen".to_string(), Value::Int(1));
    });
    engine.register_fn("start_selected_mob_combat", move || {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut().insert(
            "_event_selected_mob_start_combat".to_string(),
            Value::Int(1),
        );
    });
    engine.register_fn("set_selected_mob_corpse", move || {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut()
            .insert("_event_selected_mob_set_corpse".to_string(), Value::Int(1));
    });
    engine.register_fn("change_stat", move |key: &str, amount: i64| {
        let b = unsafe { &mut *body_ptr };
        b.set(key, b.get_int(key).saturating_add(amount));
    });
    engine.register_fn("consume_hp", move |amount: i64| {
        let b = unsafe { &mut *body_ptr };
        b.set("체력", b.get_int("체력").saturating_sub(amount));
    });
    engine.register_fn("tendency_switch", move || {
        let b = unsafe { &mut *body_ptr };
        let g = b.get_string("성격");
        if g == "사파" {
            b.set("성격", "정파");
        } else if g == "정파" {
            b.set("성격", "사파");
        }
    });
    engine.register_fn("set_giin", move || {
        let b = unsafe { &mut *body_ptr };
        let p1 = (b.get_int("힘") - 600).max(15);
        b.set("힘", p1);
        let r = b.get_int("전직");
        let mapgip = if r > 0 {
            b.get_int("맷집") * 2 / 3
        } else {
            15
        };
        b.set("맷집", mapgip);
        b.set("레벨", 1);
        b.set("현재경험치", 0);
        b.set("힘경험치", 0);
        b.set("맷집경험치", 0);
        b.set("기존성격", b.get_string("성격"));
        b.set("성격", "기인");
        b.set("내공증진아이템리스트", "");
        set_user_event(b, "소오강호끝", "1");
        let path = format!("data/user/{}.json", b.get_name());
        let _ = save_body_to_json(b, &path);
    });
    engine.register_fn("set_sunin", move || {
        let b = unsafe { &mut *body_ptr };
        b.set("기존성격", b.get_string("성격"));
        b.set("성격", "선인");
        b.set("내공증진아이템리스트", "");
        // Python Player.setSunIn() replaces, rather than appends to, the
        // event list at ascension time.
        b.set("이벤트설정리스트", "우화등선끝");
        let path = format!("data/user/{}.json", b.get_name());
        let _ = save_body_to_json(b, &path);
    });
    engine.register_fn("words", move |i: i64| -> String {
        words_vec.get(i as usize).cloned().unwrap_or_default()
    });
    engine.register_fn(
        "wait_enter",
        move |next_func: &str, prompt: &str| -> Result<(), Box<EvalAltResult>> {
            let mut m = Map::new();
            m.insert("type".into(), Dynamic::from("event_enter"));
            m.insert("next_func".into(), Dynamic::from(next_func.to_string()));
            m.insert("prompt".into(), Dynamic::from(prompt.to_string()));
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(m),
                Position::default(),
            )))
        },
    );

    let ast = match engine.compile(&src_with_preamble) {
        Ok(a) => a,
        Err(e) => return CommandResult::Output(format!("(이벤트 스크립트 컴파일 오류: {})", e)),
    };
    let mut scope = Scope::new();
    let entry = resume_func.clone().unwrap_or_else(|| "event".to_string());
    let r = engine.call_fn::<Dynamic>(&mut scope, &ast, &entry, ());

    match r {
        Ok(_) => CommandResult::MobEvent {
            output_lines: out_lines,
            set_position: out_set_position,
            broadcast_lines: out_broadcast_lines,
        },
        Err(e) => {
            // end_event()의 throw는 ErrorInFunctionCall로 감싸져 올 수 있음. 안쪽 ErrorRuntime까지 풀어서 확인.
            let mut err: &EvalAltResult = &e;
            while let EvalAltResult::ErrorInFunctionCall(_, _, inner, _) = err {
                err = inner.as_ref();
            }
            if let EvalAltResult::ErrorRuntime(d, _) = err {
                if let Some(m) = d.clone().try_cast::<Map>() {
                    let t: String = m
                        .get("type")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    if t == "event_enter" {
                        let next_func: String = m
                            .get("next_func")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                            .unwrap_or_default();
                        let prompt: String = m
                            .get("prompt")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                            .unwrap_or_default();
                        return CommandResult::MobEventEnter {
                            output_lines: out_lines,
                            set_position: out_set_position,
                            broadcast_lines: out_broadcast_lines,
                            mob_key: mob_key.to_string(),
                            event_key: event_key.to_string(),
                            words: words.to_vec(),
                            line_num: 0,
                            prompt,
                            resume_func: Some(next_func),
                        };
                    }
                    if t == "event_complete" {
                        return CommandResult::MobEvent {
                            output_lines: out_lines,
                            set_position: out_set_position,
                            broadcast_lines: out_broadcast_lines,
                        };
                    }
                }
            }
            if let EvalAltResult::ErrorFunctionNotFound(name, _) = err {
                if name == "event" && resume_func.is_none() {
                    return CommandResult::MobEvent {
                        output_lines: out_lines,
                        set_position: out_set_position,
                        broadcast_lines: out_broadcast_lines,
                    };
                }
            }
            CommandResult::Output(format!("(이벤트 스크립트 오류: {})", e))
        }
    }
}

/// [대상] [명령] [인자] 로 해석되어, 같은 방 NPC 이벤트와 매칭되면 do_event 실행 후 Some(CommandResult).
/// 대상은 정확히 일치하거나, 이름/반응이름의 접두사면 매칭(예: "왕", "왕대" → "왕대협").
/// 여럿 매칭 시 이름이 가장 긴 것(가장 구체적)을 우선.
pub fn try_mob_event(
    body: &mut Body,
    zone: &str,
    room: &str,
    raw_line: &str,
) -> Option<CommandResult> {
    let words: Vec<String> = raw_line.split_whitespace().map(|s| s.to_string()).collect();
    if words.len() < 2 {
        return None;
    }

    let raw_name = words[0].as_str();
    let corpse_number = raw_name
        .strip_suffix("시체")
        .and_then(|prefix| prefix.parse::<usize>().ok())
        .filter(|number| *number > 0);
    let name = if corpse_number.is_some() {
        "시체"
    } else {
        raw_name
    };
    let mut candidates = {
        let world = get_world_state().read().ok()?;
        let mut candidates: Vec<(u64, String, String, bool, RawMobData)> = Vec::new();
        for inst in world.mob_cache.get_all_mobs_in_room(zone, room) {
            let data = match world.mob_cache.get_instance_data(inst) {
                Some(data) => data,
                None => continue,
            };
            let corpse = !inst.alive || inst.act == 2;
            let ok = (corpse && name == "시체")
                || inst.name == name
                || inst.name.starts_with(name)
                || data
                    .reaction_names
                    .iter()
                    .any(|n| *n == name || n.starts_with(name));
            if ok {
                candidates.push((
                    inst.instance_id,
                    inst.mob_key.clone(),
                    inst.name.clone(),
                    corpse,
                    data.clone(),
                ));
            }
        }
        candidates
    };
    if let Some(number) = corpse_number {
        candidates = candidates.into_iter().nth(number - 1).into_iter().collect();
    } else {
        candidates.sort_by_key(|(_, _, mob_name, _, _)| std::cmp::Reverse(mob_name.len()));
    }

    let words_ref: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
    if candidates.is_empty() {
        info!(
            "[try_mob_event] no candidates words={:?} zone={} room={}",
            words_ref, zone, room
        );
    }
    if let Some((instance_id, mob_key, mob_name, corpse, data)) = candidates.first() {
        let event_key = match check_event_key(data, &words_ref) {
            Some(k) => k,
            None => {
                let ev: Vec<&str> = data
                    .events
                    .keys()
                    .filter(|k| k.starts_with("이벤트"))
                    .map(String::as_str)
                    .collect();
                info!(
                    "[try_mob_event] check_event_key=None words={:?} mob_key={} ev_keys={:?}",
                    words_ref, mob_key, ev
                );
                return None;
            }
        };
        if *corpse {
            body.temp_mut()
                .insert("_event_selected_mob_corpse".to_string(), Value::Int(1));
        }
        let result = do_event(body, data, &event_key, &words, mob_key, None, None);
        body.temp_mut().remove("_event_selected_mob_corpse");
        let set_regen = body
            .temp_mut()
            .remove("_event_selected_mob_set_regen")
            .is_some();
        let start_combat = body
            .temp_mut()
            .remove("_event_selected_mob_start_combat")
            .is_some();
        let set_corpse = body
            .temp_mut()
            .remove("_event_selected_mob_set_corpse")
            .is_some();
        if set_regen {
            if let Ok(mut world) = get_world_state().write() {
                if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) {
                    if let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == *instance_id) {
                        mob.act = 3;
                        mob.targets.clear();
                    }
                }
            }
        }
        if start_combat && !*corpse {
            body.act = crate::player::ActState::Fight;
            body.dex = 0;
            crate::script::combat_commands::add_target_instance_id(body, *instance_id);
            body.temp_mut().insert(
                "_attack_target_key".to_string(),
                Value::String(mob_key.clone()),
            );
            body.temp_mut().insert(
                "_attack_target".to_string(),
                Value::String(mob_name.clone()),
            );
            if let Ok(mut world) = get_world_state().write() {
                if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) {
                    if let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == *instance_id) {
                        mob.act = 1;
                        let player_name = body.get_name();
                        if !mob.targets.contains(&player_name) {
                            mob.targets.push(player_name);
                        }
                    }
                }
            }
        }
        if set_corpse {
            if let Ok(mut world) = get_world_state().write() {
                if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) {
                    if let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == *instance_id) {
                        mob.hp = 0;
                        mob.alive = false;
                        mob.act = 2;
                        mob.death_time = chrono::Utc::now().timestamp();
                        mob.targets.clear();
                    }
                }
            }
            body.clear_target(None);
            body.act = crate::player::ActState::Stand;
        }
        return Some(result);
    }

    None
}

/// 범용 인터랙티브 스크립트 한 segment 결과.
#[derive(Debug)]
pub enum ScriptNext {
    Complete,
    Wait {
        line_num: usize,
        prompt: String,
        /// 입력확인 이후 다음 아이템확인/옵션확인에서 쓸 값 (이름/옵션명)
        persist_temp: Option<String>,
        /// true이면 다음 입력에서 "취소" 체크
        from_confirm: bool,
        /// Rhai용: 다음 재개 시 ob. legacy에서는 None.
        script_ob: Option<std::collections::HashMap<String, String>>,
        /// Rhai용: 재개 op (get_word, confirm 등). legacy에서는 None.
        script_resume_op: Option<String>,
    },
}

/// $아이템삭제: 은전/금전이면 속성 감소, objs에서 인덱스 일치 cnt개 제거, 부족하면 inv_stack에서 차감.
fn del_item_from_body(body: &mut Body, index: &str, cnt: i64) {
    if index == "은전" {
        let v = (body.get_int("은전") - cnt).max(0);
        body.set("은전", v);
        return;
    }
    if index == "금전" {
        let v = (body.get_int("금전") - cnt).max(0);
        body.set("금전", v);
        return;
    }
    let mut need = cnt;
    while need > 0 {
        let arc = body.object.find_by_index(index);
        if let Some(a) = arc {
            body.object.remove(&a);
            need -= 1;
        } else {
            break;
        }
    }
    if need > 0 {
        let have = *body.object.inv_stack.get(index).unwrap_or(&0);
        let take = need.min(have);
        if take > 0 {
            let v = have - take;
            if v <= 0 {
                body.object.inv_stack.remove(index);
            } else {
                body.object.inv_stack.insert(index.to_string(), v);
            }
        }
    }
}

/// Legacy `$아이템확인! index count` was migrated as one Rhai string such
/// as `has_item("1000 5")`. Interpret the final decimal token as quantity,
/// while still accepting ordinary one-token item indices.
fn body_has_item_spec(body: &Body, spec: &str) -> bool {
    let mut parts = spec.rsplitn(2, char::is_whitespace);
    let tail = parts.next().unwrap_or_default();
    let (index, required) = match (parts.next(), tail.parse::<i64>()) {
        (Some(index), Ok(required)) if required > 0 => (index.trim_end(), required),
        _ => (spec, 1),
    };
    if index == "은전" || index == "금전" {
        return body.get_int(index) >= required;
    }
    let individual = body
        .object
        .objs
        .iter()
        .filter(|item| {
            item.lock()
                .is_ok_and(|item| item.getString("인덱스") == index)
        })
        .count() as i64;
    let stacked = *body.object.inv_stack.get(index).unwrap_or(&0);
    individual.saturating_add(stacked) >= required
}

/// 무기강화 $옵션확인 / option_confirm efun 로직. mat=합시실, op=특성치명.
fn weapon_upgrade_do_option(
    body: &mut Body,
    mat_arc: &Arc<Mutex<Object>>,
    op: &str,
) -> Result<(), String> {
    if body.get_int("최고내공") < 10 {
        return Err("☞ 내공이 부족해요.".to_string());
    }
    if op == "방어력" {
        return Err("☞ 해당 특성치는 안되요.".to_string());
    }
    let option = mat_arc.lock().ok().and_then(|o| o.get_option());
    let val = match &option {
        Some(m) => *m.get(op).unwrap_or(&0),
        None => 0,
    };
    if val == 0 {
        return Err("☞ 그런 특성치는 없어요.".to_string());
    }
    let mat_idx = mat_arc
        .lock()
        .ok()
        .map(|o| o.getString("인덱스"))
        .unwrap_or_default();
    let 올숙키 = format!("{}_올숙무기", body.get_name());
    let my_arc = body
        .object
        .find_by_index(&올숙키)
        .ok_or_else(|| "☞ 무기를 벗고 하세요.".to_string())?;
    if my_arc.lock().map(|o| o.getBool("inUse")).unwrap_or(false) {
        return Err("☞ 무기를 벗고 하세요.".to_string());
    }
    let mut my_op = my_arc
        .lock()
        .ok()
        .and_then(|o| o.get_option())
        .unwrap_or_default();
    let my_val = *my_op.get(op).unwrap_or(&0);
    if my_val >= val {
        return Err("☞ 현재 특성치 값보다 높아야 합니다.".to_string());
    }
    let mut d = val - my_val;
    if d > 10 {
        d = 10;
    }
    my_op.insert(op.to_string(), my_val + d);
    if let Ok(mut o) = my_arc.lock() {
        o.set_option(&my_op);
        let atk = o.getInt("공격력");
        if atk < 9999 {
            let v = (atk + 10).min(9999);
            o.set("공격력", v);
            o.set("기량", v);
        }
    }
    body.set("최고내공", body.get_int("최고내공") - 10);
    del_item_from_body(body, &mat_idx, 1);
    let atk = my_arc.lock().map(|o| o.getInt("공격력")).unwrap_or(0);
    if atk > 2000 {
        del_item_from_body(body, "강철판", 5);
    }
    let path = format!("data/user/{}.json", body.get_name());
    let _ = save_body_to_json(body, &path);
    body.script_temp_item = None;
    Ok(())
}

/// data/script/ 무기강화 등: $출력시작/끝, $종료, $키입력/$단어입력/$한줄입력(Wait), $입력확인(persist_temp),
/// $아이템확인, $옵션출력, $옵션확인, $무기강화, $아이템삭제.
/// input: 이번에 사용자가 입력한 값. temp_input: $입력확인 이후 유지되는 값(이름/옵션명).
pub fn run_script_chunk(
    body: &mut Body,
    lines: &[String],
    start_at: usize,
    input: Option<String>,
    temp_input: Option<String>,
) -> (Vec<String>, ScriptNext) {
    let mut out: Vec<String> = Vec::new();
    let mut i = start_at;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            out.push(String::new());
            i += 1;
            continue;
        }
        if line.starts_with('#') {
            i += 1;
            continue;
        }
        if line.starts_with('$') {
            let nw = if let Some((_, rest)) = line.split_once(|c: char| c.is_whitespace()) {
                rest.trim()
            } else {
                ""
            };
            if line.starts_with("$출력시작") || line.starts_with("$출력끝") {
                i += 1;
                continue;
            }
            if line.starts_with("$종료") {
                return (out, ScriptNext::Complete);
            }
            if line.starts_with("$키입력")
                || line.starts_with("$단어입력")
                || line.starts_with("$한줄입력")
            {
                let prompt = if nw.is_empty() {
                    "입력: ".to_string()
                } else {
                    format!("{} ", nw)
                };
                return (
                    out,
                    ScriptNext::Wait {
                        line_num: i + 1,
                        prompt,
                        persist_temp: None,
                        from_confirm: false,
                        script_ob: None,
                        script_resume_op: None,
                    },
                );
            }
            if line.starts_with("$입력확인") {
                out.push("입력하신 내용이 맞습니까? (네/취소) : ".to_string());
                let persist = input.clone();
                return (
                    out,
                    ScriptNext::Wait {
                        line_num: i + 1,
                        prompt: String::new(),
                        persist_temp: persist,
                        from_confirm: true,
                        script_ob: None,
                        script_resume_op: None,
                    },
                );
            }
            if line.starts_with("$아이템확인") {
                let name = temp_input.as_deref().unwrap_or("");
                if name.is_empty() {
                    out.push("☞ 그런 아이템이 소지품에 없어요.".to_string());
                    out.push("* 무기강화를 종료합니다.".to_string());
                    return (out, ScriptNext::Complete);
                }
                let arc = body.object.findObjInven(name, 1);
                if let Some(a) = arc {
                    body.script_temp_item = Some(a);
                } else {
                    out.push("☞ 그런 아이템이 소지품에 없어요.".to_string());
                    out.push("* 무기강화를 종료합니다.".to_string());
                    return (out, ScriptNext::Complete);
                }
                i += 1;
                continue;
            }
            if line.starts_with("$옵션출력") {
                let Some(ref arc) = body.script_temp_item else {
                    out.push("* 무기강화를 종료합니다.".to_string());
                    return (out, ScriptNext::Complete);
                };
                if let Ok(o) = arc.lock() {
                    let s = o.get_option_str();
                    if s.is_empty() {
                        out.push("☞ 해당 아이템은 특성치가 없어요.".to_string());
                        out.push("* 무기강화를 종료합니다.".to_string());
                        return (out, ScriptNext::Complete);
                    }
                    out.push(s);
                } else {
                    out.push("* 무기강화를 종료합니다.".to_string());
                    return (out, ScriptNext::Complete);
                }
                i += 1;
                continue;
            }
            if line.starts_with("$옵션확인") {
                let op = temp_input.as_deref().unwrap_or("");
                if op.is_empty() {
                    out.push("☞ 그런 특성치는 없어요.".to_string());
                    out.push("* 무기강화를 종료합니다.".to_string());
                    return (out, ScriptNext::Complete);
                }
                let mat_arc = match body.script_temp_item.clone() {
                    Some(a) => a,
                    None => {
                        out.push("* 무기강화를 종료합니다.".to_string());
                        return (out, ScriptNext::Complete);
                    }
                };
                if let Err(e) = weapon_upgrade_do_option(body, &mat_arc, op) {
                    out.push(e);
                    out.push("* 무기강화를 종료합니다.".to_string());
                    return (out, ScriptNext::Complete);
                }
                i += 1;
                continue;
            }
            if line.starts_with("$아이템삭제") {
                let (index, cnt) = get_str_cnt(line);
                if !index.is_empty() {
                    del_item_from_body(body, &index, cnt);
                }
                i += 1;
                continue;
            }
            if line.starts_with("$무기강화") {
                i += 1;
                continue;
            }
            i += 1;
            continue;
        }
        out.push(line.to_string());
        i += 1;
    }
    (out, ScriptNext::Complete)
}

fn script_hashmap_to_ob(m: HashMap<String, String>) -> Map {
    m.into_iter()
        .map(|(k, v)| (k.into(), Dynamic::from(v)))
        .collect()
}

fn script_ob_to_hashmap(m: Map) -> HashMap<String, String> {
    m.into_iter()
        .filter_map(|(k, v)| v.into_string().ok().map(|s| (k.to_string(), s)))
        .collect()
}

/// Rhai 기반 범용 스크립트. data/script/{script_name}.rhai 실행.
/// script_ob, script_resume_op가 Some이면 해당 op에서 재개.
pub fn run_script_chunk_rhai(
    body: &mut Body,
    script_name: &str,
    input: Option<String>,
    temp_input: Option<String>,
    script_ob: Option<HashMap<String, String>>,
    script_resume_op: Option<String>,
) -> (Vec<String>, ScriptNext) {
    let mut out: Vec<String> = Vec::new();
    let mut ob = script_ob.map(script_hashmap_to_ob).unwrap_or_default();

    let res_op = script_resume_op.as_deref().unwrap_or("");
    let res_in = input.as_deref().unwrap_or("");
    let persist = temp_input.clone().unwrap_or_default();

    let mut engine = Engine::new();
    let out_ptr = &mut out as *mut Vec<String>;
    let body_ptr = body as *mut Body;

    engine.register_fn("send_line", move |_ob: Dynamic, msg: &str| {
        let line = if msg.is_empty() {
            "\r\n".to_string()
        } else {
            format!("{}\r\n", msg)
        };
        unsafe { (*out_ptr).push(line) };
    });

    // end_script는 lib/script/common.rhai에 정의 (throw script_complete)

    let temp_clone = temp_input.clone();
    engine.register_fn("get_persisted", move || {
        temp_clone.clone().unwrap_or_default()
    });

    let temp_for_ci = temp_input.clone();
    engine.register_fn("confirm_item", move |_ob: Dynamic| -> bool {
        let b = unsafe { &mut *body_ptr };
        let name = temp_for_ci.clone().unwrap_or_default();
        if name.is_empty() {
            return false;
        }
        if let Some(arc) = b.object.findObjInven(&name, 1) {
            b.script_temp_item = Some(arc);
            true
        } else {
            false
        }
    });

    engine.register_fn("option_output", move |_ob: Dynamic| -> String {
        let b = unsafe { &*body_ptr };
        let Some(ref arc) = b.script_temp_item else {
            return String::new();
        };
        arc.lock()
            .ok()
            .map(|o| o.get_option_str())
            .unwrap_or_default()
    });

    let temp_for_oc = temp_input.clone();
    engine.register_fn("option_confirm", move |_ob: Dynamic| -> String {
        let b = unsafe { &mut *body_ptr };
        let op = temp_for_oc.clone().unwrap_or_default();
        let mat = match b.script_temp_item.clone() {
            Some(m) => m,
            None => return "* 무기강화를 종료합니다.".to_string(),
        };
        weapon_upgrade_do_option(b, &mat, &op)
            .err()
            .unwrap_or_default()
    });

    engine.register_fn("delete_item", move |index: &str, cnt: i64| {
        let b = unsafe { &mut *body_ptr };
        del_item_from_body(b, index, cnt);
    });

    let common_path = Path::new("lib/script/common.rhai");
    if common_path.exists() {
        let common_src = std::fs::read_to_string(common_path).unwrap_or_default();
        let _ = engine.run(&common_src);
    }

    let mut scope = Scope::new();
    scope.push("ob", ob.clone());
    scope.push("_script_resume_op", res_op.to_string());
    scope.push("_script_resume_input", res_in.to_string());
    scope.push("_script_persist", persist);

    let main_path = Path::new("data/script").join(format!("{}.rhai", script_name));
    let src = match std::fs::read_to_string(&main_path) {
        Ok(s) => s,
        Err(_) => return (out, ScriptNext::Complete),
    };

    let r = engine.eval_with_scope::<Dynamic>(&mut scope, &src);

    if let Some(d) = scope.get_value::<Dynamic>("ob") {
        if let Some(m) = d.try_cast::<Map>() {
            ob = m;
        }
    }

    if let Err(e) = r {
        if let EvalAltResult::ErrorRuntime(err, _) = *e {
            if let Some(m) = err.clone().try_cast::<Map>() {
                let t: String = m
                    .get("type")
                    .and_then(|v: &Dynamic| v.clone().into_string().ok())
                    .unwrap_or_default();
                if t == "script_suspend" {
                    let op: String = m
                        .get("op")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    let prompt: String = m
                        .get("prompt")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    let persist_temp = m
                        .get("persist")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok());
                    return (
                        out,
                        ScriptNext::Wait {
                            line_num: 0,
                            prompt,
                            persist_temp,
                            from_confirm: op == "confirm",
                            script_ob: Some(script_ob_to_hashmap(ob)),
                            script_resume_op: Some(op),
                        },
                    );
                }
                if t == "script_complete" {
                    return (out, ScriptNext::Complete);
                }
            }
        }
    }

    (out, ScriptNext::Complete)
}

/// $엔터$ 재개: mob_key로 데이터 조회 후 do_event( start_from_line, resume_for_rhai ).
/// resume_func: Rhai wait_enter 시 Some("step1") 등. Legacy면 None.
#[allow(clippy::too_many_arguments)]
pub fn try_mob_event_resume(
    body: &mut Body,
    _zone: &str,
    _room: &str,
    mob_key: &str,
    event_key: &str,
    words: Vec<String>,
    line_num: usize,
    resume_func: Option<String>,
) -> Option<CommandResult> {
    let world = get_world_state().read().ok()?;
    let data = world.mob_cache.get_mob(mob_key)?;
    Some(do_event(
        body,
        data,
        event_key,
        &words,
        mob_key,
        Some(line_num),
        resume_func,
    ))
}

#[cfg(test)]
mod tests {
    use super::{body_has_item_spec, check_event_key, do_event_rhai, get_tendency, get_user_event};
    use crate::command::CommandResult;
    use crate::player::Body;
    use crate::world::{EventScript, MobCache, MobInstance, RawMobData, RoomCache};

    fn add_test_items(body: &mut Body, index: &str, count: usize) {
        for _ in 0..count {
            body.object.objs.push(
                crate::script::object_from_item_json(index)
                    .unwrap_or_else(|| panic!("missing item fixture {index}"))
                    .0,
            );
        }
    }

    fn run_luoyang_event(body: &mut Body, script: &str) -> (Vec<String>, Option<(String, String)>) {
        run_zone_event(body, "낙양성", script, None)
    }

    fn run_zone_event(
        body: &mut Body,
        zone: &str,
        script: &str,
        resume_func: Option<&str>,
    ) -> (Vec<String>, Option<(String, String)>) {
        let mut data = RawMobData::new();
        data.zone = zone.to_string();
        match do_event_rhai(
            body,
            &data,
            "test",
            &[],
            "test",
            script,
            resume_func.map(str::to_string),
        ) {
            CommandResult::MobEvent {
                output_lines,
                set_position,
                ..
            } => (output_lines, set_position),
            other => panic!("{script} failed: {other:?}"),
        }
    }

    fn place_event_mob(zone: &str, source_key: &str, room: &str) -> (String, u64) {
        let mut world = crate::world::get_world_state().write().unwrap();
        let data = world
            .mob_cache
            .load_mob(zone, source_key)
            .unwrap_or_else(|_| panic!("missing event mob fixture {zone}:{source_key}"))
            .clone();
        let key = format!("{zone}:{source_key}-회귀-{}", std::process::id());
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        let mob = MobInstance::new(key.clone(), zone.to_string(), room.to_string(), &data);
        let instance_id = mob.instance_id;
        world.mob_cache.add_mob_instance(mob);
        (key, instance_id)
    }

    fn mark_event_mob_corpse(zone: &str, room: &str, instance_id: u64) {
        let mut world = crate::world::get_world_state().write().unwrap();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room_mut(zone, room)
            .unwrap()
            .iter_mut()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        mob.alive = false;
        mob.act = 2;
    }

    #[test]
    fn treasure_map_chain_awards_four_pills_when_unique_armor_already_exists() {
        let mut body = Body::new();
        body.set("이름", "보물지도회귀");

        run_zone_event(&mut body, "사천성", "땅_파_조사.rhai", None);
        assert!(!get_user_event(&body, "보물지도1").is_empty());
        assert!(body_has_item_spec(&body, "합성11 1"));
        assert!(body_has_item_spec(&body, "철판"));

        run_zone_event(&mut body, "낙양성", "땅_파_조사.rhai", None);
        assert!(!get_user_event(&body, "보물지도2").is_empty());
        assert!(body_has_item_spec(&body, "합성11 2"));
        assert!(body_has_item_spec(&body, "종이"));

        run_zone_event(
            &mut body,
            "안휘성",
            "폭풍_끼워_끼_넣어_넣_사용_철판.rhai",
            None,
        );
        assert!(!get_user_event(&body, "보물지도3").is_empty());
        assert!(!body_has_item_spec(&body, "철판"));
        assert!(body_has_item_spec(&body, "보물지도"));

        run_zone_event(&mut body, "호남성", "땅_파_조사.rhai", None);
        assert!(!get_user_event(&body, "보물지도4").is_empty());
        assert!(body_has_item_spec(&body, "보물상자"));

        let mut data = RawMobData::new();
        data.zone = "낙양성".to_string();
        let result = do_event_rhai(
            &mut body,
            &data,
            "test",
            &[],
            "test",
            "합체맨_대화_대_보물상자.rhai",
            None,
        );
        let expected_resume = if crate::oneitem::oneitem_get("701").is_empty() {
            "step_armor"
        } else {
            "step_pills"
        };
        match result {
            CommandResult::MobEventEnter { resume_func, .. } => {
                assert_eq!(resume_func.as_deref(), Some(expected_resume));
            }
            other => panic!("treasure box dialogue did not wait for enter: {other:?}"),
        }

        // Python's already-existing-oneitem branch completes the documented
        // four-pill reward without altering the process-global ONEITEM file.
        run_zone_event(
            &mut body,
            "낙양성",
            "합체맨_대화_대_보물상자.rhai",
            Some("step_pills"),
        );
        assert!(get_user_event(&body, "보물지도1").is_empty());
        assert!(get_user_event(&body, "보물지도2").is_empty());
        assert!(get_user_event(&body, "보물지도3").is_empty());
        assert!(get_user_event(&body, "보물지도4").is_empty());
        assert!(!get_user_event(&body, "보물지도끝").is_empty());
        assert!(!body_has_item_spec(&body, "보물상자"));
        assert!(body_has_item_spec(&body, "합성11 4"));

        run_zone_event(&mut body, "사천성", "땅_파_조사.rhai", None);
        assert!(body_has_item_spec(&body, "합성11 4"));
        let _ = std::fs::remove_file("data/user/보물지도회귀.json");
    }

    #[test]
    fn cheongeum_prison_rank_challenges_start_combat_and_unlock_exit_reward() {
        let player_name = "천금마옥회귀";
        let mut body = Body::new();
        body.set("이름", player_name);
        add_test_items(&mut body, "일수머리", 1);

        let (_, destination) = run_zone_event(&mut body, "감숙성", "17_대화_대.rhai", None);
        assert_eq!(destination, Some(("감숙성".into(), "244".into())));
        assert!(!get_user_event(&body, "즙포사신").is_empty());
        assert!(!body_has_item_spec(&body, "일수머리"));

        let ranks = [
            ("10위", "광동혈괴", "도전", None, "도전장1"),
            ("9위", "곤륜철흉", "도전", Some("도전장1"), "도전장2"),
            ("8위", "대막패황", "도전", Some("도전장2"), "도전장3"),
            ("7위", "혈의라마", "도전", Some("도전장3"), "도전장4"),
            ("6위", "독안마룡", "도전", Some("도전장4"), "도전장5"),
            ("5위", "장비신마", "도전", Some("도전장5"), "도전장6"),
            ("4위", "귀영혈검", "도전", Some("도전장6"), "도전장7"),
            ("3위", "분광쾌검", "도전", Some("도전장7"), "도전장8"),
            ("2위", "추명은검", "도전", Some("도전장8"), "도전장9"),
            ("1위", "천마혈검", "도전", Some("도전장9"), "마옥패"),
        ];
        let room = format!("천금마옥회귀-{}", std::process::id());
        let mut mob_keys = Vec::new();

        for (rank, name, command, required, dropped) in ranks {
            if let Some(item) = required {
                add_test_items(&mut body, item, 1);
            }
            let (mob_key, instance_id) = {
                let mut world = crate::world::get_world_state().write().unwrap();
                let data = world
                    .mob_cache
                    .load_mob("감숙성", rank)
                    .unwrap_or_else(|_| panic!("missing prison rank fixture {rank}"))
                    .clone();
                assert!(data
                    .drop_items
                    .iter()
                    .any(|item| { item.0 == dropped && item.1 == 1 && item.2 == 100 }));
                let key = format!("감숙성:{rank}-회귀-{}", std::process::id());
                world.mob_cache.insert_mob_data(key.clone(), data.clone());
                let mob = MobInstance::new(key.clone(), "감숙성".to_string(), room.clone(), &data);
                let id = mob.instance_id;
                world.mob_cache.add_mob_instance(mob);
                (key, id)
            };
            mob_keys.push(mob_key);

            super::try_mob_event(&mut body, "감숙성", &room, &format!("{name} {command}"))
                .unwrap_or_else(|| panic!("rank challenge was not selected for {rank}"));
            assert_eq!(body.act, crate::player::ActState::Fight, "rank {rank}");
            assert_eq!(
                crate::script::combat_commands::combat_target_instance_ids(&body),
                vec![instance_id],
                "rank {rank} must target the challenged mob",
            );
            if let Some(item) = required {
                assert!(!body_has_item_spec(&body, item), "rank {rank}");
            }
            crate::script::combat_commands::remove_combat_target_instance_id(
                &mut body,
                instance_id,
            );
            body.act = crate::player::ActState::Stand;
        }

        add_test_items(&mut body, "마옥패", 1);
        let (_, destination) = run_zone_event(&mut body, "감숙성", "19_대화_대.rhai", None);
        assert_eq!(destination, Some(("감숙성".into(), "293".into())));
        assert!(!body_has_item_spec(&body, "마옥패"));
        assert!(body_has_item_spec(&body, "802"));
        assert!(!get_user_event(&body, "청강석반지").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in mob_keys {
            world.mob_cache.remove_mob(&key);
        }
        let _ = std::fs::remove_file("data/user/천금마옥회귀.json");
    }

    #[test]
    fn seokga_house_chain_keeps_corpse_gates_combat_and_final_max_mp_reward() {
        let mut body = Body::new();
        body.set("이름", "석가장회귀");
        body.set("체력", 7_000_i64);
        body.set("최고내공", 100_i64);
        let room = format!("석가장회귀-{}", std::process::id());
        let mut mob_keys = Vec::new();

        let (museong_key, museong_id) = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world.mob_cache.load_mob("하북성", "25").unwrap().clone();
            let key = format!("하북성:25-회귀-{}", std::process::id());
            world.mob_cache.insert_mob_data(key.clone(), data.clone());
            let mob = MobInstance::new(key.clone(), "하북성".into(), room.clone(), &data);
            let id = mob.instance_id;
            world.mob_cache.add_mob_instance(mob);
            (key, id)
        };
        mob_keys.push(museong_key.clone());
        super::set_user_event(&mut body, "전투", "1");
        super::try_mob_event(&mut body, "하북성", &room, "무성호법 예 대화")
            .expect("living Museong guardian dialogue must be selected");
        assert_eq!(body.act, crate::player::ActState::Fight);
        assert_eq!(
            crate::script::combat_commands::combat_target_instance_ids(&body),
            vec![museong_id]
        );
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room_mut("하북성", &room)
                .unwrap()
                .iter_mut()
                .find(|mob| mob.instance_id == museong_id)
                .unwrap();
            mob.alive = false;
            mob.act = 2;
        }
        super::try_mob_event(&mut body, "하북성", &room, "무성호법 영대혈 눌러")
            .expect("corpse pressure event must be selected");
        assert!(!get_user_event(&body, "무성호법").is_empty());
        assert!(get_user_event(&body, "전투").is_empty());

        super::set_user_event(&mut body, "피리", "1");
        run_zone_event(&mut body, "하북성", "26_대_대화_예.rhai", None);
        assert_eq!(body.get_int("체력"), 1_000);

        let (zombie_key, zombie_id) = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world.mob_cache.load_mob("하북성", "29").unwrap().clone();
            let key = format!("하북성:29-회귀-{}", std::process::id());
            world.mob_cache.insert_mob_data(key.clone(), data.clone());
            let mob = MobInstance::new(key.clone(), "하북성".into(), room.clone(), &data);
            let id = mob.instance_id;
            world.mob_cache.add_mob_instance(mob);
            (key, id)
        };
        mob_keys.push(zombie_key.clone());
        super::set_user_event(&mut body, "혈유비", "1");
        add_test_items(&mut body, "340", 1);
        super::try_mob_event(&mut body, "하북성", &room, "석융빈 쳐")
            .expect("zombie challenge must be selected");
        assert_eq!(body.act, crate::player::ActState::Fight);
        assert_eq!(
            crate::script::combat_commands::combat_target_instance_ids(&body),
            vec![museong_id, zombie_id]
        );
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room_mut("하북성", &room)
                .unwrap()
                .iter_mut()
                .find(|mob| mob.instance_id == zombie_id)
                .unwrap();
            mob.alive = false;
            mob.act = 2;
        }
        super::try_mob_event(&mut body, "하북성", &room, "석융빈 백우선 꽂아")
            .expect("zombie corpse insert event must be selected");
        assert!(!get_user_event(&body, "마룡혈부").is_empty());
        assert!(get_user_event(&body, "진백우").is_empty());
        {
            let world = crate::world::get_world_state().read().unwrap();
            let zombie = world
                .mob_cache
                .get_all_mobs_in_room("하북성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == zombie_id)
                .unwrap();
            assert_eq!(zombie.act, 3);
        }

        let old_man_key = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world.mob_cache.load_mob("하북성", "28").unwrap().clone();
            let key = format!("하북성:28-회귀-{}", std::process::id());
            world.mob_cache.insert_mob_data(key.clone(), data.clone());
            let mut mob = MobInstance::new(key.clone(), "하북성".into(), room.clone(), &data);
            mob.alive = false;
            mob.act = 2;
            world.mob_cache.add_mob_instance(mob);
            key
        };
        mob_keys.push(old_man_key);
        super::try_mob_event(&mut body, "하북성", &room, "앉은뱅이 마룡혈부 꽂아")
            .expect("old man corpse final insert must be selected");
        assert_eq!(body.get_int("최고내공"), 130);
        assert!(!get_user_event(&body, "마룡혈부끝").is_empty());
        assert!(get_user_event(&body, "마룡혈부").is_empty());

        // 세 만년옥수 웅덩이는 석가장주 단계에서 각각 최고내공을 10씩
        // 올리고, 같은 웅덩이를 다시 마시지 못하게 해야 한다.
        super::del_user_event(&mut body, "마룡혈부끝");
        super::set_user_event(&mut body, "석가장주", "1");
        for (script, flag) in [
            ("30-1_마셔_먹_마.rhai", "만년옥수1"),
            ("30-2_마셔_먹_마.rhai", "만년옥수2"),
            ("30-3_마셔_먹_마.rhai", "만년옥수3"),
        ] {
            let before = body.get_int("최고내공");
            run_zone_event(&mut body, "하북성", script, None);
            assert_eq!(body.get_int("최고내공"), before + 10);
            assert!(!get_user_event(&body, flag).is_empty());
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in mob_keys {
            world.mob_cache.remove_mob(&key);
        }
        let _ = std::fs::remove_file("data/user/석가장회귀.json");
    }

    #[test]
    fn ming_cave_chain_handles_live_targets_corpse_heads_and_faction_rewards() {
        let mut body = Body::new();
        body.set("이름", "명교비동회귀");
        let room = format!("명교비동회귀-{}", std::process::id());
        let mut mob_keys = Vec::new();
        super::set_user_event(&mut body, "양소범요", "1");

        for (source_key, name, head) in [("12", "양소", "양소머리"), ("13", "범요", "범요머리")]
        {
            let (key, instance_id) = place_event_mob("사천성", source_key, &room);
            mob_keys.push(key);
            super::try_mob_event(&mut body, "사천성", &room, &format!("{name} 쳐"))
                .unwrap_or_else(|| panic!("living {name} event must be selected"));
            assert_eq!(body.act, crate::player::ActState::Fight);
            assert!(
                crate::script::combat_commands::combat_target_instance_ids(&body)
                    .contains(&instance_id)
            );
            mark_event_mob_corpse("사천성", &room, instance_id);
            super::try_mob_event(&mut body, "사천성", &room, &format!("{name} 잘라"))
                .unwrap_or_else(|| panic!("corpse {name} event must be selected"));
            assert!(body_has_item_spec(&body, head));
            let world = crate::world::get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room("사천성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            assert_eq!(mob.act, 3);
        }

        super::set_user_event(&mut body, "명교비동", "1");
        run_zone_event(&mut body, "사천성", "11_대화_대.rhai", None);
        assert!(body_has_item_spec(&body, "금정신단"));
        assert!(get_user_event(&body, "명교비동").is_empty());
        assert!(!get_user_event(&body, "금정신단").is_empty());

        super::set_user_event(&mut body, "멸절사태", "1");
        let (abbess_key, abbess_id) = place_event_mob("사천성", "11", &room);
        mob_keys.push(abbess_key);
        super::try_mob_event(&mut body, "사천성", &room, "멸절사태 잘라")
            .expect("living abbess event must be selected");
        assert!(
            crate::script::combat_commands::combat_target_instance_ids(&body).contains(&abbess_id)
        );
        mark_event_mob_corpse("사천성", &room, abbess_id);
        super::try_mob_event(&mut body, "사천성", &room, "멸절사태 잘라")
            .expect("abbess corpse event must be selected");
        assert!(body_has_item_spec(&body, "멸절머리"));
        run_zone_event(&mut body, "사천성", "12_대화_대.rhai", None);
        assert!(body_has_item_spec(&body, "구황신단"));
        assert!(!get_user_event(&body, "구황신단").is_empty());

        run_zone_event(&mut body, "사천성", "14_대화_대.rhai", None);
        assert!(!get_user_event(&body, "의천검").is_empty());
        let (jiji_key, jiji_id) = place_event_mob("사천성", "14", &room);
        mob_keys.push(jiji_key);
        super::try_mob_event(&mut body, "사천성", &room, "주지약 쳐")
            .expect("Jiji attack event must be selected");
        assert!(
            crate::script::combat_commands::combat_target_instance_ids(&body).contains(&jiji_id)
        );

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in mob_keys {
            world.mob_cache.remove_mob(&key);
        }
        let _ = std::fs::remove_file("data/user/명교비동회귀.json");
    }

    #[test]
    fn baekhon_temple_chain_starts_each_guard_fight_and_advances_map_clues() {
        let mut body = Body::new();
        body.set("이름", "백혼사회귀");
        let room = format!("백혼사회귀-{}", std::process::id());
        let mut mob_keys = Vec::new();

        run_zone_event(&mut body, "감숙성", "65_대화_대.rhai", None);
        assert!(!get_user_event(&body, "쇄혼곡").is_empty());

        for (source_key, name, prerequisite, key_item, map_script, map_piece, next_event) in [
            (
                "62",
                "섭혼객",
                "쇄혼곡",
                "철주",
                "78_사용_사_꼽_꼽아_철주_열쇠.rhai",
                "지도조각1",
                "음양과",
            ),
            (
                "63",
                "주벽신장",
                "주벽동",
                "벽옥주",
                "79_사용_사_꼽_꼽아_벽옥주_열쇠.rhai",
                "지도조각2",
                "천독내단",
            ),
            (
                "64",
                "신검장주",
                "신검산장",
                "황금열쇠",
                "80_사용_사_꼽_꼽아_황금열쇠_열쇠.rhai",
                "지도조각3",
                "천령영단",
            ),
        ] {
            let (mob_key, instance_id) = place_event_mob("감숙성", source_key, &room);
            mob_keys.push(mob_key);
            super::try_mob_event(&mut body, "감숙성", &room, &format!("{name} 대화"))
                .unwrap_or_else(|| panic!("{name} prerequisite dialogue must be selected"));
            assert!(get_user_event(&body, name).len() > 0);
            assert!(get_user_event(&body, prerequisite).is_empty());

            super::try_mob_event(&mut body, "감숙성", &room, &format!("{name} 대화"))
                .unwrap_or_else(|| panic!("living {name} challenge must be selected"));
            assert!(
                crate::script::combat_commands::combat_target_instance_ids(&body)
                    .contains(&instance_id)
            );
            mark_event_mob_corpse("감숙성", &room, instance_id);
            super::try_mob_event(&mut body, "감숙성", &room, &format!("{name} 대화"))
                .unwrap_or_else(|| panic!("{name} corpse cleanup must be selected"));
            {
                let world = crate::world::get_world_state().read().unwrap();
                let mob = world
                    .mob_cache
                    .get_all_mobs_in_room("감숙성", &room)
                    .into_iter()
                    .find(|mob| mob.instance_id == instance_id)
                    .unwrap();
                assert_eq!(mob.act, 3);
            }

            add_test_items(&mut body, key_item, 1);
            run_zone_event(&mut body, "감숙성", map_script, None);
            assert!(body_has_item_spec(&body, map_piece));
            run_zone_event(&mut body, "감숙성", "65_대화_대.rhai", None);
            assert!(!get_user_event(&body, next_event).is_empty());

            if next_event == "음양과" {
                add_test_items(&mut body, "음양과", 1);
                run_zone_event(&mut body, "감숙성", "65_대화_대.rhai", None);
                assert!(!get_user_event(&body, "주벽동").is_empty());
            } else if next_event == "천독내단" {
                add_test_items(&mut body, "천독내단", 1);
                run_zone_event(&mut body, "감숙성", "65_대화_대.rhai", None);
                assert!(!get_user_event(&body, "신검산장").is_empty());
            } else {
                add_test_items(&mut body, "천령영단", 1);
                run_zone_event(&mut body, "감숙성", "65_대화_대.rhai", None);
                let (_, destination) = run_zone_event(&mut body, "감숙성", "65_대화_대.rhai", None);
                assert_eq!(destination, Some(("감숙성".into(), "790".into())));
                assert!(!get_user_event(&body, "지하궁전").is_empty());
            }
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in mob_keys {
            world.mob_cache.remove_mob(&key);
        }
        let _ = std::fs::remove_file("data/user/백혼사회귀.json");
    }

    #[test]
    fn sogo_river_chain_starts_final_fight_awards_head_and_retires_player() {
        let mut body = Body::new();
        body.set("이름", "소오강호회귀");
        body.set("힘", 700_i64);
        body.set("맷집", 90_i64);
        body.set("성격", "정파");
        super::set_user_event(&mut body, "동정호진짜끝", "1");

        run_zone_event(&mut body, "동정호", "24_대화_대.rhai", None);
        assert!(body_has_item_spec(&body, "제일신룡단"));
        assert!(!get_user_event(&body, "제일신룡단").is_empty());

        let room = format!("소오강호회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("감숙성", "76", &room);
        super::try_mob_event(&mut body, "감숙성", &room, "강우혁 대화")
            .expect("first Kang Woo-hyuk dialogue must be selected");
        assert!(!get_user_event(&body, "과거").is_empty());
        for expected in ["과거1", "과거2", "과거끝"] {
            super::try_mob_event(&mut body, "감숙성", &room, "강우혁 대화")
                .expect("Kang Woo-hyuk history dialogue must be selected");
            assert!(!get_user_event(&body, expected).is_empty());
        }
        super::try_mob_event(&mut body, "감숙성", &room, "강우혁 대화")
            .expect("final Kang Woo-hyuk challenge must be selected");
        assert!(
            crate::script::combat_commands::combat_target_instance_ids(&body)
                .contains(&instance_id)
        );

        mark_event_mob_corpse("감숙성", &room, instance_id);
        super::try_mob_event(&mut body, "감숙성", &room, "강우혁 머리 잘라")
            .expect("Kang Woo-hyuk corpse head event must be selected");
        assert!(body_has_item_spec(&body, "강우혁머리"));
        {
            let world = crate::world::get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room("감숙성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            assert_eq!(mob.act, 3);
        }

        super::set_user_event(&mut body, "소오강호끝", "1");
        run_luoyang_event(&mut body, "83_대화_대_소오강호.rhai");
        assert!(!body_has_item_spec(&body, "강우혁머리"));
        assert!(!get_user_event(&body, "소오강호진짜끝").is_empty());
        assert_eq!(body.get_int("힘"), 100);
        assert_eq!(body.get_int("맷집"), 15);
        assert_eq!(body.get_string("성격"), "기인");

        crate::world::get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/소오강호회귀.json");
    }

    #[test]
    fn sogo_stone_records_and_reads_the_python_rank_entry() {
        let rank_path = std::path::Path::new("data/config/rank.json");
        let saved_rank_file = std::fs::read(rank_path).ok();
        crate::world::rank::rank_clear("소오강호");

        let mut body = Body::new();
        body.set("이름", "소오강호순위회귀");
        let mut stone = RawMobData::new();
        stone.zone = "구층탑".to_string();
        let CommandResult::MobEvent {
            output_lines: write_lines,
            broadcast_lines,
            ..
        } = do_event_rhai(
            &mut body,
            &stone,
            "test",
            &[],
            "test",
            "비석_적어_적_새겨_이름.rhai",
            None,
        )
        else {
            panic!("stone rank event must finish immediately");
        };
        assert!(write_lines
            .iter()
            .any(|line| line.contains("우수에 진기를")));
        assert!(write_lines
            .iter()
            .any(|line| line.contains("당신이 강호의")));
        assert!(broadcast_lines
            .iter()
            .any(|line| line.contains("소오강호순위회귀가 강호의")));
        assert_eq!(
            crate::world::rank::rank_read("소오강호", "소오강호순위회귀"),
            1
        );
        assert!(!get_user_event(&body, "소오강호끝").is_empty());

        let (view_lines, _) =
            run_zone_event(&mut body, "구층탑", "비석_보_봐_보아_보다_본다.rhai", None);
        assert!(view_lines
            .iter()
            .any(|line| line.contains("소오강호순위회귀") && line.contains("1")));

        crate::world::rank::rank_clear("소오강호");
        if let Some(contents) = saved_rank_file {
            std::fs::write(rank_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(rank_path);
        }
        let _ = std::fs::remove_file("data/user/소오강호순위회귀.json");
    }

    #[test]
    fn shaolin_three_prisoners_require_corpses_then_yield_three_wigs() {
        let room = "shaolin-wig-regression";
        let mut body = Body::new();
        body.set("이름", "소림삼승회귀");
        let mut keys = Vec::new();

        for (mob_key, wig) in [
            ("싸리나무", "가발1"),
            ("산딸기", "가발2"),
            ("도토리", "가발3"),
        ] {
            let (key, instance_id) = place_event_mob("참회동", mob_key, room);
            keys.push(key);
            let live =
                super::try_mob_event(&mut body, "참회동", room, &format!("{mob_key} 가발 벗겨"))
                    .expect("wig command must select its prisoner");
            let CommandResult::MobEvent { output_lines, .. } = live else {
                panic!("live prisoner wig command must be an event");
            };
            assert!(output_lines
                .iter()
                .any(|line| line == "아무일도 일어나지 않습니다"));
            assert!(!body_has_item_spec(&body, wig));

            mark_event_mob_corpse("참회동", room, instance_id);
            super::try_mob_event(&mut body, "참회동", room, &format!("{mob_key} 가발 벗겨"))
                .expect("corpse wig command must select its prisoner");
            assert!(body_has_item_spec(&body, wig));
        }

        let (hand_in, _) = run_zone_event(&mut body, "무림맹", "총관_대_대화_소림사.rhai", None);
        assert!(hand_in.iter().any(|line| line.contains("허접한 가발들")));
        assert!(!body_has_item_spec(&body, "가발1"));
        assert!(!body_has_item_spec(&body, "가발2"));
        assert!(!body_has_item_spec(&body, "가발3"));
        assert!(body_has_item_spec(&body, "합성11"));
        assert!(!get_user_event(&body, "가발임무").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in keys {
            world.mob_cache.remove_mob(&key);
        }
        let _ = std::fs::remove_file("data/user/소림삼승회귀.json");
    }

    #[test]
    fn namhwang_sinta_choice_restores_stat_damage_and_unique_reward_gate() {
        let mut rooms = RoomCache::new();
        let final_room = rooms.get_room("강소성", "121").unwrap();
        let final_room = final_room.read().unwrap();
        assert_eq!(final_room.display_name, "만병천살대진 - 남황신타묘");
        assert_eq!(final_room.mob_ids, vec!["육자홍", "진마혁"]);
        drop(final_room);
        let monk_room = rooms.get_room("강소성", "239").unwrap();
        assert_eq!(monk_room.read().unwrap().mob_ids, vec!["무무성승"]);

        let mut body = Body::new();
        body.set("이름", "남황신타회귀");
        body.set("체력", 1_100_000_i64);
        body.set("최고내공", 100_i64);

        super::set_user_event(&mut body, "황룡마조1", "1");
        run_zone_event(&mut body, "강소성", "진마혁_부셔_부수.rhai", None);
        assert_eq!(body.get_int("최고내공"), 150);
        assert!(!get_user_event(&body, "진마혁끝").is_empty());

        run_zone_event(&mut body, "강소성", "화문청_부셔_부수_부.rhai", None);
        assert_eq!(body.get_int("체력"), 1_080_000);
        super::set_user_event(&mut body, "파사신검", "1");
        run_zone_event(
            &mut body,
            "강소성",
            "파사신검_가져_가_집_집어_주워_주_뽑_뽑아.rhai",
            None,
        );
        assert_eq!(body.get_int("체력"), 80_000);
        assert!(get_user_event(&body, "파사신검").is_empty());

        let mut dialogue_body = Body::new();
        dialogue_body.set("이름", "남황신타대화회귀");
        super::set_user_event(&mut dialogue_body, "운조하4", "1");
        let mut dialogue_data = RawMobData::new();
        dialogue_data.zone = "강소성".to_string();
        let first = do_event_rhai(
            &mut dialogue_body,
            &dialogue_data,
            "test",
            &[],
            "test",
            "무무성승_절.rhai",
            None,
        );
        let CommandResult::MobEventEnter { resume_func, .. } = first else {
            panic!("Mu-mu monk first dialogue must wait for enter");
        };
        assert_eq!(resume_func.as_deref(), Some("step1"));
        let second = do_event_rhai(
            &mut dialogue_body,
            &dialogue_data,
            "test",
            &[],
            "test",
            "무무성승_절.rhai",
            resume_func,
        );
        let CommandResult::MobEventEnter { resume_func, .. } = second else {
            panic!("Mu-mu monk second dialogue must wait for enter");
        };
        assert_eq!(resume_func.as_deref(), Some("step2"));
        let CommandResult::MobEvent { set_position, .. } = do_event_rhai(
            &mut dialogue_body,
            &dialogue_data,
            "test",
            &[],
            "test",
            "무무성승_절.rhai",
            resume_func,
        ) else {
            panic!("Mu-mu monk final dialogue must complete");
        };
        assert_eq!(set_position, Some(("강소성".into(), "121".into())));
        assert!(!get_user_event(&dialogue_body, "무무성승").is_empty());

        let mut alternate_body = Body::new();
        alternate_body.set("이름", "남황신타대체회귀");
        for event in ["황룡마조2", "진마혁끝", "무무성승", "운조하4"] {
            super::set_user_event(&mut alternate_body, event, "1");
        }
        run_zone_event(&mut alternate_body, "강소성", "육자홍_절.rhai", None);
        assert!(body_has_item_spec(&alternate_body, "황룡마조-5"));
        assert!(!get_user_event(&alternate_body, "황룡마조끝").is_empty());
        for event in ["황룡마조2", "진마혁끝", "무무성승", "운조하4"] {
            assert!(get_user_event(&alternate_body, event).is_empty());
        }

        if !crate::oneitem::oneitem_get_index_by_name("황룡마조").is_empty()
            && crate::oneitem::oneitem_get("황룡마조").is_empty()
        {
            run_zone_event(&mut body, "강소성", "육자홍_절.rhai", None);
            assert!(body_has_item_spec(&body, "황룡마조"));
            assert!(!get_user_event(&body, "황룡마조끝").is_empty());
        }
        let _ = std::fs::remove_file("data/user/남황신타회귀.json");
        let _ = std::fs::remove_file("data/user/남황신타대체회귀.json");
        let _ = std::fs::remove_file("data/user/남황신타대화회귀.json");
    }

    #[test]
    fn ma_ryeong_valley_loads_original_final_room_exits_and_guard_stats() {
        let mut rooms = RoomCache::new();
        let final_room = rooms.get_room("호남성", "272").unwrap();
        let final_room = final_room.read().unwrap();
        assert_eq!(final_room.display_name, "마령곡-천의검마전");
        assert!(final_room.exits.is_empty());
        assert_eq!(final_room.mob_ids, vec!["39"]);
        drop(final_room);

        let maze_room = rooms.get_room("호남성", "203").unwrap();
        let maze_room = maze_room.read().unwrap();
        assert_eq!(maze_room.display_name, "마령곡-사상환형살무진");
        assert_eq!(
            maze_room
                .exits
                .get("북")
                .unwrap()
                .destination
                .as_ref()
                .unwrap(),
            &("호남성".to_string(), "202".to_string())
        );
        assert_eq!(maze_room.mob_ids, vec!["33"]);

        let mut mobs = MobCache::new();
        let final_guard = mobs.load_mob("호남성", "39").unwrap();
        assert_eq!(final_guard.name, "천의검마");
        assert_eq!(final_guard.level, 840);
        assert_eq!(final_guard.combat_type, 0);
        assert_eq!(final_guard.locations, vec!["272"]);
    }

    #[test]
    fn blood_mist_valley_guard_starts_masked_fight_or_escorts_to_final_room() {
        let mut rooms = RoomCache::new();
        let final_gate = rooms.get_room("산서성", "1189").unwrap();
        let final_gate = final_gate.read().unwrap();
        assert_eq!(final_gate.display_name, "오대산 혈무별원");
        assert!(final_gate.exits.is_empty());
        assert_eq!(final_gate.mob_ids, vec!["66"]);
        drop(final_gate);

        let room = format!("혈무곡회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("산서성", "66", &room);
        let mut masked = Body::new();
        masked.set("이름", "혈무곡가면회귀");
        add_test_items(&mut masked, "인피면구", 1);
        super::try_mob_event(&mut masked, "산서성", &room, "혈광호위 대화")
            .expect("masked guard dialogue must be selected");
        assert!(
            crate::script::combat_commands::combat_target_instance_ids(&masked)
                .contains(&instance_id)
        );

        let mut unmasked = Body::new();
        unmasked.set("이름", "혈무곡안내회귀");
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut unmasked, "산서성", &room, "혈광호위 대화")
                .expect("unmasked guard dialogue must be selected")
        else {
            panic!("guard dialogue must complete immediately");
        };
        assert_eq!(set_position, Some(("산서성".into(), "1190".into())));

        crate::world::get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/혈무곡가면회귀.json");
        let _ = std::fs::remove_file("data/user/혈무곡안내회귀.json");
    }

    #[test]
    fn heaven_earth_palace_grants_unique_or_fallback_and_marks_the_statue_corpse() {
        let room = format!("천지제황회귀-{}", std::process::id());
        let mut keys = Vec::new();
        for (source, name, unique, fallback) in [
            ("무황", "무황", "348", "348-5"),
            ("요마", "요마", "163", "163-5"),
        ] {
            let (key, id) = place_event_mob("동정호", source, &room);
            keys.push(key);
            let mut body = Body::new();
            body.set("이름", format!("제황{}회귀", name));
            super::try_mob_event(&mut body, "동정호", &room, &format!("{name} 절"))
                .expect("palace statue prayer must be selected");
            assert!(body_has_item_spec(&body, unique) || body_has_item_spec(&body, fallback));
            let world = crate::world::get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room("동정호", &room)
                .into_iter()
                .find(|mob| mob.instance_id == id)
                .unwrap();
            assert!(!mob.alive);
            assert_eq!(mob.act, 2);
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        for key in keys {
            world.mob_cache.remove_mob(&key);
        }
    }

    #[test]
    fn o_ryong_and_lightning_temple_rooms_keep_original_guard_placement() {
        let mut rooms = RoomCache::new();
        for (room, mob, display) in [
            ("499", "52", "오룡성전 해천신룡각"),
            ("500", "53", "오룡성전 성결화룡각"),
            ("501", "54", "오룡성전 제천황룡각"),
            ("502", "55", "오룡성전 월영창룡각"),
            ("503", "56", "오룡성전 환무운룡각"),
        ] {
            let loaded = rooms.get_room("섬서성", room).unwrap();
            let loaded = loaded.read().unwrap();
            assert_eq!(loaded.display_name, display);
            assert_eq!(loaded.mob_ids, vec![mob]);
        }
        let lightning = rooms.get_room("청해성", "95").unwrap();
        let lightning = lightning.read().unwrap();
        assert_eq!(lightning.mob_ids, vec!["1"]);

        let mut mobs = MobCache::new();
        assert_eq!(mobs.load_mob("섬서성", "56").unwrap().name, "신군");
        assert_eq!(mobs.load_mob("청해성", "1").unwrap().name, "벽력신장");
    }

    #[test]
    fn tendency_matches_python_kill_threshold_and_alignment_counts() {
        let threshold = crate::script::get_murim_config_int("무림별호이벤트킬수");
        let mut body = Body::new();
        body.set("성격", "정파");
        body.set("0 성격플킬", threshold - 1);
        assert!(!get_tendency(&body, "정파"));
        assert!(!get_tendency(&body, "사파"));

        body.set("0 성격플킬", threshold);
        assert!(get_tendency(&body, "정파"));
        assert!(get_tendency(&body, "사파"));

        body.set("0 성격플킬", 0_i64);
        body.set("1 성격플킬", threshold);
        body.set("2 성격플킬", 0_i64);
        assert!(get_tendency(&body, "정파"));
        assert!(!get_tendency(&body, "사파"));

        body.set("1 성격플킬", 0_i64);
        body.set("2 성격플킬", threshold);
        assert!(!get_tendency(&body, "정파"));
        assert!(get_tendency(&body, "사파"));
    }

    #[test]
    fn test_check_event_key_picks_편지_over_대화_for_왕대협_편지_대화() {
        let mut data = RawMobData::new();
        data.events.insert(
            "이벤트 $대화 $대".to_string(),
            EventScript::Rhai("83_대화_대.rhai".to_string()),
        );
        data.events.insert(
            "이벤트: $대화 $대 편지".to_string(),
            EventScript::Rhai("83_편지.rhai".to_string()),
        );
        let words = ["왕대협", "편지", "대화"];
        let got = check_event_key(&data, &words);
        assert_eq!(got.as_deref(), Some("이벤트: $대화 $대 편지"));
    }

    #[test]
    fn legacy_event_item_check_supports_index_and_quantity() {
        let mut body = Body::new();
        body.set("은전", 9_999_i64);
        add_test_items(&mut body, "1000", 5);
        assert!(body_has_item_spec(&body, "1000 5"));
        assert!(!body_has_item_spec(&body, "1000 6"));
        assert!(body_has_item_spec(&body, "은전 9999"));
        assert!(!body_has_item_spec(&body, "은전 10000"));
    }

    #[test]
    fn baeksa_liquor_event_completes_all_state_and_item_transitions() {
        let mut body = Body::new();
        body.set("이름", "백사주회귀");

        run_luoyang_event(&mut body, "9_대화_대_취선노인_취선.rhai");
        assert!(!get_user_event(&body, "취선노인").is_empty());
        assert!(body_has_item_spec(&body, "편지"));

        run_luoyang_event(&mut body, "정보맨_대화_대_편지.rhai");
        assert!(!get_user_event(&body, "편지").is_empty());
        run_luoyang_event(&mut body, "83_편지.rhai");
        assert!(!get_user_event(&body, "취선노인1").is_empty());
        assert!(!body_has_item_spec(&body, "편지"));
        run_luoyang_event(&mut body, "84_대화_대_취선노인_취선.rhai");
        assert!(!get_user_event(&body, "취선노인2").is_empty());
        run_luoyang_event(&mut body, "곤륜선인_대화_대_은린빙백사_백사.rhai");
        assert!(!get_user_event(&body, "곤륜선인").is_empty());
        run_luoyang_event(&mut body, "83_대화_대_은린빙백사_백사.rhai");
        assert!(!get_user_event(&body, "취선노인3").is_empty());
        // The second Wang dialogue gives the tool-shop hint without changing state.
        run_luoyang_event(&mut body, "83_대화_대_은린빙백사_백사.rhai");
        run_luoyang_event(&mut body, "2_대화_대_호리병.rhai");
        assert!(!get_user_event(&body, "취선노인4").is_empty());

        add_test_items(&mut body, "합성8", 1);
        run_luoyang_event(&mut body, "2_줘_적송오지.rhai");
        assert!(!get_user_event(&body, "취선노인5").is_empty());
        assert!(!body_has_item_spec(&body, "합성8"));
        add_test_items(&mut body, "합성2", 1);
        run_luoyang_event(&mut body, "2_줘_설삼과.rhai");
        assert!(!get_user_event(&body, "취선노인6").is_empty());
        assert!(body_has_item_spec(&body, "호리병"));

        run_luoyang_event(&mut body, "6_대화_대_도토리절편.rhai");
        assert!(!get_user_event(&body, "취선노인7").is_empty());
        add_test_items(&mut body, "1000", 5);
        run_luoyang_event(&mut body, "6_줘_도토리.rhai");
        assert!(!get_user_event(&body, "취선노인8").is_empty());
        assert!(!body_has_item_spec(&body, "1000"));
        add_test_items(&mut body, "1001", 1);
        run_luoyang_event(&mut body, "6_줘_산딸기.rhai");
        assert!(!get_user_event(&body, "취선노인9").is_empty());
        assert!(body_has_item_spec(&body, "도토리절편"));
        run_luoyang_event(&mut body, "6_대화_대_도토리절편.rhai");
        assert!(body_has_item_spec(&body, "호리병1"));
        assert!(!body_has_item_spec(&body, "호리병"));
        assert!(!body_has_item_spec(&body, "도토리절편"));

        run_luoyang_event(&mut body, "호리병_묻어.rhai");
        assert!(!get_user_event(&body, "취선노인11").is_empty());
        run_luoyang_event(&mut body, "호리병_막아_닫아_마개.rhai");
        assert!(!get_user_event(&body, "취선노인12").is_empty());
        assert!(!get_user_event(&body, "취선노인10").is_empty());
        assert!(body_has_item_spec(&body, "호리병2"));
        run_luoyang_event(&mut body, "83_대화_대_은린빙백사_백사.rhai");
        assert!(get_user_event(&body, "취선노인10").is_empty());
        run_luoyang_event(&mut body, "83_대화_대_은린빙백사_백사.rhai");

        let (_, destination) = run_luoyang_event(&mut body, "5_대화_대_은린빙백사_백사.rhai");
        assert_eq!(destination, Some(("낙양성".into(), "7000".into())));
        run_luoyang_event(&mut body, "중년인_대화_대.rhai");
        assert!(!get_user_event(&body, "취선노인13").is_empty());
        assert!(!body_has_item_spec(&body, "호리병2"));

        add_test_items(&mut body, "271", 5);
        run_luoyang_event(&mut body, "아궁이_이벤트__넣_넣어_너_싸리나무.rhai");
        assert!(!get_user_event(&body, "취선노인14").is_empty());
        assert!(!body_has_item_spec(&body, "271"));
        let (_, destination) = run_luoyang_event(&mut body, "중년인_대화_대.rhai");
        assert_eq!(destination, Some(("낙양성".into(), "7002".into())));
        run_luoyang_event(&mut body, "중년인-1_대화_대.rhai");
        assert!(!get_user_event(&body, "취선노인14-1").is_empty());
        run_luoyang_event(&mut body, "중년인-1_대화_대.rhai");
        assert!(!get_user_event(&body, "취선노인15").is_empty());
        assert!(body_has_item_spec(&body, "백사주"));

        add_test_items(&mut body, "견자단", 1);
        run_luoyang_event(&mut body, "중년인-1_대화_대.rhai");
        assert!(!get_user_event(&body, "취선노인끝").is_empty());
        assert!(!body_has_item_spec(&body, "견자단"));
        assert!(body_has_item_spec(&body, "은린3"));
    }

    #[test]
    fn badger_hide_event_uses_corpse_then_awards_two_max_mp_pills() {
        let mut body = Body::new();
        body.set("이름", "오소리가죽회귀");
        body.set("최고내공", 10_i64);
        body.set("내공", 10_i64);

        run_luoyang_event(&mut body, "82_대화_대.rhai");
        assert!(!get_user_event(&body, "오소리가죽 이벤트").is_empty());

        let room = format!("오소리가죽회귀-{}", std::process::id());
        let mob_key = format!("낙양성:25-회귀-{}", std::process::id());
        let instance_id = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("낙양성", "25")
                .expect("badger fixture")
                .clone();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let mut corpse =
                MobInstance::new(mob_key.clone(), "낙양성".to_string(), room.clone(), &data);
            corpse.alive = false;
            corpse.act = 2;
            let instance_id = corpse.instance_id;
            world.mob_cache.add_mob_instance(corpse);
            instance_id
        };
        let result = super::try_mob_event(&mut body, "낙양성", &room, "시체 가죽 벗겨")
            .expect("corpse event must be selected");
        assert!(matches!(result, CommandResult::MobEvent { .. }));
        assert!(body_has_item_spec(&body, "오소리가죽"));
        {
            let world = crate::world::get_world_state().read().unwrap();
            let corpse = world
                .mob_cache
                .get_all_mobs_in_room("낙양성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            assert_eq!(corpse.act, 3, "skinned corpse must enter regen state");
        }

        run_luoyang_event(&mut body, "82_대화_대_오소리_가죽_오소리가죽.rhai");
        assert!(get_user_event(&body, "오소리가죽 이벤트").is_empty());
        assert!(!get_user_event(&body, "오소리가죽 이벤트 끝").is_empty());
        assert!(!body_has_item_spec(&body, "오소리가죽"));
        assert!(body_has_item_spec(&body, "합성10 2"));

        let storage = crate::script::ScriptStorage::default();
        storage
            .execute("먹어", &mut body, "음양속고구환단", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("최고내공"), 11);
        assert!(body_has_item_spec(&body, "합성10 1"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_instance("낙양성", &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/오소리가죽회귀.json");
    }

    #[test]
    fn deer_antler_event_uses_corpse_then_awards_two_max_mp_pills() {
        let mut body = Body::new();
        body.set("이름", "녹용회귀");
        body.set("최고내공", 10_i64);
        body.set("내공", 10_i64);

        run_luoyang_event(&mut body, "82_대화_대_사슴_녹용_뿔.rhai");
        assert!(!get_user_event(&body, "녹용이벤트").is_empty());

        let room = format!("녹용회귀-{}", std::process::id());
        let mob_key = format!("낙양성:24-회귀-{}", std::process::id());
        let instance_id = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("낙양성", "24")
                .expect("deer fixture")
                .clone();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let mut corpse =
                MobInstance::new(mob_key.clone(), "낙양성".to_string(), room.clone(), &data);
            corpse.alive = false;
            corpse.act = 2;
            let instance_id = corpse.instance_id;
            world.mob_cache.add_mob_instance(corpse);
            instance_id
        };
        let result = super::try_mob_event(&mut body, "낙양성", &room, "시체 녹용 잘라")
            .expect("corpse event must be selected");
        assert!(matches!(result, CommandResult::MobEvent { .. }));
        assert!(body_has_item_spec(&body, "녹용"));
        {
            let world = crate::world::get_world_state().read().unwrap();
            let corpse = world
                .mob_cache
                .get_all_mobs_in_room("낙양성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            assert_eq!(corpse.act, 3, "cut corpse must enter regen state");
        }

        run_luoyang_event(&mut body, "82_대화_대_사슴_녹용_뿔.rhai");
        assert!(get_user_event(&body, "녹용이벤트").is_empty());
        assert!(!get_user_event(&body, "녹용이벤트끝").is_empty());
        assert!(!body_has_item_spec(&body, "녹용"));
        assert!(body_has_item_spec(&body, "합성10 2"));
        run_luoyang_event(&mut body, "82_대화_대_사슴_녹용_뿔.rhai");
        assert!(body_has_item_spec(&body, "합성10 2"));

        let storage = crate::script::ScriptStorage::default();
        storage
            .execute("먹어", &mut body, "음양속고구환단", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("최고내공"), 11);
        assert!(body_has_item_spec(&body, "합성10 1"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_instance("낙양성", &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/녹용회귀.json");
    }

    #[test]
    fn royal_tomb_corpse_awards_one_three_color_lotus_and_it_adds_thirty_max_mp() {
        let mut body = Body::new();
        body.set("이름", "삼색수련회귀");
        body.set("최고내공", 100_i64);
        body.set("내공", 100_i64);

        let room = format!("삼색수련회귀-{}", std::process::id());
        let mob_key = format!("낙양성:38-회귀-{}", std::process::id());
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("낙양성", "38")
                .expect("martial artist corpse fixture")
                .clone();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                "낙양성".to_string(),
                room.clone(),
                &data,
            ));

            let statue = world
                .mob_cache
                .load_mob("낙양성", "35")
                .expect("stone statue fixture");
            assert!(statue
                .use_items
                .iter()
                .any(|item| { item.0 == "275" && item.1 == 1 && item.2 == 10 }));
            let general = world
                .mob_cache
                .load_mob("낙양성", "37")
                .expect("general statue fixture");
            assert!(general
                .use_items
                .iter()
                .any(|item| { item.0 == "275" && item.1 == 1 && item.2 == 5 }));
        }

        let result = super::try_mob_event(&mut body, "낙양성", &room, "무림인 뒤져")
            .expect("corpse search event must be selected");
        let CommandResult::MobEvent { output_lines, .. } = result else {
            panic!("corpse search must execute a mob event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("삼색수련") && line.contains("시체")));
        assert!(body_has_item_spec(&body, "yak1 1"));
        assert!(!get_user_event(&body, "삼색수련").is_empty());

        let repeated = super::try_mob_event(&mut body, "낙양성", &room, "무림인 뒤져")
            .expect("repeated corpse search event must still be selected");
        let CommandResult::MobEvent { output_lines, .. } = repeated else {
            panic!("repeated corpse search must execute a mob event");
        };
        assert!(output_lines.iter().any(|line| line.contains("뭐 없나~~")));
        assert!(body_has_item_spec(&body, "yak1 1"));

        let storage = crate::script::ScriptStorage::default();
        storage
            .execute("먹어", &mut body, "삼색수련", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("최고내공"), 130);
        assert!(!body_has_item_spec(&body, "yak1"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_instance("낙양성", &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/삼색수련회귀.json");
    }

    #[test]
    fn general_tomb_water_event_moves_through_statue_and_awards_one_soul_pill() {
        let mut body = Body::new();
        body.set("이름", "초혼단회귀");
        body.set("최고내공", 100_i64);
        body.set("내공", 100_i64);

        let entrance_room = format!("장군묘입구회귀-{}", std::process::id());
        let statue_key = format!("낙양성:93-회귀-{}", std::process::id());
        let hermit_room = format!("영환도사회귀-{}", std::process::id());
        let hermit_key = format!("낙양성:97-회귀-{}", std::process::id());
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let statue = world
                .mob_cache
                .load_mob("낙양성", "93")
                .expect("general tomb statue fixture")
                .clone();
            world
                .mob_cache
                .insert_mob_data(statue_key.clone(), statue.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                statue_key.clone(),
                "낙양성".to_string(),
                entrance_room.clone(),
                &statue,
            ));

            let hermit = world
                .mob_cache
                .load_mob("낙양성", "97")
                .expect("Younghwan hermit fixture")
                .clone();
            world
                .mob_cache
                .insert_mob_data(hermit_key.clone(), hermit.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                hermit_key.clone(),
                "낙양성".to_string(),
                hermit_room.clone(),
                &hermit,
            ));

            let innkeeper = world
                .mob_cache
                .load_mob("낙양성", "5")
                .expect("Luoyang innkeeper fixture");
            assert!(innkeeper
                .items_for_sale
                .iter()
                .any(|item| item.0 == "1055" && item.1 == 100));

            let white_fox = world
                .mob_cache
                .load_mob("낙양성", "31")
                .expect("white fox fixture");
            assert_eq!(white_fox.combat_type, 1);
            assert!(white_fox
                .drop_items
                .iter()
                .any(|item| { item.0 == "412" && item.1 == 1 && item.2 == 10 }));
            for mob_id in ["94", "95", "96"] {
                let zombie = world
                    .mob_cache
                    .load_mob("낙양성", mob_id)
                    .unwrap_or_else(|_| panic!("zombie fixture {mob_id}"));
                assert!(zombie.combat_type == 1 || zombie.combat_type == 2);
                assert!(zombie
                    .drop_items
                    .iter()
                    .any(|item| { item.0 == "1003" && item.1 == 1 && item.2 == 5 }));
            }
        }

        let moved = super::try_mob_event(&mut body, "낙양성", &entrance_room, "석상 돌려")
            .expect("statue turn event must be selected");
        let CommandResult::MobEvent { set_position, .. } = moved else {
            panic!("statue turn must execute a mob event");
        };
        assert_eq!(
            set_position,
            Some(("낙양성".to_string(), "1455".to_string()))
        );

        let missing = super::try_mob_event(&mut body, "낙양성", &hermit_room, "영환 생수 줘")
            .expect("water event without water must be selected");
        let CommandResult::MobEvent { output_lines, .. } = missing else {
            panic!("missing-water branch must execute a mob event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("물....물...물")));
        assert!(!body_has_item_spec(&body, "yak12"));

        add_test_items(&mut body, "1055", 1);
        let rewarded = super::try_mob_event(&mut body, "낙양성", &hermit_room, "영환 생수 줘")
            .expect("water reward event must be selected");
        let CommandResult::MobEvent { output_lines, .. } = rewarded else {
            panic!("water reward branch must execute a mob event");
        };
        assert!(output_lines.iter().any(|line| line.contains("초혼단")));
        assert!(!body_has_item_spec(&body, "1055"));
        assert!(body_has_item_spec(&body, "yak12 1"));
        assert!(!get_user_event(&body, "영환도사").is_empty());

        add_test_items(&mut body, "1055", 1);
        super::try_mob_event(&mut body, "낙양성", &hermit_room, "영환 생수 줘")
            .expect("repeated water event must be selected");
        assert!(!body_has_item_spec(&body, "1055"));
        assert!(body_has_item_spec(&body, "yak12 1"));

        let storage = crate::script::ScriptStorage::default();
        storage
            .execute("먹어", &mut body, "초혼단", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("최고내공"), 110);
        assert!(!body_has_item_spec(&body, "yak12"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world
            .mob_cache
            .remove_instance("낙양성", &entrance_room, &statue_key);
        world.mob_cache.remove_mob(&statue_key);
        world
            .mob_cache
            .remove_instance("낙양성", &hermit_room, &hermit_key);
        world.mob_cache.remove_mob(&hermit_key);
        let _ = std::fs::remove_file("data/user/초혼단회귀.json");
    }

    #[test]
    fn bandit_bounty_accepts_numbered_boss_corpse_and_pays_once() {
        let mut body = Body::new();
        body.set("이름", "산적현상금회귀");
        body.set("은전", 100_i64);
        body.set("최고내공", 10_i64);
        body.set("내공", 10_i64);

        run_luoyang_event(&mut body, "80_대화_대.rhai");
        assert!(!get_user_event(&body, "포교 산적 이벤트").is_empty());
        run_luoyang_event(&mut body, "80_대화_대.rhai");

        let room = format!("산적두목회귀-{}", std::process::id());
        let mob_key = format!("낙양성:45-회귀-{}", std::process::id());
        let third_id = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("낙양성", "45")
                .expect("bandit boss fixture")
                .clone();
            assert!(data.combat_type == 2);
            assert!(data
                .use_items
                .iter()
                .any(|item| item.0 == "99" && item.2 == 8));
            assert!(data
                .use_items
                .iter()
                .any(|item| item.0 == "699" && item.2 == 2));
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            for _ in 0..3 {
                let mut corpse =
                    MobInstance::new(mob_key.clone(), "낙양성".to_string(), room.clone(), &data);
                corpse.alive = false;
                corpse.act = 2;
                world.mob_cache.add_mob_instance(corpse);
            }
            world
                .mob_cache
                .get_all_mobs_in_room("낙양성", &room)
                .into_iter()
                .filter(|mob| mob.act == 2)
                .nth(2)
                .expect("third corpse in room order")
                .instance_id
        };
        super::try_mob_event(&mut body, "낙양성", &room, "3시체 머리 잘라")
            .expect("numbered third corpse event must be selected");
        assert!(body_has_item_spec(&body, "두목머리"));
        {
            let world = crate::world::get_world_state().read().unwrap();
            let third = world
                .mob_cache
                .get_all_mobs_in_room("낙양성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == third_id)
                .unwrap();
            assert_eq!(third.act, 3);
        }

        run_luoyang_event(&mut body, "80_대화_대_산적_머리.rhai");
        assert!(get_user_event(&body, "포교 산적 이벤트").is_empty());
        assert!(!get_user_event(&body, "포교 산적 이벤트 끝").is_empty());
        assert_eq!(body.get_int("은전"), 30_100);
        assert!(body_has_item_spec(&body, "합성10 2"));
        assert!(!body_has_item_spec(&body, "두목머리"));
        run_luoyang_event(&mut body, "80_대화_대_산적_머리.rhai");
        assert_eq!(body.get_int("은전"), 30_100);
        assert!(body_has_item_spec(&body, "합성10 2"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/산적현상금회귀.json");
    }

    #[test]
    fn fawang_temple_rock_event_refills_searches_and_awards_five_pills_once() {
        let player_name = "법왕사바윗돌회귀";
        let room = format!("법왕사바위회귀-{}", std::process::id());
        let mob_key = format!("낙양성:87-회귀-{}", std::process::id());
        let mut body = Body::new();
        body.set("이름", player_name);
        body.set("힘", 100_i64);
        body.set("최고내공", 10_i64);
        body.set("내공", 10_i64);

        run_luoyang_event(&mut body, "10_대화_대.rhai");
        assert!(!get_user_event(&body, "바윗돌 이벤트").is_empty());

        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("낙양성", "87")
                .expect("rock pile fixture")
                .clone();
            assert_eq!(data.mob_type, 6);
            assert_eq!(
                data.item_regen, 180,
                "Python clamps item regen to 180 seconds"
            );
            assert!(data
                .drop_items
                .iter()
                .any(|item| { item.0 == "바윗돌" && item.1 == 1 && item.2 == 100 }));
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let mut pile =
                MobInstance::new(mob_key.clone(), "낙양성".to_string(), room.clone(), &data);
            pile.time_of_regen = chrono::Utc::now().timestamp() - data.item_regen;
            world.mob_cache.add_mob_instance(pile);
            world.set_player_position(
                player_name,
                crate::world::PlayerPosition::new("낙양성".to_string(), room.clone()),
            );
            world.update_occupied_room_mobs(&[("낙양성".to_string(), room.clone())]);
        }

        let storage = crate::script::ScriptStorage::default();
        storage
            .execute("뒤져", &mut body, "바위", None, None, None)
            .unwrap();
        assert!(body_has_item_spec(&body, "바윗돌"));
        assert_eq!(body.get_item_weight(), 550);

        run_luoyang_event(&mut body, "10_대화_대_바윗돌_바위_돌.rhai");
        assert!(get_user_event(&body, "바윗돌 이벤트").is_empty());
        assert!(!get_user_event(&body, "바윗돌 이벤트 끝").is_empty());
        assert!(!body_has_item_spec(&body, "바윗돌"));
        assert!(body_has_item_spec(&body, "합성10 5"));
        run_luoyang_event(&mut body, "10_대화_대_바윗돌_바위_돌.rhai");
        assert!(body_has_item_spec(&body, "합성10 5"));

        storage
            .execute("먹어", &mut body, "음양속고구환단", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("최고내공"), 11);
        assert!(body_has_item_spec(&body, "합성10 4"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.remove_player_position(player_name);
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/법왕사바윗돌회귀.json");
    }

    #[test]
    fn taesil_peak_leaf_chain_ends_with_live_snake_blood_and_ten_max_mp() {
        let mut body = Body::new();
        body.set("이름", "태실봉보혈회귀");
        body.set("체력", 2_000_i64);
        body.set("최고체력", 2_000_i64);
        body.set("내공", 100_i64);
        body.set("최고내공", 100_i64);

        run_luoyang_event(&mut body, "98_대화_대.rhai");
        assert!(!get_user_event(&body, "주엽초").is_empty());
        run_luoyang_event(&mut body, "82_대화_대_현마장.rhai");
        assert!(get_user_event(&body, "주엽초").is_empty());
        assert!(!get_user_event(&body, "주엽초1").is_empty());
        run_luoyang_event(&mut body, "48_대화_대_주엽초_약초.rhai");

        add_test_items(&mut body, "주엽초", 1);
        run_luoyang_event(&mut body, "98_줘_주어_준다_먹여_먹_주엽초.rhai");
        assert!(!body_has_item_spec(&body, "주엽초"));
        assert!(get_user_event(&body, "주엽초1").is_empty());
        assert!(!get_user_event(&body, "주엽초끝").is_empty());

        let room = format!("태실봉보혈회귀-{}", std::process::id());
        let mob_key = format!("낙양성:99-회귀-{}", std::process::id());
        let instance_id = {
            let mut world = crate::world::get_world_state().write().unwrap();
            let data = world
                .mob_cache
                .load_mob("낙양성", "99")
                .expect("two-headed snake fixture")
                .clone();
            assert!(data
                .drop_items
                .iter()
                .any(|item| { item.0 == "주엽초" && item.1 == 1 && item.2 == 99 }));
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let snake =
                MobInstance::new(mob_key.clone(), "낙양성".to_string(), room.clone(), &data);
            let instance_id = snake.instance_id;
            world.mob_cache.add_mob_instance(snake);
            instance_id
        };

        super::try_mob_event(&mut body, "낙양성", &room, "금관쌍두사 보혈 빨아")
            .expect("live snake blood event must be selected");
        assert_eq!(body.get_int("최고내공"), 110);
        assert!(get_user_event(&body, "주엽초끝").is_empty());
        assert!(!get_user_event(&body, "주엽초진짜끝").is_empty());
        assert_eq!(body.act, crate::player::ActState::Stand);
        {
            let world = crate::world::get_world_state().read().unwrap();
            let snake = world
                .mob_cache
                .get_all_mobs_in_room("낙양성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            assert!(!snake.alive);
            assert_eq!(snake.act, 2);
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/태실봉보혈회귀.json");
    }

    #[test]
    fn muguk_cave_centipedes_drop_twenty_max_mp_core_at_source_rates() {
        let mut world = crate::world::get_world_state().write().unwrap();
        let ordinary = world
            .mob_cache
            .load_mob("낙양성", "100")
            .expect("ordinary black centipede fixture");
        assert_eq!(ordinary.combat_type, 1);
        assert_eq!(ordinary.locations.first().map(String::as_str), Some("6014"));
        assert_eq!(ordinary.locations.last().map(String::as_str), Some("6039"));
        assert!(ordinary
            .drop_items
            .iter()
            .any(|item| { item.0 == "독혈내단" && item.1 == 1 && item.2 == 1 }));
        let deep = world
            .mob_cache
            .load_mob("낙양성", "100-1")
            .expect("deep black centipede fixture");
        assert_eq!(deep.combat_type, 1);
        assert_eq!(deep.locations, vec!["6040"]);
        assert!(deep
            .drop_items
            .iter()
            .any(|item| { item.0 == "독혈내단" && item.1 == 1 && item.2 == 5 }));
        drop(world);

        let mut body = Body::new();
        body.set("이름", "독혈내단회귀");
        body.set("내공", 100_i64);
        body.set("최고내공", 100_i64);
        add_test_items(&mut body, "독혈내단", 2);
        let storage = crate::script::ScriptStorage::default();
        storage
            .execute("먹어", &mut body, "독혈내단", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("최고내공"), 120);
        assert!(body_has_item_spec(&body, "독혈내단 1"));
        let _ = std::fs::remove_file("data/user/독혈내단회귀.json");
    }

    #[test]
    fn huashan_swordsman_chain_gives_the_unique_reward_and_marks_completion() {
        let mut body = Body::new();
        body.set("이름", "화산검객회귀");

        run_luoyang_event(&mut body, "9_대화_대_장문령부_화산검객.rhai");
        assert!(body_has_item_spec(&body, "장문령부"));
        assert!(!get_user_event(&body, "장문령부").is_empty());

        let room = format!("화산검객회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("섬서성", "29-1", &room);
        let result = super::try_mob_event(&mut body, "섬서성", &room, "화산검객 장문령부 줘")
            .expect("Huashan swordsman hand-in must select its event");
        let CommandResult::MobEvent { output_lines, .. } = result else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("도룡도")));
        assert!(!body_has_item_spec(&body, "장문령부"));
        assert!(body_has_item_spec(&body, "158"));
        assert!(!get_user_event(&body, "화산검객").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/화산검객회귀.json");
    }

    #[test]
    fn dharma_cave_corpse_search_gives_unique_sword_and_feather_pants() {
        let mut body = Body::new();
        body.set("이름", "달마동회귀");
        let room = format!("달마동회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("낙양성", "86", &room);
        mark_event_mob_corpse("낙양성", &room, instance_id);

        let result = super::try_mob_event(&mut body, "낙양성", &room, "시체 뒤져")
            .expect("Dharma cave corpse search must select its event");
        let CommandResult::MobEvent { output_lines, .. } = result else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("수라도")));
        assert!(body_has_item_spec(&body, "161"));
        assert!(body_has_item_spec(&body, "587"));
        assert!(!get_user_event(&body, "호교법왕").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/달마동회귀.json");
    }

    #[test]
    fn dae_an_cliff_scholar_sets_and_consumes_the_original_event_flag() {
        let mut body = Body::new();
        body.set("이름", "대안황애회귀");
        let room = format!("대안황애회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("호북성", "18", &room);

        let first = super::try_mob_event(&mut body, "호북성", &room, "서생 대화")
            .expect("scholar dialogue must select its event");
        let CommandResult::MobEvent { output_lines, .. } = first else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("무당파")));
        assert!(!get_user_event(&body, "대안황애 이벤트").is_empty());

        let repeat = super::try_mob_event(&mut body, "호북성", &room, "서생 대화")
            .expect("follow-up scholar dialogue must select its event");
        let CommandResult::MobEvent { output_lines, .. } = repeat else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("일조봉")));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/대안황애회귀.json");
    }

    #[test]
    fn jeseok_chamber_statue_requires_a_live_target_and_marks_the_unique_reward() {
        let mut body = Body::new();
        body.set("이름", "제석천실회귀");
        let room = format!("제석천실회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("산서성", "6", &room);

        let result = super::try_mob_event(&mut body, "산서성", &room, "석가여래상 손바닥 눌러")
            .expect("statue hand event must be selected");
        let CommandResult::MobEvent { output_lines, .. } = result else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("제석천옥륜")));
        assert!(body_has_item_spec(&body, "918"));
        assert!(!get_user_event(&body, "석가여래상").is_empty());

        let mut corpse_body = Body::new();
        corpse_body.set("이름", "제석천실시체회귀");
        mark_event_mob_corpse("산서성", &room, instance_id);
        let corpse = super::try_mob_event(&mut corpse_body, "산서성", &room, "시체 손바닥 눌러")
            .expect("corpse statue hand event must be selected");
        let CommandResult::MobEvent { output_lines, .. } = corpse else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("아무런 변화")));
        assert!(!body_has_item_spec(&corpse_body, "918"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/제석천실회귀.json");
        let _ = std::fs::remove_file("data/user/제석천실시체회귀.json");
    }

    #[test]
    fn blood_spirit_cave_altar_awards_its_unique_sword_once_per_player_event() {
        let mut body = Body::new();
        body.set("이름", "혈령동회귀");
        let room = format!("혈령동회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("호북성", "35", &room);

        let result = super::try_mob_event(&mut body, "호북성", &room, "혈석대 뒤져")
            .expect("blood altar search must select its event");
        let CommandResult::MobEvent { output_lines, .. } = result else {
            panic!("unexpected event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("아수라혈령도")));
        assert!(body_has_item_spec(&body, "151"));
        assert!(!get_user_event(&body, "혈석대").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/혈령동회귀.json");
    }

    #[test]
    fn huashan_thief_and_wudang_bandit_keep_their_source_special_drop_tables() {
        let mut world = crate::world::get_world_state().write().unwrap();
        let huashan = world
            .mob_cache
            .load_mob("섬서성", "12-1")
            .expect("Huashan thief fixture");
        assert!(huashan
            .drop_items
            .iter()
            .any(|item| item.0 == "63" && item.1 == 1 && item.2 == 10));
        assert!(huashan
            .drop_items
            .iter()
            .any(|item| item.0 == "63-5" && item.1 == 1 && item.2 == 3));

        let wudang = world
            .mob_cache
            .load_mob("호북성", "12-1")
            .expect("Wudang bandit fixture");
        assert!(wudang
            .drop_items
            .iter()
            .any(|item| item.0 == "152" && item.1 == 1 && item.2 == 10));
        assert!(wudang
            .drop_items
            .iter()
            .any(|item| item.0 == "152-5" && item.1 == 1 && item.2 == 5));

        let desert_bandit = world
            .mob_cache
            .load_mob("낙양성", "70-1")
            .expect("desert bandit fixture");
        assert_eq!(desert_bandit.locations, vec!["1016"]);
        assert!(desert_bandit
            .drop_items
            .iter()
            .any(|item| item.0 == "338" && item.1 == 1 && item.2 == 2));
        assert!(desert_bandit
            .drop_items
            .iter()
            .any(|item| item.0 == "338-5" && item.1 == 1 && item.2 == 2));
    }

    #[test]
    fn demon_cult_stele_enters_the_source_first_sanctum_room() {
        let mut body = Body::new();
        body.set("이름", "마교회귀");
        let room = format!("마교회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("섬서성", "37", &room);

        let result = super::try_mob_event(&mut body, "섬서성", &room, "비석 존자 눌러")
            .expect("demon cult stele event must be selected");
        let CommandResult::MobEvent {
            output_lines,
            set_position,
            ..
        } = result
        else {
            panic!("unexpected event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("빨려들어갑니다")));
        assert_eq!(
            set_position,
            Some(("섬서성".to_string(), "457".to_string()))
        );

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/마교회귀.json");
    }

    #[test]
    fn peach_blossom_forest_tree_corpse_and_bird_chain_match_source_transitions() {
        let mut body = Body::new();
        body.set("이름", "도화림회귀");
        let room = format!("도화림회귀-{}", std::process::id());
        let (tree_key, _) = place_event_mob("호북성", "31-2", &room);
        let (python_key, python_id) = place_event_mob("호북성", "31", &room);
        let (bird_key, _) = place_event_mob("호북성", "30", &room);

        super::try_mob_event(&mut body, "호북성", &room, "음양신목 뒤져")
            .expect("yin-yang tree event must be selected");
        assert!(body_has_item_spec(&body, "yak10"));
        assert!(!get_user_event(&body, "음양선도신과").is_empty());

        mark_event_mob_corpse("호북성", &room, python_id);
        let corpse = super::try_mob_event(&mut body, "호북성", &room, "시체 배째")
            .expect("great python corpse event must be selected");
        let CommandResult::MobEvent { output_lines, .. } = corpse else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("음양과")));
        assert!(body_has_item_spec(&body, "음양과"));
        {
            let world = crate::world::get_world_state().read().unwrap();
            let python = world
                .mob_cache
                .get_all_mobs_in_room("호북성", &room)
                .into_iter()
                .find(|mob| mob.instance_id == python_id)
                .unwrap();
            assert_eq!(python.act, 3);
        }

        let bird = super::try_mob_event(&mut body, "호북성", &room, "붕조 음양과 줘")
            .expect("bird feeding event must be selected");
        let CommandResult::MobEvent { set_position, .. } = bird else {
            panic!("unexpected event result");
        };
        assert_eq!(
            set_position,
            Some(("호북성".to_string(), "499".to_string()))
        );
        assert!(!body_has_item_spec(&body, "음양과"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&tree_key);
        world.mob_cache.remove_mob(&python_key);
        world.mob_cache.remove_mob(&bird_key);
        let _ = std::fs::remove_file("data/user/도화림회귀.json");
    }

    #[test]
    fn thousand_wall_cave_entrance_and_true_plate_reward_match_source() {
        if !crate::oneitem::oneitem_get("245").is_empty() {
            return;
        }
        let mut body = Body::new();
        body.set("이름", "천벽동회귀");
        let room = format!("천벽동회귀-{}", std::process::id());
        let (entrance_key, _) = place_event_mob("낙양성", "1-4", &room);
        let (chest_key, _) = place_event_mob("낙양성", "78-2", &room);

        let entrance = super::try_mob_event(&mut body, "낙양성", &room, "비석 돌려")
            .expect("Thousand Wall Cave entrance must select its event");
        let CommandResult::MobEvent { set_position, .. } = entrance else {
            panic!("unexpected event result");
        };
        assert_eq!(
            set_position,
            Some(("낙양성".to_string(), "499".to_string()))
        );

        super::set_user_event(&mut body, "진짜발판", "1");
        let chest = super::try_mob_event(&mut body, "낙양성", &room, "석합 열어")
            .expect("true-plate stone chest must select its event");
        let CommandResult::MobEvent { output_lines, .. } = chest else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("음양쌍두창")));
        assert!(body_has_item_spec(&body, "245"));
        assert!(body_has_item_spec(&body, "자모환"));
        assert!(!get_user_event(&body, "자모환끝").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&entrance_key);
        world.mob_cache.remove_mob(&chest_key);
        let _ = std::fs::remove_file("data/user/천벽동회귀.json");
    }

    #[test]
    fn unganga_cave_evil_relic_scorpion_and_curse_transitions_match_source() {
        let mut body = Body::new();
        body.set("이름", "운강사회귀");
        let room = format!("운강사회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("산서성", "7", &room);

        super::try_mob_event(&mut body, "산서성", &room, "수라제석천존 발등 조사")
            .expect("evil relic foot inspection must select its event");
        assert!(!get_user_event(&body, "수라제석천존").is_empty());

        let scorpion = super::try_mob_event(&mut body, "산서성", &room, "수라제석천존 전갈 잡아")
            .expect("evil relic scorpion capture must select its event");
        let CommandResult::MobEvent { output_lines, .. } = scorpion else {
            panic!("unexpected event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("전갈")));
        assert!(body_has_item_spec(&body, "전갈"));
        assert!(get_user_event(&body, "수라제석천존").is_empty());
        assert!(!get_user_event(&body, "수라제석천존끝").is_empty());

        let curse = super::try_mob_event(&mut body, "산서성", &room, "수라제석천존 뒤져")
            .expect("evil relic search must select its event");
        let CommandResult::MobEvent { set_position, .. } = curse else {
            panic!("unexpected event result");
        };
        assert_eq!(
            set_position,
            Some(("산서성".to_string(), "3116".to_string()))
        );

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/운강사회귀.json");
    }

    #[test]
    fn sacred_tree_keeps_the_source_axe_branch_and_midnight_stone_sword_entrance() {
        let room = format!("신령수회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "89", &room);

        let mut axe_body = Body::new();
        axe_body.set("이름", "신령수도끼회귀");
        add_test_items(&mut axe_body, "275", 1);
        let axe = super::try_mob_event(&mut axe_body, "낙양성", &room, "신령수 베어")
            .expect("sacred tree axe event must be selected");
        let CommandResult::MobEvent { set_position, .. } = axe else {
            panic!("unexpected event result");
        };
        assert_eq!(set_position, None);
        assert!(!body_has_item_spec(&axe_body, "275"));

        let mut sword_body = Body::new();
        sword_body.set("이름", "신령수검회귀");
        add_test_items(&mut sword_body, "자오석의검", 1);
        let sword = super::try_mob_event(&mut sword_body, "낙양성", &room, "신령수 잘라")
            .expect("sacred tree sword event must be selected");
        let CommandResult::MobEvent { set_position, .. } = sword else {
            panic!("unexpected event result");
        };
        assert_eq!(
            set_position,
            Some(("낙양성".to_string(), "1430".to_string()))
        );

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/신령수도끼회귀.json");
        let _ = std::fs::remove_file("data/user/신령수검회귀.json");
    }

    #[test]
    fn mighty_vajra_grasp_repeat_hint_applies_the_source_hundred_thousand_hp_penalty() {
        let mut body = Body::new();
        body.set("이름", "대력금나수회귀");
        body.set("체력", 120_000_i64);
        super::set_user_event(&mut body, "육룡초면", "1");
        let (lines, _) = run_luoyang_event(&mut body, "기타맨_대화_대_대력금나수.rhai");
        assert!(lines.iter().any(|line| line.contains("100000")));
        assert_eq!(body.get_int("체력"), 20_000);
        let _ = std::fs::remove_file("data/user/대력금나수회귀.json");
    }

    #[test]
    fn mighty_vajra_grasp_final_ruler_phrase_awards_sword_and_clears_source_clues() {
        let mut body = Body::new();
        body.set("이름", "대력금나수완료회귀");
        for key in ["육룡초면", "장천독단", "대엽기행", "군주대면"] {
            super::set_user_event(&mut body, key, "1");
        }
        let (lines, _) = run_zone_event(&mut body, "육룡신전", "심슨_대화_대_만사형통.rhai", None);
        assert!(lines.iter().any(|line| line.contains("무혈정검")));
        assert!(body_has_item_spec(&body, "무혈정검"));
        assert!(!get_user_event(&body, "대력금나수끝").is_empty());
        for key in ["육룡초면", "장천독단", "대엽기행", "군주대면"] {
            assert!(get_user_event(&body, key).is_empty());
        }
        let _ = std::fs::remove_file("data/user/대력금나수완료회귀.json");
    }

    #[test]
    fn dark_cliff_turning_repeat_and_failure_penalties_apply_source_hp_damage() {
        for event in ["전암전회끝", "전암조건"] {
            let mut body = Body::new();
            body.set("이름", format!("전암전회회귀{event}"));
            body.set("체력", 120_000_i64);
            super::set_user_event(&mut body, event, "1");
            let (lines, _) = run_luoyang_event(&mut body, "기타맨_대화_대_전암전회.rhai");
            assert!(lines.iter().any(|line| line.contains("100000")));
            assert_eq!(body.get_int("체력"), 20_000);
            let _ = std::fs::remove_file(format!("data/user/전암전회회귀{event}.json"));
        }
    }

    #[test]
    fn dark_cliff_turning_stone_coffin_awards_blood_tear_blade_and_returns_home() {
        let mut body = Body::new();
        body.set("이름", "전암전회완료회귀");
        super::set_user_event(&mut body, "엽기비동", "1");
        let (lines, position) = run_zone_event(&mut body, "육룡신전", "석관_절_삼배.rhai", None);
        assert!(lines.iter().any(|line| line.contains("혈루인")));
        assert!(body_has_item_spec(&body, "혈루인"));
        assert!(get_user_event(&body, "엽기비동").is_empty());
        assert!(!get_user_event(&body, "전암전회끝").is_empty());
        assert_eq!(position, Some(("낙양성".to_string(), "1".to_string())));
        let _ = std::fs::remove_file("data/user/전암전회완료회귀.json");
    }

    #[test]
    fn divine_spear_secret_requires_all_source_items_and_awards_both_rewards() {
        let mut body = Body::new();
        body.set("이름", "차기미기회귀");
        super::set_user_event(&mut body, "차기미기", "1");
        body.set("체력", 120_000_i64);
        let (failure, _) = run_luoyang_event(&mut body, "기타맨_대화_대_차기미기.rhai");
        assert!(failure.iter().any(|line| line.contains("독혈내단이 없다")));
        assert_eq!(body.get_int("체력"), 20_000);

        body.set("체력", 120_000_i64);
        for item in ["독혈내단", "1050", "893", "824", "289"] {
            add_test_items(&mut body, item, 1);
        }
        let (success, _) = run_luoyang_event(&mut body, "기타맨_대화_대_차기미기.rhai");
        assert!(success.iter().any(|line| line.contains("창룡뇌격극")));
        assert!(body_has_item_spec(&body, "창룡뇌격극"));
        assert!(body_has_item_spec(&body, "합성10"));
        assert!(!get_user_event(&body, "차기미기끝").is_empty());
        for item in ["독혈내단", "1050", "893", "824", "289"] {
            assert!(!body_has_item_spec(&body, item));
        }
        let _ = std::fs::remove_file("data/user/차기미기회귀.json");
    }

    #[test]
    fn five_elements_cycle_chief_reward_and_gate_use_match_source_chain() {
        let mut body = Body::new();
        body.set("이름", "오행연환회귀");
        for key in ["토령문", "목령관", "오행관주"] {
            super::set_user_event(&mut body, key, "1");
        }
        let (chief_lines, _) = run_zone_event(&mut body, "산동성", "57__소멸이벤트_.rhai", None);
        assert!(chief_lines
            .iter()
            .any(|line| line.contains("오색으로 빛나는 열쇠")));
        assert!(body_has_item_spec(&body, "오행연환시"));
        for item in ["615", "558", "913", "693"] {
            assert!(body_has_item_spec(&body, item));
        }
        for key in ["토령문", "목령관", "오행관주"] {
            assert!(get_user_event(&body, key).is_empty());
        }
        assert!(!get_user_event(&body, "오행관").is_empty());

        let (gate_lines, position) = run_zone_event(
            &mut body,
            "산동성",
            "74_사용_돌려_돌_오행연환시_오행_연환시.rhai",
            None,
        );
        assert!(gate_lines
            .iter()
            .any(|line| line.contains("열쇠가 부러지며")));
        assert!(!body_has_item_spec(&body, "오행연환시"));
        for item in ["금", "390", "505", "950", "991"] {
            assert!(body_has_item_spec(&body, item));
        }
        assert!(!get_user_event(&body, "오행문").is_empty());
        assert_eq!(position, Some(("산동성".to_string(), "763".to_string())));
        let _ = std::fs::remove_file("data/user/오행연환회귀.json");
    }

    #[test]
    fn yin_yang_gate_and_yun_chain_keep_source_transitions_and_failure_damage() {
        let mut body = Body::new();
        body.set("이름", "음양무극회귀");

        // 산동성:556 윤대인은 오행연환시를 확인한 뒤 오행관 진행 상태를
        // 끝내고, 학을 통해 음양무극진 입구로 보낸다.
        super::set_user_event(&mut body, "오행관", "1");
        add_test_items(&mut body, "오행연환시", 1);
        let (gate_lines, _) = run_zone_event(&mut body, "산동성", "58_대_대화.rhai", None);
        assert!(gate_lines.iter().any(|line| line.contains("음양무극진")));
        assert!(get_user_event(&body, "오행관").is_empty());
        assert!(!get_user_event(&body, "오행관끝").is_empty());

        let (_, destination) = run_zone_event(
            &mut body,
            "산동성",
            "학_타_탄_사용_올라_올라타_음양무극진_음양무극_음양.rhai",
            None,
        );
        assert_eq!(destination, Some(("산동성".to_string(), "654".to_string())));

        // 오행문 통과 보고는 원본처럼 오행문/오행관끝을 지우고 윤대인 내실로
        // 이동시킨다. 이어지는 대화의 도경 보상도 같은 상태 전이를 유지한다.
        super::set_user_event(&mut body, "오행문", "1");
        let (_, destination) = run_zone_event(&mut body, "산동성", "58_대_대화.rhai", None);
        assert_eq!(destination, Some(("산동성".to_string(), "825".to_string())));
        assert!(get_user_event(&body, "오행문").is_empty());
        assert!(get_user_event(&body, "오행관끝").is_empty());
        assert!(!get_user_event(&body, "혼원1").is_empty());

        super::set_user_event(&mut body, "혼원2", "1");
        let (book_lines, _) = run_zone_event(&mut body, "산동성", "58-1_대_대화.rhai", None);
        assert!(book_lines.iter().any(|line| line.contains("도경")));
        assert!(body_has_item_spec(&body, "도경"));
        assert!(get_user_event(&body, "혼원2").is_empty());
        assert!(!get_user_event(&body, "혼원3").is_empty());

        for event in ["혼원4", "혼원5"] {
            let mut penalty_body = Body::new();
            penalty_body.set("이름", format!("음양무극벌칙회귀{event}"));
            penalty_body.set("체력", 1_200_000_i64);
            super::set_user_event(&mut penalty_body, event, "1");
            let (lines, _) = run_zone_event(&mut penalty_body, "산동성", "58-1_대_대화.rhai", None);
            assert!(lines.iter().any(|line| line.contains("1000000")));
            assert_eq!(penalty_body.get_int("체력"), 200_000, "{event}");
            let _ = std::fs::remove_file(format!("data/user/음양무극벌칙회귀{event}.json"));
        }
        let _ = std::fs::remove_file("data/user/음양무극회귀.json");
    }

    #[test]
    fn primal_cycle_elder_combines_both_books_and_unlocks_the_source_route() {
        let mut body = Body::new();
        body.set("이름", "혼원영겁회귀");
        super::set_user_event(&mut body, "혼원", "1");
        add_test_items(&mut body, "도경", 1);
        add_test_items(&mut body, "덕경", 1);

        let mut data = RawMobData::new();
        data.zone = "산동성".to_string();
        let initial = do_event_rhai(
            &mut body,
            &data,
            "test",
            &[],
            "test",
            "노인_대_대화.rhai",
            None,
        );
        let CommandResult::MobEventEnter { resume_func, .. } = initial else {
            panic!("elder dialogue must suspend for the first enter key");
        };
        assert_eq!(resume_func.as_deref(), Some("step1"));

        for resume in ["step1", "step2"] {
            let result = do_event_rhai(
                &mut body,
                &data,
                "test",
                &[],
                "test",
                "노인_대_대화.rhai",
                Some(resume.to_string()),
            );
            let CommandResult::MobEventEnter { resume_func, .. } = result else {
                panic!("elder dialogue must suspend at {resume}");
            };
            assert!(resume_func.is_some());
        }

        let (lines, destination) =
            run_zone_event(&mut body, "산동성", "노인_대_대화.rhai", Some("step3"));
        assert!(lines.iter().any(|line| line.contains("한권의 책")));
        assert!(body_has_item_spec(&body, "도덕경"));
        assert!(!body_has_item_spec(&body, "도경"));
        assert!(!body_has_item_spec(&body, "덕경"));
        assert!(get_user_event(&body, "혼원").is_empty());
        assert!(!get_user_event(&body, "혼원끝").is_empty());
        assert_eq!(destination, Some(("산동성".to_string(), "556".to_string())));

        let (_, destination) = run_zone_event(
            &mut body,
            "산동성",
            "학_타_탄_사용_올라_올라타_혼원영겁진_혼원영겁_혼원.rhai",
            None,
        );
        assert_eq!(destination, Some(("산동성".to_string(), "563".to_string())));

        let (lines, destination) = run_zone_event(&mut body, "산동성", "74_열어_열.rhai", None);
        assert!(lines
            .iter()
            .any(|line| line.contains("꿈쩍도 하지 않습니다")));
        assert_eq!(destination, None);
        let _ = std::fs::remove_file("data/user/혼원영겁회귀.json");
    }

    #[test]
    fn ascension_scripts_restore_source_mp_status_rank_and_celestial_reward() {
        let rank_path = std::path::Path::new("data/config/rank.json");
        let saved_rank_file = std::fs::read(rank_path).ok();
        crate::world::rank::rank_clear("우화등선");

        let mut body = Body::new();
        body.set("이름", "우화등선회귀");
        body.set("성격", "정파");
        body.set("최고내공", 100_i64);
        super::set_user_event(&mut body, "소오강호진짜끝", "1");
        super::set_user_event(&mut body, "선인", "1");

        let (lines, destination) = run_zone_event(&mut body, "선인", "노자_대_대화.rhai", None);
        assert!(lines.iter().any(|line| line.contains("雲鶴靈靈大丸丹")));
        assert_eq!(body.get_int("최고내공"), 400);
        assert_eq!(body.get_string("기존성격"), "정파");
        assert_eq!(body.get_string("성격"), "선인");
        assert!(!get_user_event(&body, "우화등선끝").is_empty());
        assert!(get_user_event(&body, "선인").is_empty());
        assert_eq!(destination, Some(("선인".to_string(), "223".to_string())));
        assert_eq!(crate::world::rank::rank_read("우화등선", "우화등선회귀"), 1);

        let (reward_lines, _) = run_zone_event(&mut body, "선인", "옥황상제_대_대화.rhai", None);
        assert!(reward_lines.iter().any(|line| line.contains("우화등선")));
        for item in [
            "선인투구",
            "선인머리",
            "선인어깨",
            "선인상의",
            "선인하의",
            "선인장신구",
            "선인갑옷",
            "선인허리",
            "선인장갑",
            "선인반지",
            "선인목걸이",
            "선인귀걸이",
            "선인팔찌",
            "선인슬호",
            "선인신발",
        ] {
            assert!(body_has_item_spec(&body, item), "missing {item}");
        }
        assert!(!get_user_event(&body, "선인끝").is_empty());
        assert!(get_user_event(&body, "우화등선끝").is_empty());

        crate::world::rank::rank_clear("우화등선");
        if let Some(contents) = saved_rank_file {
            std::fs::write(rank_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(rank_path);
        }
        let _ = std::fs::remove_file("data/user/우화등선회귀.json");
    }

    #[test]
    fn ascension_and_celestial_reward_use_actual_source_mobs_and_rooms() {
        let sage_room = format!("우화노진인회귀-{}", std::process::id());
        let emperor_room = format!("우화옥황회귀-{}", std::process::id());
        let (sage_key, _) = place_event_mob("선인", "노자", &sage_room);
        let (emperor_key, _) = place_event_mob("선인", "옥황상제", &emperor_room);
        let mut body = Body::new();
        body.set("이름", "우화실몹회귀");
        body.set("성격", "정파");
        body.set("최고내공", 100_i64);
        super::set_user_event(&mut body, "소오강호진짜끝", "1");
        super::set_user_event(&mut body, "선인", "1");

        let ascension = super::try_mob_event(&mut body, "선인", &sage_room, "노진인 대화")
            .expect("actual Laozi mob must accept ascension dialogue");
        match ascension {
            CommandResult::MobEvent { set_position, .. } => {
                assert_eq!(set_position, Some(("선인".to_string(), "223".to_string())))
            }
            other => panic!("source ascension returned {other:?}"),
        }
        assert_eq!(body.get_int("최고내공"), 400);
        assert_eq!(body.get_string("성격"), "선인");
        assert!(!get_user_event(&body, "우화등선끝").is_empty());

        super::try_mob_event(&mut body, "선인", &emperor_room, "옥황상제 대화")
            .expect("actual Jade Emperor mob must grant celestial reward");
        assert!(body_has_item_spec(&body, "선인투구"));
        assert!(body_has_item_spec(&body, "선인신발"));
        assert!(get_user_event(&body, "우화등선끝").is_empty());
        assert!(!get_user_event(&body, "선인끝").is_empty());

        for (room, mob) in [("223", "옥황상제"), ("224", "노자")] {
            let path = format!("data/map/선인/{room}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(mob)));
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&sage_key);
        world.mob_cache.remove_mob(&emperor_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/우화실몹회귀.json");
    }

    #[test]
    fn celestial_tower_records_unlock_cloud_destinations_in_source_order() {
        let rank_path = std::path::Path::new("data/config/rank.json");
        let saved_rank_file = std::fs::read(rank_path).ok();
        crate::world::rank::rank_clear("무적선인");
        let mut body = Body::new();
        body.set("이름", "선인탑회귀");

        let (_, denied) = run_zone_event(&mut body, "선인", "구름_출발_출_1관문_일관문.rhai", None);
        assert_eq!(denied, None);

        let stages = [
            ("1관문_통과기록.rhai", "2층끝", None),
            ("2관문_통과기록.rhai", "4층끝", Some("2층끝")),
            ("3관문_통과기록.rhai", "6층끝", Some("4층끝")),
            ("4관문_통과기록.rhai", "8층끝", Some("6층끝")),
            ("5관문_통과기록.rhai", "10층끝", Some("8층끝")),
        ];
        for (script, added, removed) in stages {
            run_zone_event(&mut body, "선인", script, None);
            assert!(!get_user_event(&body, added).is_empty(), "{script}");
            if let Some(removed) = removed {
                assert!(get_user_event(&body, removed).is_empty(), "{script}");
            }
        }

        let cloud_routes = [
            ("구름_출발_출_1관문_일관문.rhai", "424"),
            ("구름_출발_출_2관문_이관문.rhai", "381"),
            ("구름_출발_출_3관문_삼관문.rhai", "409"),
            ("구름_출발_출_4관문_사관문.rhai", "391"),
            ("구름_출발_출_5관문_오관문.rhai", "353"),
        ];
        for (script, room) in cloud_routes {
            let (_, destination) = run_zone_event(&mut body, "선인", script, None);
            assert_eq!(destination, Some(("선인".to_string(), room.to_string())));
        }

        super::set_user_event(&mut body, "반고선택", "1");
        let (lines, _) = run_zone_event(&mut body, "선인", "반고_대_대화.rhai", None);
        assert!(lines.iter().any(|line| line.contains("진정한 선인이")));
        assert!(get_user_event(&body, "반고선택").is_empty());
        assert!(get_user_event(&body, "10층끝").is_empty());
        assert!(!get_user_event(&body, "선인탑끝").is_empty());
        assert_eq!(crate::world::rank::rank_read("무적선인", "선인탑회귀"), 1);

        crate::world::rank::rank_clear("무적선인");
        if let Some(contents) = saved_rank_file {
            std::fs::write(rank_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(rank_path);
        }
        let _ = std::fs::remove_file("data/user/선인탑회귀.json");
    }

    #[test]
    fn hundred_floor_tower_boss_rewards_restore_source_mp_and_repeat_routes() {
        let mut body = Body::new();
        body.set("이름", "백층탑보상회귀");
        body.set("최고내공", 100_i64);
        for (script, gained, set, removed) in [
            ("20__소멸이벤트_.rhai", 20, "백층탑20", None),
            ("40__소멸이벤트_.rhai", 30, "백층탑40", Some("백층탑20")),
            ("60__소멸이벤트_.rhai", 40, "백층탑60", Some("백층탑40")),
            ("80__소멸이벤트_.rhai", 50, "백층탑80", Some("백층탑60")),
            ("100__소멸이벤트_.rhai", 60, "백층탑100", Some("백층탑80")),
        ] {
            run_zone_event(&mut body, "백층탑", script, None);
            assert!(body.get_int("최고내공") >= 100 + gained);
            assert!(!get_user_event(&body, set).is_empty());
            if let Some(removed) = removed {
                assert!(get_user_event(&body, removed).is_empty());
            }
        }
        assert_eq!(body.get_int("최고내공"), 300);
        let (_, destination) = run_zone_event(&mut body, "백층탑", "100__소멸이벤트_.rhai", None);
        assert_eq!(
            destination,
            Some(("백층탑".to_string(), "3000".to_string()))
        );
        let _ = std::fs::remove_file("data/user/백층탑보상회귀.json");
    }

    #[test]
    fn hundred_floor_tower_all_death_scripts_keep_source_progression_routes() {
        for floor in 1..=100 {
            let script = format!("{floor}__소멸이벤트_.rhai");
            let mut body = Body::new();
            body.set("이름", format!("백층탑{floor}층경로회귀"));
            let expected_room = match floor {
                50 => "3001".to_string(),
                70 => "3002".to_string(),
                86 => "287".to_string(),
                87 => "286".to_string(),
                90 => "3003".to_string(),
                100 => "3000".to_string(),
                _ => (floor + 200).to_string(),
            };
            if [20, 40, 60, 80, 100].contains(&floor) {
                super::set_user_event(&mut body, &format!("백층탑{floor}"), "1");
            }
            let (_, destination) = run_zone_event(&mut body, "백층탑", &script, None);
            assert_eq!(
                destination,
                Some(("백층탑".to_string(), expected_room.clone())),
                "floor {floor}"
            );
            let path = format!("data/map/백층탑/{expected_room}.json");
            assert!(
                std::path::Path::new(&path).exists(),
                "floor {floor}: {path}"
            );
            let _ = std::fs::remove_file(format!("data/user/백층탑{floor}층경로회귀.json"));
        }
    }

    #[test]
    fn hundred_floor_tower_cremation_restores_source_regen_after_corpse_reward() {
        let cremation_scripts = std::fs::read_dir("data/script/백층탑")
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("_부셔_부수_부숴_태워_삼매진화.rhai"))
            })
            .collect::<Vec<_>>();
        assert_eq!(cremation_scripts.len(), 100);
        for path in cremation_scripts {
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(
                source.contains("if !selected_mob_is_corpse() { end_event(); }"),
                "{} must retain the source corpse gate",
                path.display()
            );
        }
        let room = format!("백층탑소각회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("백층탑", "20", &room);
        let mut body = Body::new();
        body.set("이름", "백층탑소각회귀");
        super::try_mob_event(&mut body, "백층탑", &room, "주천성룡 태워")
            .expect("living tower boss must still select the source command");
        assert!(
            !body_has_item_spec(&body, "1070"),
            "source cremation must not reward a living mob"
        );
        mark_event_mob_corpse("백층탑", &room, instance_id);
        super::try_mob_event(&mut body, "백층탑", &room, "주천성룡 태워")
            .expect("source cremation command must select tower boss corpse");
        assert!(body_has_item_spec(&body, "1070"));
        let world = crate::world::get_world_state().read().unwrap();
        let room_mobs = world.mob_cache.get_all_mobs_in_room("백층탑", &room);
        let mob = room_mobs
            .iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert_eq!(mob.act, 3, "source cremation must move corpse to regen");
        drop(world);
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/백층탑소각회귀.json");
    }

    #[test]
    fn colored_gyunjadan_artisan_chain_consumes_each_source_pair_in_order() {
        let room = format!("견자단장인회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("하북성", "장인", &room);
        let mut body = Body::new();
        body.set("이름", "견자단장인회귀");
        super::set_user_event(&mut body, "마룡혈부끝", "1");

        for (volcano, stone, prior, reward, next_state) in [
            ("화산2", "화룡석", "금린", "적린", "적린"),
            ("화산3", "수룡석", "적린", "청린", "청린"),
            ("화산4", "월광석", "청린", "백린", "분화구"),
        ] {
            super::set_user_event(&mut body, volcano, "1");
            add_test_items(&mut body, stone, 1);
            add_test_items(&mut body, prior, 1);
            super::try_mob_event(&mut body, "하북성", &room, "장인 대화")
                .expect("artisan must accept source material handoff");
            assert!(
                !body_has_item_spec(&body, stone),
                "{stone} must be consumed"
            );
            assert!(
                !body_has_item_spec(&body, prior),
                "{prior} must be consumed"
            );
            assert!(get_user_event(&body, volcano).is_empty());

            super::try_mob_event(&mut body, "하북성", &room, "장인 대화")
                .expect("artisan must complete source reward handoff");
            assert!(body_has_item_spec(&body, reward), "missing {reward}");
            assert!(!get_user_event(&body, next_state).is_empty());
            super::set_user_event(&mut body, next_state, "");
            body.object.objs.clear();
        }
        let path = "data/map/하북성/93.json";
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(json["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("장인")));
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/견자단장인회귀.json");
    }

    #[test]
    fn first_volcano_craft_keeps_true_silver_variant_and_returns_false_variant() {
        for (name, silver, expected_after_first) in [
            ("견자단진품회귀", "은린3", "황룡석"),
            ("견자단가짜회귀", "견자단", "황룡석1"),
        ] {
            let room = format!("{name}-{}", std::process::id());
            let (mob_key, _) = place_event_mob("낙양성", "이름맨", &room);
            let mut body = Body::new();
            body.set("이름", name);
            super::set_user_event(&mut body, "화산1", "1");
            add_test_items(&mut body, "황룡석", 1);
            add_test_items(&mut body, silver, 1);

            super::try_mob_event(&mut body, "낙양성", &room, "크래프트 대화")
                .expect("craft must accept first volcano materials");
            assert!(!body_has_item_spec(&body, "황룡석"));
            assert!(!get_user_event(&body, expected_after_first).is_empty());
            if silver == "은린3" {
                assert!(body_has_item_spec(&body, "은린3"));
                assert!(get_user_event(&body, "돌산").is_empty());
            } else {
                assert!(!body_has_item_spec(&body, "견자단"));
                super::try_mob_event(&mut body, "낙양성", &room, "크래프트 대화")
                    .expect("false variant must be returned by source follow-up");
                assert!(body_has_item_spec(&body, "견자단"));
                assert!(body_has_item_spec(&body, "황룡석"));
            }
            if silver == "은린3" {
                super::try_mob_event(&mut body, "낙양성", &room, "크래프트 대화")
                    .expect("true variant must produce gold craft reward");
                assert!(body_has_item_spec(&body, "금린"));
                assert!(!get_user_event(&body, "금린").is_empty());
                assert!(get_user_event(&body, "화산1").is_empty());
            }
            let mut world = crate::world::get_world_state().write().unwrap();
            world.mob_cache.remove_mob(&mob_key);
            drop(world);
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }
    }

    #[test]
    fn first_colored_gyunjadan_volcano_awards_source_yellow_dragon_stone() {
        let room = format!("견자단화산일회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "화산1", &room);
        let mut body = Body::new();
        body.set("이름", "견자단화산일회귀");
        add_test_items(&mut body, "바위4", 1);
        super::try_mob_event(&mut body, "낙양성", &room, "분화구 바위산 막아")
            .expect("first source volcano must accept the mountain-sized rock");
        assert!(!body_has_item_spec(&body, "바위4"));
        assert!(body_has_item_spec(&body, "황룡석"));
        assert!(!get_user_event(&body, "화산1").is_empty());

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string("data/map/낙양성/3001.json").unwrap())
                .unwrap();
        assert!(json["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("화산1")));
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/견자단화산일회귀.json");
    }

    #[test]
    fn colored_gyunjadan_volcanoes_consume_required_stones_and_record_final_rescue() {
        let rank_path = std::path::Path::new("data/config/rank.json");
        let saved_rank_file = std::fs::read(rank_path).ok();
        crate::world::rank::rank_clear("화산");
        for (zone, source_key, command, required_state, stone, reward, completed_state) in [
            (
                "감숙성",
                "화산2",
                "분화구 바위 막아",
                "금린",
                "바위2",
                "화룡석",
                "화산2",
            ),
            (
                "동정호",
                "화산3",
                "분화구 큰바위 막아",
                "적린",
                "바위3",
                "수룡석",
                "화산3",
            ),
            (
                "운남성",
                "화산4",
                "분화구 바위산 막아",
                "청린",
                "바위4",
                "월광석",
                "화산4",
            ),
        ] {
            let name = format!("견자단{source_key}회귀");
            let room = format!("{name}-{}", std::process::id());
            let (mob_key, _) = place_event_mob(zone, source_key, &room);
            let mut body = Body::new();
            body.set("이름", name.clone());
            super::set_user_event(&mut body, required_state, "1");
            add_test_items(&mut body, stone, 1);
            super::try_mob_event(&mut body, zone, &room, command)
                .expect("source volcano command must select its crater");
            assert!(!body_has_item_spec(&body, stone));
            assert!(body_has_item_spec(&body, reward));
            assert!(!get_user_event(&body, completed_state).is_empty());
            if source_key == "화산4" {
                assert_eq!(crate::world::rank::rank_read("화산", &name), 1);
            }
            let mut world = crate::world::get_world_state().write().unwrap();
            world.mob_cache.remove_mob(&mob_key);
            drop(world);
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }
        crate::world::rank::rank_clear("화산");
        if let Some(contents) = saved_rank_file {
            std::fs::write(rank_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(rank_path);
        }
    }

    #[test]
    fn bloody_tear_pickup_keeps_source_hundred_thousand_hp_penalty() {
        let room = format!("혈루인회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("육룡신전", "혈루인", &room);
        let mut body = Body::new();
        body.set("이름", "혈루인회귀");
        body.set("체력", 150000_i64);
        super::try_mob_event(&mut body, "육룡신전", &room, "혈루인 가져")
            .expect("source bloody-tear pickup command must select the mob");
        assert_eq!(body.get_int("체력"), 50000);
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/혈루인회귀.json");
    }

    #[test]
    fn immortal_valley_riddle_awards_source_unique_or_fallback_and_punishes_wrong_answer() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();

        for (name, unique_exists, expected_item) in [
            ("불혼곡진짜회귀", false, "해왕조"),
            ("불혼곡가짜회귀", true, "해왕조-5"),
        ] {
            crate::oneitem::oneitem_clear();
            if unique_exists {
                assert!(crate::oneitem::oneitem_have("해왕조", "먼저온사람"));
            }
            let mut body = Body::new();
            body.set("이름", name);
            super::set_user_event(&mut body, "불혼곡", "1");
            let mut data = RawMobData::new();
            data.zone = "산서성".to_string();
            let result = do_event_rhai(
                &mut body,
                &data,
                "test",
                &["불혼곡주".into(), "242".into(), "답".into()],
                "test",
                "46_답.rhai",
                None,
            );
            let CommandResult::MobEvent { output_lines, .. } = result else {
                panic!("riddle answer did not complete");
            };
            assert!(output_lines.iter().any(|line| line.contains("242개")));
            assert!(body_has_item_spec(&body, expected_item));
            assert!(get_user_event(&body, "불혼곡").is_empty());
            assert!(!get_user_event(&body, "불혼곡끝").is_empty());
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }

        let mut body = Body::new();
        body.set("이름", "불혼곡오답회귀");
        super::set_user_event(&mut body, "불혼곡", "1");
        let mut data = RawMobData::new();
        data.zone = "산서성".to_string();
        let result = do_event_rhai(
            &mut body,
            &data,
            "test",
            &["불혼곡주".into(), "241".into(), "답".into()],
            "test",
            "46_답.rhai",
            None,
        );
        let CommandResult::MobEvent { set_position, .. } = result else {
            panic!("wrong riddle answer did not complete");
        };
        assert_eq!(
            set_position,
            Some(("낙양성".to_string(), "혼돈의방".to_string()))
        );

        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
        let _ = std::fs::remove_file("data/user/불혼곡오답회귀.json");
    }

    #[test]
    fn dragon_tomb_chain_consumes_source_materials_and_starts_fight_on_failure() {
        let mut body = Body::new();
        body.set("이름", "용의무덤회귀");
        add_test_items(&mut body, "환룡석", 1);

        let (_, destination) = run_zone_event(&mut body, "광서성", "7_대화_대.rhai", None);
        assert_eq!(destination, Some(("광서성".to_string(), "100".to_string())));
        assert!(!get_user_event(&body, "용의무덤").is_empty());

        add_test_items(&mut body, "금강마강시", 5);
        let (lines, destination) = run_zone_event(&mut body, "광서성", "6_주문_주술.rhai", None);
        assert!(lines.iter().any(|line| line.contains("비학천룡")));
        assert_eq!(destination, Some(("광서성".to_string(), "100".to_string())));
        assert!(!body_has_item_spec(&body, "환룡석"));
        assert!(!body_has_item_spec(&body, "금강마강시"));
        assert!(body_has_item_spec(&body, "비학천룡"));

        run_zone_event(&mut body, "광서성", "7_대화_대.rhai", None);
        assert!(get_user_event(&body, "용의무덤").is_empty());
        assert!(!get_user_event(&body, "용끝").is_empty());

        let (_, destination) = run_zone_event(&mut body, "광서성", "7_대화_대.rhai", None);
        assert_eq!(destination, Some(("광서성".to_string(), "100".to_string())));

        let room = format!("용의무덤전투회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("광서성", "6", &room);
        let mut fighter = Body::new();
        fighter.set("이름", "용의무덤전투회귀");
        super::try_mob_event(&mut fighter, "광서성", &room, "비학천룡 주문")
            .expect("dragon command must select the placed dragon");
        assert_eq!(fighter.act, crate::player::ActState::Fight);
        assert_eq!(
            crate::script::combat_commands::combat_target_instance_ids(&fighter),
            vec![instance_id]
        );
        crate::script::combat_commands::remove_combat_target_instance_id(&mut fighter, instance_id);
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);

        let _ = std::fs::remove_file("data/user/용의무덤회귀.json");
        let _ = std::fs::remove_file("data/user/용의무덤전투회귀.json");
    }

    #[test]
    fn blood_tower_scripts_keep_source_unique_rewards_corpse_state_and_hp_penalties() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();

        for (name, occupied, expected) in [
            ("검후용천진짜회귀", false, "80"),
            ("검후용천가짜회귀", true, "80-5"),
        ] {
            crate::oneitem::oneitem_clear();
            if occupied {
                assert!(crate::oneitem::oneitem_have("80", "먼저온사람"));
            }
            let mut body = Body::new();
            body.set("이름", name);
            let (lines, _) =
                run_zone_event(&mut body, "절강성", "검후_대_대화.rhai", Some("step3"));
            assert!(lines.iter().any(|line| line.contains("용천검")));
            assert!(body_has_item_spec(&body, expected));
            assert!(!get_user_event(&body, "검후무기").is_empty());
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }

        for (name, script, resume) in [("검후처벌회귀", "검후_대_대화.rhai", "step6")] {
            let mut body = Body::new();
            body.set("이름", name);
            body.set("체력", 250_000_i64);
            let (lines, _) = run_zone_event(&mut body, "절강성", script, Some(resume));
            assert!(lines.iter().any(|line| line.contains("200000")), "{script}");
            assert_eq!(body.get_int("체력"), 50_000, "{script}");
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }

        // 적수염마의 검후공격은 용천검(진품/가품)과 무극검을 모두 요구한다.
        // 어느 하나라도 없으면 원본처럼 즉시 200,000의 체력이 감소한다.
        let mut missing_requirement = Body::new();
        missing_requirement.set("이름", "염마조건부족회귀");
        missing_requirement.set("체력", 250_000_i64);
        super::set_user_event(&mut missing_requirement, "검후공격", "1");
        let (lines, _) = run_zone_event(
            &mut missing_requirement,
            "절강성",
            "적수염마_대_대화.rhai",
            None,
        );
        assert!(lines.iter().any(|line| line.contains("200000")));
        assert_eq!(missing_requirement.get_int("체력"), 50_000);

        let mut winner = Body::new();
        winner.set("이름", "염마검후승리회귀");
        add_test_items(&mut winner, "80", 1);
        winner.skill_list.push("무극검".to_string());
        super::set_user_event(&mut winner, "검후공격", "1");
        let mut data = RawMobData::new();
        data.zone = "절강성".to_string();
        let started = do_event_rhai(
            &mut winner,
            &data,
            "test",
            &[],
            "test",
            "적수염마_대_대화.rhai",
            None,
        );
        let CommandResult::MobEventEnter { resume_func, .. } = started else {
            panic!("source-valid demon challenge must wait for enter");
        };
        assert_eq!(resume_func.as_deref(), Some("attack_step1"));
        let middle = do_event_rhai(
            &mut winner,
            &data,
            "test",
            &[],
            "test",
            "적수염마_대_대화.rhai",
            Some("attack_step1".to_string()),
        );
        let CommandResult::MobEventEnter { resume_func, .. } = middle else {
            panic!("source-valid demon challenge must wait twice");
        };
        assert_eq!(resume_func.as_deref(), Some("attack_step2"));
        run_zone_event(
            &mut winner,
            "절강성",
            "적수염마_대_대화.rhai",
            Some("attack_step2"),
        );
        assert!(!get_user_event(&winner, "검후끝").is_empty());
        assert!(get_user_event(&winner, "검후공격").is_empty());
        assert_eq!(
            winner.temp().get("_event_selected_mob_set_corpse"),
            Some(&crate::object::Value::Int(1))
        );

        let mut choice = Body::new();
        choice.set("이름", "염마거절회귀");
        choice.set("체력", 250_000_i64);
        super::set_user_event(&mut choice, "적수염마아니", "1");
        run_zone_event(&mut choice, "절강성", "적수염마_대_대화_예.rhai", None);
        assert_eq!(choice.get_int("체력"), 50_000);

        let mut corpse = Body::new();
        corpse.set("이름", "염마시체회귀");
        run_zone_event(
            &mut corpse,
            "절강성",
            "적수염마_대_대화.rhai",
            Some("attack_step2"),
        );
        assert_eq!(
            corpse.temp().get("_event_selected_mob_set_corpse"),
            Some(&crate::object::Value::Int(1))
        );

        let mut dialogue = Body::new();
        dialogue.set("이름", "검후대화회귀");
        super::set_user_event(&mut dialogue, "청성장문아니", "1");
        let mut data = RawMobData::new();
        data.zone = "절강성".to_string();
        let started = do_event_rhai(
            &mut dialogue,
            &data,
            "test",
            &[],
            "test",
            "검후_대_대화.rhai",
            None,
        );
        let CommandResult::MobEventEnter { resume_func, .. } = started else {
            panic!("source gatekeeper dialogue must wait for enter");
        };
        assert_eq!(resume_func.as_deref(), Some("step1"));
        let resumed = do_event_rhai(
            &mut dialogue,
            &data,
            "test",
            &[],
            "test",
            "검후_대_대화.rhai",
            Some("step1".to_string()),
        );
        let CommandResult::MobEventEnter { resume_func, .. } = resumed else {
            panic!("source gatekeeper dialogue must continue waiting at step1");
        };
        assert_eq!(resume_func.as_deref(), Some("step2"));

        let mut unrelated = Body::new();
        unrelated.set("이름", "검후무관회귀");
        let (lines, _) = run_zone_event(&mut unrelated, "절강성", "검후_대_대화.rhai", None);
        assert!(lines.iter().any(|line| line.contains("아무나 함부로")));

        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
        let _ = std::fs::remove_file("data/user/염마조건부족회귀.json");
        let _ = std::fs::remove_file("data/user/염마검후승리회귀.json");
        let _ = std::fs::remove_file("data/user/염마거절회귀.json");
        let _ = std::fs::remove_file("data/user/염마시체회귀.json");
        let _ = std::fs::remove_file("data/user/검후대화회귀.json");
        let _ = std::fs::remove_file("data/user/검후무관회귀.json");
    }

    #[test]
    fn blood_tower_gate_records_and_shortcuts_follow_source_state_order() {
        let mut body = Body::new();
        body.set("이름", "혈살관문회귀");

        // 원본은 기록 전에는 어느 관문도 열지 않는다.
        let (lines, destination) =
            run_zone_event(&mut body, "절강성", "혈살관문_통과_혈살초관.rhai", None);
        assert!(lines
            .iter()
            .any(|line| line.contains("꼼짝도 하지 않습니다")));
        assert_eq!(destination, None);

        for (record, added, removed) in [
            ("혈살초관_통과기록.rhai", "혈살무관", None),
            ("혈살무관_통과기록.rhai", "혈살루관", Some("혈살무관")),
            ("혈살루관_통과기록.rhai", "혈살신관", Some("혈살루관")),
            ("혈살비석_통과기록.rhai", "혈살루", Some("혈살신관")),
        ] {
            run_zone_event(&mut body, "절강성", record, None);
            assert!(!get_user_event(&body, added).is_empty(), "{record}");
            if let Some(removed) = removed {
                assert!(get_user_event(&body, removed).is_empty(), "{record}");
            }
        }

        // 완료 뒤에는 원본의 관문 순서와 같은 방으로 곧바로 이동한다.
        for (gate, room) in [
            ("혈살관문_통과_혈살초관.rhai", "76"),
            ("혈살관문_통과_혈살무관.rhai", "105"),
            ("혈살관문_통과_혈살루관.rhai", "130"),
            ("혈살관문_통과_혈살신관.rhai", "314"),
        ] {
            let (_, destination) = run_zone_event(&mut body, "절강성", gate, None);
            assert_eq!(
                destination,
                Some(("절강성".to_string(), room.to_string())),
                "{gate}"
            );
        }

        let _ = std::fs::remove_file("data/user/혈살관문회귀.json");
    }

    #[test]
    fn sword_empress_requires_source_blood_weapon_and_skill_then_awards_tears() {
        let mut data = RawMobData::new();
        data.zone = "절강성".to_string();

        let mut failure = Body::new();
        failure.set("이름", "검후조건부족회귀");
        failure.set("체력", 250_000_i64);
        super::set_user_event(&mut failure, "적수염마무기", "1");
        let started = do_event_rhai(
            &mut failure,
            &data,
            "test",
            &[],
            "test",
            "검후_대_대화.rhai",
            None,
        );
        let CommandResult::MobEventEnter { resume_func, .. } = started else {
            panic!("source-invalid sword empress challenge must wait once");
        };
        assert_eq!(resume_func.as_deref(), Some("blood_failure"));
        run_zone_event(
            &mut failure,
            "절강성",
            "검후_대_대화.rhai",
            Some("blood_failure"),
        );
        assert_eq!(failure.get_int("체력"), 50_000);

        let mut winner = Body::new();
        winner.set("이름", "검후혈세승리회귀");
        add_test_items(&mut winner, "171-5", 1);
        winner.skill_list.push("혈세천하".to_string());
        super::set_user_event(&mut winner, "적수염마무기", "1");
        let started = do_event_rhai(
            &mut winner,
            &data,
            "test",
            &[],
            "test",
            "검후_대_대화.rhai",
            None,
        );
        let CommandResult::MobEventEnter { resume_func, .. } = started else {
            panic!("source-valid sword empress challenge must wait");
        };
        assert_eq!(resume_func.as_deref(), Some("blood_step1"));
        let middle = do_event_rhai(
            &mut winner,
            &data,
            "test",
            &[],
            "test",
            "검후_대_대화.rhai",
            Some("blood_step1".to_string()),
        );
        let CommandResult::MobEventEnter { resume_func, .. } = middle else {
            panic!("source-valid sword empress challenge must wait twice");
        };
        assert_eq!(resume_func.as_deref(), Some("blood_step2"));
        run_zone_event(
            &mut winner,
            "절강성",
            "검후_대_대화.rhai",
            Some("blood_step2"),
        );
        assert!(!get_user_event(&winner, "검후승리").is_empty());
        assert!(get_user_event(&winner, "적수염마무기").is_empty());

        run_zone_event(&mut winner, "절강성", "검후_대_대화.rhai", None);
        assert!(body_has_item_spec(&winner, "검후의눈물"));
        assert!(get_user_event(&winner, "검후승리").is_empty());
        assert!(!get_user_event(&winner, "검후가짜눈물").is_empty());

        let _ = std::fs::remove_file("data/user/검후조건부족회귀.json");
        let _ = std::fs::remove_file("data/user/검후혈세승리회귀.json");
    }

    #[test]
    fn blood_demon_final_exchange_keeps_source_unique_reward_and_completion_state() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();

        for (name, occupied, expected_helmet) in [
            ("염마최종진품회귀", false, true),
            ("염마최종대체회귀", true, false),
        ] {
            crate::oneitem::oneitem_clear();
            if occupied {
                assert!(crate::oneitem::oneitem_have("400", "먼저온사람"));
            }
            let mut body = Body::new();
            body.set("이름", name);
            body.set("최고내공", 10_i64);
            add_test_items(&mut body, "검후의눈물", 1);
            super::set_user_event(&mut body, "검후가짜눈물", "1");
            super::set_user_event(&mut body, "청성장문예", "1");
            let (lines, _) = run_zone_event(&mut body, "절강성", "적수염마_대_대화.rhai", None);
            assert!(lines.iter().any(|line| line.contains("혈마지기")));
            assert_eq!(body.get_int("최고내공"), 60);
            assert!(!body_has_item_spec(&body, "검후의눈물"));
            assert_eq!(body_has_item_spec(&body, "400"), expected_helmet);
            assert!(get_user_event(&body, "검후가짜눈물").is_empty());
            assert!(get_user_event(&body, "청성장문예").is_empty());
            assert!(!get_user_event(&body, "혈살루끝").is_empty());
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }

        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn qingcheng_leader_start_dialogue_and_choices_follow_source_chain() {
        let room = format!("청성장문회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("절강성", "청성장문", &room);
        let mut body = Body::new();
        body.set("이름", "청성장문회귀");

        let first = super::try_mob_event(&mut body, "절강성", &room, "청성장문 대화")
            .expect("actual mob command must find source dialogue event");
        let CommandResult::MobEventEnter {
            resume_func,
            event_key,
            words,
            ..
        } = first
        else {
            panic!("source opening must wait for enter");
        };
        assert_eq!(resume_func.as_deref(), Some("step1"));

        let second = super::try_mob_event_resume(
            &mut body,
            "절강성",
            &room,
            &mob_key,
            &event_key,
            words.clone(),
            0,
            Some("step1".into()),
        )
        .expect("first enter must resume source dialogue");
        let CommandResult::MobEventEnter { resume_func, .. } = second else {
            panic!("source opening must wait twice");
        };
        assert_eq!(resume_func.as_deref(), Some("step2"));
        super::try_mob_event_resume(
            &mut body,
            "절강성",
            &room,
            &mob_key,
            &event_key,
            words,
            0,
            Some("step2".into()),
        )
        .expect("second enter must complete source dialogue");
        assert!(!get_user_event(&body, "청성장문1").is_empty());

        let accepted = super::try_mob_event(&mut body, "절강성", &room, "청성장문 예 대화")
            .expect("actual accept command must select source event");
        let CommandResult::MobEvent { output_lines, .. } = accepted else {
            panic!("accept branch must complete");
        };
        assert!(output_lines.iter().any(|line| line.contains("혈살루는")));
        assert!(get_user_event(&body, "청성장문1").is_empty());
        assert!(!get_user_event(&body, "청성장문예").is_empty());

        super::set_user_event(&mut body, "청성장문예", "");
        super::set_user_event(&mut body, "청성장문1", "1");
        let refused = super::try_mob_event(&mut body, "절강성", &room, "청성장문 아니오 대화")
            .expect("actual refusal command must select source event");
        let CommandResult::MobEvent { output_lines, .. } = refused else {
            panic!("refusal branch must complete");
        };
        assert!(output_lines.iter().any(|line| line.contains("보타암")));
        assert!(get_user_event(&body, "청성장문1").is_empty());
        assert!(!get_user_event(&body, "청성장문아니").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/청성장문회귀.json");
    }

    #[test]
    fn blood_tower_and_potuo_map_data_place_the_source_npcs_and_route_gates() {
        for (room, mob, required_exit) in [
            ("16", "청성장문", "남 15"),
            ("60", "혈살관문", "혈살초관 61"),
            ("347", "적수염마", "아래 358 348"),
            ("489", "검후", "동 490"),
            ("490", "해연신니", "서 489"),
        ] {
            let path = format!("data/map/절강성/{room}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let room_info = &json["맵정보"];
            assert!(
                room_info["몹"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(mob)),
                "{path} must place {mob}"
            );
            assert!(
                room_info["출구"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(required_exit)),
                "{path} must retain source exit {required_exit}"
            );
        }
    }

    #[test]
    fn mara_cave_mingming_uses_source_combat_corpse_and_regen_branches() {
        let room = format!("마라염동주명명회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("귀주성", "22", &room);
        let mut body = Body::new();
        body.set("이름", "마라염동주명명회귀");
        super::set_user_event(&mut body, "오지산끝", "1");

        super::try_mob_event(&mut body, "귀주성", &room, "주명명 목 잘라")
            .expect("source final-state corpse command must select mingming");
        assert_eq!(body.act, crate::player::ActState::Fight);
        assert_eq!(
            crate::script::combat_commands::combat_target_instance_ids(&body),
            vec![instance_id]
        );
        crate::script::combat_commands::remove_combat_target_instance_id(&mut body, instance_id);
        body.act = crate::player::ActState::Stand;

        super::set_user_event(&mut body, "오지산끝", "");
        super::set_user_event(&mut body, "양정봉끝", "1");
        mark_event_mob_corpse("귀주성", &room, instance_id);
        super::try_mob_event(&mut body, "귀주성", &room, "시체 머리 잘라")
            .expect("source corpse command must select mingming corpse");
        assert!(body_has_item_spec(&body, "주명명머리"));
        let world = crate::world::get_world_state().read().unwrap();
        let room_mobs = world.mob_cache.get_all_mobs_in_room("귀주성", &room);
        let mob = room_mobs
            .iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert_eq!(mob.act, 3, "source corpse branch must switch to regen");
        drop(world);
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/마라염동주명명회귀.json");
    }

    #[test]
    fn mara_cave_five_ritual_order_and_final_reward_follow_source_flags() {
        let mut body = Body::new();
        body.set("이름", "마라염동순서회귀");

        // 임자영의 원본 대화는 여섯 번에 걸쳐 수령봉 시작 상태까지 진행된다.
        for _ in 0..6 {
            run_zone_event(&mut body, "귀주성", "23_대화_대.rhai", None);
        }
        assert!(!get_user_event(&body, "수령봉").is_empty());

        for (script, consumed, produced) in [
            ("20_대화_대.rhai", "수령봉", "수령봉끝"),
            ("20-1_대화_대.rhai", "수령봉끝", "검주봉끝"),
            ("20-2_대화_대.rhai", "검주봉끝", "삼화봉끝"),
            ("20-3_대화_대.rhai", "삼화봉끝", "양정봉끝"),
        ] {
            run_zone_event(&mut body, "귀주성", script, None);
            assert!(get_user_event(&body, consumed).is_empty(), "{script}");
            assert!(!get_user_event(&body, produced).is_empty(), "{script}");
        }

        // 양정봉 뒤에는 주명명 시체에서 얻은 머리를 소설령에게 전달해 마지막 봉우리로 간다.
        add_test_items(&mut body, "주명명머리", 1);
        run_zone_event(&mut body, "귀주성", "21_대화_대.rhai", None);
        assert!(!body_has_item_spec(&body, "주명명머리"));
        assert!(get_user_event(&body, "양정봉끝").is_empty());
        assert!(!get_user_event(&body, "대렴봉끝").is_empty());

        let (_, destination) = run_zone_event(&mut body, "귀주성", "23_대화_대.rhai", None);
        assert_eq!(destination, Some(("귀주성".to_string(), "188".to_string())));
        assert!(get_user_event(&body, "대렴봉끝").is_empty());
        assert!(!get_user_event(&body, "오지산끝").is_empty());

        run_zone_event(&mut body, "귀주성", "21_대화_대.rhai", None);
        assert!(body_has_item_spec(&body, "환룡석"));
        assert!(body_has_item_spec(&body, "만년화리의내단"));
        assert!(!get_user_event(&body, "환룡석").is_empty());
        let _ = std::fs::remove_file("data/user/마라염동순서회귀.json");
    }

    #[test]
    fn mara_cave_map_data_places_each_source_ritual_actor() {
        for (room, mob) in [
            ("187", "23"),
            ("188", "21"),
            ("483", "20"),
            ("484", "20-1"),
            ("306", "20-2"),
            ("363", "20-3"),
            ("465", "22"),
        ] {
            let path = format!("data/map/귀주성/{room}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(
                json["맵정보"]["몹"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(mob)),
                "{path} must place source actor {mob}"
            );
        }
    }

    #[test]
    fn pirate_route_black_ogre_requires_corpse_before_head_and_regens_afterward() {
        let room = format!("동정호흑괴회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("동정호", "8", &room);
        let mut body = Body::new();
        body.set("이름", "동정호흑괴회귀");

        super::try_mob_event(&mut body, "동정호", &room, "흑괴 머리 잘라")
            .expect("alive black ogre command must select event mob");
        assert_eq!(body.act, crate::player::ActState::Fight);
        assert!(!body_has_item_spec(&body, "흑괴머리"));
        crate::script::combat_commands::remove_combat_target_instance_id(&mut body, instance_id);
        body.act = crate::player::ActState::Stand;

        mark_event_mob_corpse("동정호", &room, instance_id);
        super::try_mob_event(&mut body, "동정호", &room, "시체 목 잘라")
            .expect("black ogre corpse command must select event mob");
        assert!(body_has_item_spec(&body, "흑괴머리"));
        let world = crate::world::get_world_state().read().unwrap();
        let room_mobs = world.mob_cache.get_all_mobs_in_room("동정호", &room);
        let mob = room_mobs
            .iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert_eq!(mob.act, 3);
        drop(world);
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/동정호흑괴회귀.json");
    }

    #[test]
    fn pirate_route_boat_chain_reaches_source_moon_island_crossing() {
        let mut body = Body::new();
        body.set("이름", "해적선항로회귀");
        add_test_items(&mut body, "649", 1);

        // 강태공의 세 대화는 흑괴 처치 의뢰까지 순서대로 진행한다.
        for expected in ["강우혁", "강우혁1", "흑괴머리"] {
            run_zone_event(&mut body, "동정호", "24_대화_대.rhai", None);
            assert!(!get_user_event(&body, expected).is_empty(), "{expected}");
        }
        add_test_items(&mut body, "흑괴머리", 1);
        run_zone_event(&mut body, "동정호", "24_대화_대.rhai", None);
        assert!(get_user_event(&body, "흑괴머리").is_empty());
        assert!(!get_user_event(&body, "뱃사공").is_empty());

        run_zone_event(&mut body, "동정호", "21_대화_대.rhai", None);
        assert!(!get_user_event(&body, "초선").is_empty());
        run_zone_event(&mut body, "동정호", "21_대화_대.rhai", None);
        assert!(!get_user_event(&body, "순금팔찌").is_empty());
        add_test_items(&mut body, "905", 1);
        run_zone_event(&mut body, "동정호", "19_대화_대.rhai", None);
        assert!(get_user_event(&body, "순금팔찌").is_empty());
        assert!(!get_user_event(&body, "손수건").is_empty());
        assert!(body_has_item_spec(&body, "손수건"));

        run_zone_event(&mut body, "동정호", "21_대화_대.rhai", None);
        assert!(get_user_event(&body, "손수건").is_empty());
        assert!(!get_user_event(&body, "손수건끝").is_empty());
        assert!(!body_has_item_spec(&body, "손수건"));
        let (_, destination) = run_zone_event(&mut body, "동정호", "21_대화_대.rhai", None);
        assert_eq!(destination, Some(("동정호".to_string(), "200".to_string())));

        let crossing: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string("data/map/동정호/200.json").unwrap())
                .unwrap();
        assert!(crossing["맵정보"]["출구"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("동$ 201")));
        let moon_approach: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string("data/map/동정호/206.json").unwrap())
                .unwrap();
        assert!(moon_approach["맵정보"]["출구"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("동$ 286 286 121")));
        let _ = std::fs::remove_file("data/user/해적선항로회귀.json");
    }

    #[test]
    fn cliff_help_label_has_only_guha_mountain_map_topology_in_source_data() {
        for (room, required_exit) in [
            ("394", "남 395"),
            ("399", "서 400"),
            ("403", "남 404"),
            ("407", "남 408"),
            ("521", "서 376"),
            ("525", "남 526"),
            ("529", "남 530"),
            ("532", "남 533"),
            ("534", "서 535"),
            ("535", "남 536"),
        ] {
            let path = format!("data/map/안휘성/{room}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let info = &json["맵정보"];
            assert!(
                info["설명"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|line| line.as_str().unwrap_or_default().contains("낭떠러지")),
                "{path} must retain cliff description"
            );
            assert!(
                info["출구"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|exit| exit.as_str() == Some(required_exit)),
                "{path} must retain source route {required_exit}"
            );
        }
    }

    #[test]
    fn unganga_forest_help_label_keeps_the_source_guangxi_border_topology() {
        // 도움말의 `운강밀림`은 별도 이벤트 이름이 아니다. 원본 광서성 4번 방의
        // "운강"과 "우거진 밀림" 설명을 합쳐 부른 지역 표기이며, 서쪽 출구로
        // 운남성 56번 방에 이어진다.
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string("data/map/광서성/4.json").unwrap())
                .unwrap();
        let info = &json["맵정보"];
        let description = info["설명"].as_array().unwrap();
        assert!(description
            .iter()
            .any(|line| line.as_str().unwrap_or_default().contains("운강")));
        assert!(description
            .iter()
            .any(|line| line.as_str().unwrap_or_default().contains("밀림")));
        assert!(info["출구"]
            .as_array()
            .unwrap()
            .iter()
            .any(|exit| exit.as_str() == Some("서 운남성:56")));
        assert!(info["몹"].is_null());
    }

    #[test]
    fn hebei_five_heroes_help_event_uses_dancheonmageom_root_and_unique_reward() {
        // 도움말의 `하북형재`는 원본에 없는 표기이며, 산서성의 하북오걸 첫째
        // 단천마검(47-1)이 적송오지를 받고 비월을 주는 체인이 대응한다.
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();

        let room = format!("하북오걸회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("산서성", "47-1", &room);
        for (name, preclaimed, expected) in [
            ("하북오걸진품회귀", false, "244"),
            ("하북오걸가품회귀", true, "244-5"),
        ] {
            crate::oneitem::oneitem_clear();
            if preclaimed {
                assert!(crate::oneitem::oneitem_have("244", "먼저온사람"));
            }
            let mut body = Body::new();
            body.set("이름", name);
            add_test_items(&mut body, "합성8", 1);
            let CommandResult::MobEvent {
                output_lines,
                set_position,
                ..
            } = super::try_mob_event(&mut body, "산서성", &room, "단천 적송오지 줘")
                .expect("source Dancheon exchange must select the first Hebei hero")
            else {
                panic!("Dancheon exchange was not a mob event");
            };
            assert!(
                output_lines.iter().any(|line| line.contains(if preclaimed {
                    "가짜비월"
                } else {
                    "비월"
                })),
                "unexpected source output for {name}: {output_lines:?}"
            );
            assert!(body_has_item_spec(&body, expected), "{name}");
            assert!(!body_has_item_spec(&body, "합성8"), "{name}");
            assert!(!get_user_event(&body, "단천마검").is_empty(), "{name}");
            assert_eq!(set_position, None, "{name}");
            let CommandResult::MobEvent { set_position, .. } =
                super::try_mob_event(&mut body, "산서성", &room, "단천 대화")
                    .expect("completed Dancheon event must offer the source shortcut")
            else {
                panic!("Dancheon shortcut was not a mob event");
            };
            assert_eq!(
                set_position,
                Some(("산서성".to_string(), "824".to_string())),
                "{name}"
            );
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }

        let map: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string("data/map/산서성/715.json").unwrap())
                .unwrap();
        assert!(map["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("47-1")));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn jinling_help_event_keeps_village_entry_and_single_immortal_orb_reward() {
        let village_room = format!("진릉촌장회귀-{}", std::process::id());
        let chest_room = format!("진릉상자회귀-{}", std::process::id());
        let (village_key, village_instance_id) = place_event_mob("섬서성", "32", &village_room);
        let (chest_key, chest_instance_id) = place_event_mob("섬서성", "35", &chest_room);
        let mut body = Body::new();
        body.set("이름", "진릉도움말회귀");

        // 원본은 일반 대화로 임동부락 상태를 시작한 뒤, 진시황릉 키워드 대화에서
        // 그 상태를 끝으로 전환하고 292번 밭으로 옮긴다.
        super::try_mob_event(&mut body, "섬서성", &village_room, "촌장 대화")
            .expect("village chief must accept the source opening dialogue");
        assert!(!get_user_event(&body, "임동부락").is_empty());
        let entry = super::try_mob_event(&mut body, "섬서성", &village_room, "촌장 진시황릉 대화")
            .expect("village chief must accept the source mausoleum keyword dialogue");
        match entry {
            CommandResult::MobEvent { set_position, .. } => {
                assert_eq!(
                    set_position,
                    Some(("섬서성".to_string(), "292".to_string()))
                );
            }
            other => panic!("source village entry returned {other:?}"),
        }
        assert!(get_user_event(&body, "임동부락").is_empty());
        assert!(!get_user_event(&body, "임동부락끝").is_empty());

        // 진릉 석실 상자는 원본처럼 계정당 천령주를 한 번만 준다.
        super::try_mob_event(&mut body, "섬서성", &chest_room, "상자 뒤져")
            .expect("jinling chest must accept the source search command");
        assert!(body_has_item_spec(&body, "yak2"));
        assert!(!get_user_event(&body, "천령주").is_empty());
        let before = body.object.objs.len();
        super::try_mob_event(&mut body, "섬서성", &chest_room, "상자 뒤져")
            .expect("repeat chest search must remain handled");
        assert_eq!(
            body.object.objs.len(),
            before,
            "source repeat search gives no second orb"
        );

        for (room, mob) in [("287", "32"), ("292", "33"), ("419", "35")] {
            let path = format!("data/map/섬서성/{room}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(
                json["맵정보"]["몹"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(mob)),
                "{path} must place source mob {mob}"
            );
        }

        for (key, instance_id, room) in [
            (village_key, village_instance_id, village_room),
            (chest_key, chest_instance_id, chest_room),
        ] {
            let mut world = crate::world::get_world_state().write().unwrap();
            assert!(world
                .mob_cache
                .get_all_mobs_in_room("섬서성", &room)
                .iter()
                .any(|mob| mob.instance_id == instance_id));
            world.mob_cache.remove_mob(&key);
        }
        let _ = std::fs::remove_file("data/user/진릉도움말회귀.json");
    }

    #[test]
    fn poison_pool_thousand_year_toad_consumes_spear_and_marks_source_corpse() {
        let room = format!("독담하수오회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("안휘성", "43", &room);
        let mut body = Body::new();
        body.set("이름", "독담하수오회귀");
        add_test_items(&mut body, "개구리창", 1);
        super::set_user_event(&mut body, "황소개구리끝", "1");

        super::try_mob_event(&mut body, "안휘성", &room, "천년하수오 개구리창 찔러")
            .expect("source frog-spear command must select the thousand-year toad");
        assert!(body_has_item_spec(&body, "yak11"));
        assert!(!body_has_item_spec(&body, "개구리창"));
        assert!(get_user_event(&body, "황소개구리끝").is_empty());
        assert!(!get_user_event(&body, "천년하수오").is_empty());
        let world = crate::world::get_world_state().read().unwrap();
        let room_mobs = world.mob_cache.get_all_mobs_in_room("안휘성", &room);
        let mob = room_mobs
            .iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert!(!mob.alive, "Python $몹상태설정 시체 must be retained");
        assert_eq!(mob.act, 2);
        drop(world);

        for (room_id, mob) in [("400", "40"), ("771", "43")] {
            let path = format!("data/map/안휘성/{room_id}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(mob)));
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/독담하수오회귀.json");
    }

    #[test]
    fn huangshan_cave_elder_key_and_rusty_chest_follow_source_state_order() {
        let elder_room = format!("황산동굴노인회귀-{}", std::process::id());
        let chest_room = format!("황산동굴상자회귀-{}", std::process::id());
        let (elder_key, elder_instance_id) = place_event_mob("안휘성", "44", &elder_room);
        let (chest_key, _) = place_event_mob("안휘성", "45", &chest_room);
        let mut body = Body::new();
        body.set("이름", "황산동굴회귀");

        super::try_mob_event(&mut body, "안휘성", &elder_room, "노인 대화")
            .expect("source elder opening must start the key state");
        assert!(!get_user_event(&body, "열쇠").is_empty());
        super::try_mob_event(&mut body, "안휘성", &elder_room, "노인 대화")
            .expect("source elder acceptance must yield the key");
        assert!(body_has_item_spec(&body, "열쇠"));
        assert!(get_user_event(&body, "열쇠").is_empty());
        assert!(!get_user_event(&body, "열쇠끝").is_empty());
        let world = crate::world::get_world_state().read().unwrap();
        let room_mobs = world.mob_cache.get_all_mobs_in_room("안휘성", &elder_room);
        let elder = room_mobs
            .iter()
            .find(|mob| mob.instance_id == elder_instance_id)
            .unwrap();
        assert!(
            !elder.alive,
            "key handoff kills the elder in the Python source"
        );
        drop(world);

        super::try_mob_event(&mut body, "안휘성", &chest_room, "상자 열쇠 사용")
            .expect("source rusty chest must accept its key command");
        assert!(!body_has_item_spec(&body, "열쇠"));
        assert!(body_has_item_spec(&body, "천룡삭"));

        for (room, mob) in [("190", "44"), ("193", "45")] {
            let path = format!("data/map/안휘성/{room}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(mob)));
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&elder_key);
        world.mob_cache.remove_mob(&chest_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/황산동굴회귀.json");
    }

    #[test]
    fn sacred_tree_zao_sword_branch_enters_source_cave() {
        let room = format!("신령수회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "89", &room);
        let mut body = Body::new();
        body.set("이름", "신령수회귀");
        add_test_items(&mut body, "자오석의검", 1);

        let result = super::try_mob_event(&mut body, "낙양성", &room, "신령수 잘라")
            .expect("source zao sword command must select sacred tree");
        match result {
            CommandResult::MobEvent { set_position, .. } => assert_eq!(
                set_position,
                Some(("낙양성".to_string(), "1430".to_string()))
            ),
            other => panic!("source sacred-tree entry returned {other:?}"),
        }
        assert!(body_has_item_spec(&body, "자오석의검"));

        let path = "data/map/낙양성/1131.json";
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(json["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("89")));
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/신령수회귀.json");
    }

    #[test]
    fn wudang_well_dialogues_use_source_flag_and_relic_completion_state() {
        let room = format!("무당우물회귀-{}", std::process::id());
        let (remnant_key, _) = place_event_mob("호북성", "20", &room);
        let (injured_key, _) = place_event_mob("호북성", "21", &room);
        let mut body = Body::new();
        body.set("이름", "무당우물회귀");

        super::try_mob_event(&mut body, "호북성", &room, "무당잔당 유물 대화")
            .expect("source relic dialogue must select wudang remnant");
        assert!(!get_user_event(&body, "무당산우물").is_empty());

        add_test_items(&mut body, "71", 1);
        super::try_mob_event(&mut body, "호북성", &room, "무당잔당 유물 대화")
            .expect("source relic completion dialogue must select wudang remnant");
        assert!(get_user_event(&body, "무당산우물").is_empty());
        assert!(!get_user_event(&body, "무당산우물 끝").is_empty());
        // Python only checks item 71 and does not delete it in this branch.
        assert!(body_has_item_spec(&body, "71"));

        let mut injured = Body::new();
        injured.set("이름", "무당우물부상자회귀");
        super::try_mob_event(&mut injured, "호북성", &room, "부상자 우물 대화")
            .expect("source injured dialogue must select well hint");
        assert!(!get_user_event(&injured, "무당산우물").is_empty());

        for (room_id, mob) in [("375", "20"), ("372", "21")] {
            let path = format!("data/map/호북성/{room_id}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(mob)));
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&remnant_key);
        world.mob_cache.remove_mob(&injured_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/무당우물회귀.json");
        let _ = std::fs::remove_file("data/user/무당우물부상자회귀.json");
    }

    #[test]
    fn hundred_floor_monuments_restore_source_ranks_unique_claim_and_mp_reward() {
        let rank_path = std::path::Path::new("data/config/rank.json");
        let saved_rank_file = std::fs::read(rank_path).ok();
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();
        for ty in ["백층탑5", "백층탑7", "백층탑9", "백층탑"] {
            crate::world::rank::rank_clear(ty);
        }

        let room = format!("백층비석회귀-{}", std::process::id());
        let mut mobs = Vec::new();
        for (source_key, rank_type) in [
            ("비석50", "백층탑5"),
            ("비석70", "백층탑7"),
            ("비석90", "백층탑9"),
        ] {
            let (mob_key, _) = place_event_mob("백층탑", source_key, &room);
            mobs.push(mob_key);
            let name = format!("{source_key}순위회귀");
            let mut body = Body::new();
            body.set("이름", name.as_str());
            super::try_mob_event(&mut body, "백층탑", &room, "비석 이름 새겨")
                .expect("source monument command must select its monument");
            assert_eq!(crate::world::rank::rank_read(rank_type, &name), 1);
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }

        let (hundred_key, _) = place_event_mob("백층탑", "비석백", &room);
        mobs.push(hundred_key);
        let mut final_body = Body::new();
        final_body.set("이름", "백층탑완주회귀");
        add_test_items(&mut final_body, "백린", 1);
        super::try_mob_event(&mut final_body, "백층탑", &room, "비석 이름 새겨")
            .expect("white-scale completion must select the hundredth monument");
        assert!(body_has_item_spec(&final_body, "극신"));
        assert!(!body_has_item_spec(&final_body, "백린"));
        assert!(!get_user_event(&final_body, "백층탑").is_empty());
        assert_eq!(crate::world::rank::rank_read("백층탑", "백층탑완주회귀"), 1);

        let (treasure_key, _) = place_event_mob("백층탑", "비석백-1", &room);
        mobs.push(treasure_key);
        let mut treasure = Body::new();
        treasure.set("이름", "백층탑보물회귀");
        treasure.set("최고내공", 100_i64);
        super::set_user_event(&mut treasure, "전설4", "1");
        super::try_mob_event(&mut treasure, "백층탑", &room, "백층비석 뒤져")
            .expect("source treasure monument command must select the monument");
        assert!(body_has_item_spec(&treasure, "246"));
        assert_eq!(treasure.get_int("최고내공"), 110);
        assert!(crate::oneitem::oneitem_get("246").contains("백층탑보물회귀"));

        let mut late = Body::new();
        late.set("이름", "백층탑늦은회귀");
        late.set("최고내공", 100_i64);
        super::set_user_event(&mut late, "전설4", "1");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut late, "백층탑", &room, "백층비석 뒤져")
                .expect("late treasure command must still complete")
        else {
            panic!("late treasure command was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("아무것도 찾을수가")));
        assert!(!body_has_item_spec(&late, "246"));
        assert_eq!(late.get_int("최고내공"), 110);

        for (room_id, source_key) in [
            ("3001", "비석50"),
            ("3002", "비석70"),
            ("3003", "비석90"),
            ("3000", "비석백"),
            ("300", "비석백-1"),
        ] {
            let path = format!("data/map/백층탑/{room_id}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(source_key)));
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        for mob_key in mobs {
            world.mob_cache.remove_mob(&mob_key);
        }
        drop(world);
        for name in ["백층탑완주회귀", "백층탑보물회귀", "백층탑늦은회귀"] {
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }
        for ty in ["백층탑5", "백층탑7", "백층탑9", "백층탑"] {
            crate::world::rank::rank_clear(ty);
        }
        if let Some(contents) = saved_rank_file {
            std::fs::write(rank_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(rank_path);
        }
        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn ascension_guard_duels_start_source_selected_mob_combat_in_order() {
        let room = format!("선인대결회귀-{}", std::process::id());
        let mut mob_keys = Vec::new();
        for (source_key, dialogue, duel) in [
            ("아랑", "아랑 대화", "아랑 대결"),
            ("전설왕", "전설왕 대화", "전설왕 대결"),
            ("유섬", "유섬 대화", "유섬 대결"),
            ("천마후", "천마후 대화", "천마후 대결"),
        ] {
            let (mob_key, instance_id) = place_event_mob("선인", source_key, &room);
            mob_keys.push(mob_key);
            let mut body = Body::new();
            body.set("이름", format!("{source_key}대결회귀"));
            add_test_items(&mut body, "도덕경", 1);
            super::try_mob_event(&mut body, "선인", &room, dialogue)
                .expect("source ascension guard dialogue must select the guard");
            super::try_mob_event(&mut body, "선인", &room, duel)
                .expect("source ascension guard duel must select the guard");
            assert_eq!(body.act, crate::player::ActState::Fight, "{source_key}");
            assert_eq!(
                crate::script::combat_commands::combat_target_instance_ids(&body),
                vec![instance_id],
                "{source_key}"
            );
            crate::script::combat_commands::remove_combat_target_instance_id(
                &mut body,
                instance_id,
            );
            let _ = std::fs::remove_file(format!("data/user/{source_key}대결회귀.json"));
        }
        for (room_id, source_key) in [
            ("334", "아랑"),
            ("420", "전설왕"),
            ("421", "유섬"),
            ("422", "천마후"),
        ] {
            let path = format!("data/map/선인/{room_id}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(source_key)));
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        for mob_key in mob_keys {
            world.mob_cache.remove_mob(&mob_key);
        }
    }

    #[test]
    fn celestial_emperor_teachings_restore_vision_mp_and_skill_count_gates() {
        let room = format!("옥황상제수련회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("선인", "옥황상제", &room);

        let mut blocked_by_vision = Body::new();
        blocked_by_vision.set("이름", "옥황상제비전회귀");
        blocked_by_vision.add_secret_skill("멸천혈폭비전");
        let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event(
            &mut blocked_by_vision,
            "선인",
            &room,
            "옥황상제 역근경 대화",
        )
        .expect("source emperor teaching must select the emperor") else {
            panic!("vision-gated teaching was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("멸천혈폭비전 먼저")));
        assert!(!blocked_by_vision
            .skill_list
            .iter()
            .any(|skill| skill == "역근경"));

        for (skill, threshold) in [
            ("태극강기", 60_usize),
            ("고영신공", 70),
            ("가의신공", 80),
            ("명옥공", 85),
            ("북명신공", 90),
            ("천외비선", 108),
        ] {
            let mut body = Body::new();
            body.set("이름", format!("옥황상제{skill}회귀"));
            body.set("최고내공", 3200_i64);
            for index in 0..threshold {
                body.skill_list.push(format!("기초무공{index}"));
            }
            super::try_mob_event(&mut body, "선인", &room, &format!("옥황상제 {skill} 대화"))
                .expect("source skill teaching must select the emperor");
            assert!(body.skill_list.iter().any(|name| name == skill), "{skill}");
            let _ = std::fs::remove_file(format!("data/user/옥황상제{skill}회귀.json"));
        }

        let mut insufficient_count = Body::new();
        insufficient_count.set("이름", "옥황상제수련부족회귀");
        insufficient_count.set("최고내공", 3200_i64);
        for index in 0..49 {
            insufficient_count
                .skill_list
                .push(format!("기초무공{index}"));
        }
        let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event(
            &mut insufficient_count,
            "선인",
            &room,
            "옥황상제 역근경 대화",
        )
        .expect("insufficient-count teaching must select the emperor") else {
            panic!("insufficient-count teaching was not an event");
        };
        assert!(output_lines.iter().any(|line| line.contains("무공 50개")));

        let mut root = Body::new();
        root.set("이름", "옥황상제역근회귀");
        root.set("최고내공", 3200_i64);
        root.set("최고체력", 100_i64);
        for index in 0..50 {
            root.skill_list.push(format!("기초무공{index}"));
        }
        super::try_mob_event(&mut root, "선인", &room, "옥황상제 역근경 대화")
            .expect("source root-classic teaching must select the emperor");
        assert!(root.skill_list.iter().any(|name| name == "역근경"));
        assert_eq!(root.get_int("최고체력"), 50_100);
        assert!(!get_user_event(&root, "역근경체력이벤트").is_empty());

        let mut yowol = Body::new();
        yowol.set("이름", "옥황상제요월회귀");
        add_test_items(&mut yowol, "요월머리", 1);
        super::try_mob_event(&mut yowol, "선인", &room, "옥황상제 요월 대화")
            .expect("source yowol reward must select the emperor");
        assert!(!body_has_item_spec(&yowol, "요월머리"));
        assert!(body_has_item_spec(&yowol, "적오골환단"));
        assert!(yowol.skill_list.iter().any(|name| name == "허공답보"));
        assert!(!get_user_event(&yowol, "요월머리이벤트").is_empty());

        let path = "data/map/선인/223.json";
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(json["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("옥황상제")));
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        for name in [
            "옥황상제비전회귀",
            "옥황상제수련부족회귀",
            "옥황상제역근회귀",
            "옥황상제요월회귀",
        ] {
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }
    }

    #[test]
    fn help_pirate_ship_maps_to_dongting_pirate_hunting_zone() {
        let mut rooms = RoomCache::new();
        let room = rooms.get_room("동정호", "19").unwrap();
        let room = room.read().unwrap();
        assert_eq!(room.display_name, "동정호변");
        assert_eq!(room.mob_ids, vec!["5", "6"]);
        drop(room);

        let mut mobs = MobCache::new();
        for (source_key, name) in [("5", "동정수로"), ("6", "장강수로")] {
            let data = mobs.load_mob("동정호", source_key).unwrap();
            assert_eq!(data.name, name);
            assert!(data.reaction_names.iter().any(|alias| alias == "해적"));
            assert_eq!(data.combat_type, 1);
            assert!(data.events.is_empty());
            assert!(data.locations.iter().any(|room| room == "19"));
        }
    }

    #[test]
    fn celestial_realm_hunting_maps_preserve_source_spawn_pairs_and_final_emperor() {
        let mut rooms = RoomCache::new();
        for room_id in [
            "0101", "0201", "0301", "0401", "0501", "0601", "0701", "0801", "0901", "1001",
        ] {
            let room = rooms.get_room("천상선계", room_id).unwrap();
            assert_eq!(
                room.read().unwrap().mob_ids,
                vec!["4150a", "4150aa"],
                "{room_id}"
            );
        }
        let final_room = rooms.get_room("천상선계", "0110").unwrap();
        assert_eq!(final_room.read().unwrap().mob_ids, vec!["옥황상제"]);

        let mut mobs = MobCache::new();
        for (source_key, skill) in [("4150a", "혈세천하"), ("4150aa", "대비단혼강")] {
            let data = mobs.load_mob("천상선계", source_key).unwrap();
            assert_eq!(data.name, "제천대성");
            assert_eq!(data.level, 4150);
            assert_eq!(data.combat_type, 1);
            assert!(data.skills.iter().any(|(name, ..)| name == skill));
            assert!(data.drop_items.iter().any(|(name, ..)| name == "선인갑옷"));
        }
        let emperor = mobs.load_mob("천상선계", "옥황상제").unwrap();
        assert_eq!(emperor.level, 4700);
        assert_eq!(emperor.combat_type, 1);
        assert!(emperor
            .drop_items
            .iter()
            .any(|(name, ..)| name == "옥황상제"));
    }

    #[test]
    fn ascension_gate_records_and_cloud_shortcuts_use_actual_source_mobs() {
        let room = format!("선인관문회귀-{}", std::process::id());
        let mut mob_keys = Vec::new();
        let mut body = Body::new();
        body.set("이름", "선인관문회귀");
        for (source_key, command, set_flag, cleared_flag) in [
            ("1관문", "일관문 통과기록", "2층끝", None),
            ("2관문", "이관문 통과기록", "4층끝", Some("2층끝")),
            ("3관문", "삼관문 통과기록", "6층끝", Some("4층끝")),
            ("4관문", "사관문 통과기록", "8층끝", Some("6층끝")),
            ("5관문", "오관문 통과기록", "10층끝", Some("8층끝")),
        ] {
            let (mob_key, _) = place_event_mob("선인", source_key, &room);
            mob_keys.push(mob_key);
            super::try_mob_event(&mut body, "선인", &room, command)
                .expect("source ascension gate record must select its gate");
            assert!(!get_user_event(&body, set_flag).is_empty(), "{source_key}");
            if let Some(cleared_flag) = cleared_flag {
                assert!(
                    get_user_event(&body, cleared_flag).is_empty(),
                    "{source_key}"
                );
            }
        }

        let (cloud_key, _) = place_event_mob("선인", "구름", &room);
        mob_keys.push(cloud_key);
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "선인", &room, "구름 5관문 출발")
                .expect("source cloud command must select the cloud")
        else {
            panic!("cloud shortcut was not an event");
        };
        assert_eq!(set_position, Some(("선인".to_string(), "353".to_string())));

        for (room_id, source_key) in [
            ("362", "1관문"),
            ("400", "2관문"),
            ("419", "3관문"),
            ("372", "4관문"),
            ("334", "5관문"),
        ] {
            let path = format!("data/map/선인/{room_id}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(source_key)));
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        for mob_key in mob_keys {
            world.mob_cache.remove_mob(&mob_key);
        }
        drop(world);
        let _ = std::fs::remove_file("data/user/선인관문회귀.json");
    }

    #[test]
    fn ascension_dragon_entry_dialogue_and_ascent_use_source_state_order() {
        let room = format!("비학천룡회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("선인", "비학천룡", &room);
        let mut body = Body::new();
        body.set("이름", "비학천룡회귀");
        add_test_items(&mut body, "비학천룡", 1);

        run_zone_event(&mut body, "선인", "비학천룡__입장이벤트_.rhai", None);
        assert!(!get_user_event(&body, "비학천룡1").is_empty());
        super::try_mob_event(&mut body, "선인", &room, "비학천룡 대화")
            .expect("source dragon dialogue must select the dragon");
        assert!(get_user_event(&body, "비학천룡1").is_empty());
        assert!(!get_user_event(&body, "비학천룡2").is_empty());
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "선인", &room, "비학천룡 주문")
                .expect("source dragon ascent must select the dragon")
        else {
            panic!("dragon ascent was not an event");
        };
        assert_eq!(set_position, Some(("선인".to_string(), "221".to_string())));

        let path = "data/map/선인/200.json";
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(json["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("비학천룡")));
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        let _ = std::fs::remove_file("data/user/비학천룡회귀.json");
    }

    #[test]
    fn middle_aged_man_uses_next_unclaimed_silver_scale_reward_in_source_order() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();

        let room = format!("은린견자단회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "중년인-1", &room);
        for (name, claimed, expected) in [
            ("은린삼회귀", &[][..], "은린3"),
            ("은린이회귀", &["은린3"][..], "은린2"),
            ("은린일회귀", &["은린3", "은린2"][..], "은린1"),
            ("은린일반회귀", &["은린3", "은린2", "은린1"][..], "은린"),
        ] {
            crate::oneitem::oneitem_clear();
            for index in claimed {
                assert!(crate::oneitem::oneitem_have(index, "먼저온사람"));
            }
            let mut body = Body::new();
            body.set("이름", name);
            add_test_items(&mut body, "견자단", 1);
            super::set_user_event(&mut body, "취선노인15", "1");
            super::set_user_event(&mut body, "취선노인", "1");
            super::try_mob_event(&mut body, "낙양성", &room, "중년인 대화")
                .expect("source middle-aged-man exchange must select the NPC");
            assert!(body_has_item_spec(&body, expected), "{name}");
            assert!(!body_has_item_spec(&body, "견자단"), "{name}");
            assert!(get_user_event(&body, "취선노인15").is_empty(), "{name}");
            assert!(get_user_event(&body, "취선노인").is_empty(), "{name}");
            assert!(!get_user_event(&body, "취선노인끝").is_empty(), "{name}");
            if expected != "은린" {
                assert!(crate::oneitem::oneitem_get(expected).contains(name));
            }
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string("data/map/낙양성/7002.json").unwrap())
                .unwrap();
        assert!(json["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("중년인-1")));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn ascension_duel_deaths_and_pangu_complete_the_source_route() {
        for (script, destination) in [
            ("아랑__소멸이벤트_.rhai", "420"),
            ("전설왕__소멸이벤트_.rhai", "421"),
            ("유섬__소멸이벤트_.rhai", "422"),
            ("천마후__소멸이벤트_.rhai", "423"),
        ] {
            let mut body = Body::new();
            let (_, position) = run_zone_event(&mut body, "선인", script, None);
            assert_eq!(
                position,
                Some(("선인".to_string(), destination.to_string())),
                "{script}"
            );
        }

        let rank_path = std::path::Path::new("data/config/rank.json");
        let saved_rank_file = std::fs::read(rank_path).ok();
        crate::world::rank::rank_clear("무적선인");
        let room = format!("반고완료회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("선인", "반고", &room);
        let mut body = Body::new();
        body.set("이름", "반고완료회귀");
        body.set("최고내공", 100_i64);
        add_test_items(&mut body, "도덕경", 1);
        super::set_user_event(&mut body, "천마후끝", "1");

        let entered = super::try_mob_event(&mut body, "선인", &room, "반고 대화")
            .expect("source Pangu dialogue must select the actual NPC");
        let CommandResult::MobEventEnter {
            event_key, words, ..
        } = entered
        else {
            panic!("source Pangu dialogue must wait for enter");
        };
        super::try_mob_event_resume(
            &mut body,
            "선인",
            &room,
            &mob_key,
            &event_key,
            words,
            0,
            Some("step1".into()),
        )
        .expect("Pangu enter must complete the source reward");
        assert_eq!(body.get_int("최고내공"), 400);
        assert!(!body_has_item_spec(&body, "도덕경"));
        assert!(get_user_event(&body, "천마후끝").is_empty());
        assert!(!get_user_event(&body, "반고선택").is_empty());

        super::set_user_event(&mut body, "10층끝", "1");
        super::try_mob_event(&mut body, "선인", &room, "반고 대화")
            .expect("selected Pangu reward must complete ascension tower");
        assert!(get_user_event(&body, "반고선택").is_empty());
        assert!(get_user_event(&body, "10층끝").is_empty());
        assert!(!get_user_event(&body, "선인탑끝").is_empty());
        assert_eq!(crate::world::rank::rank_read("무적선인", "반고완료회귀"), 1);

        let path = "data/map/선인/423.json";
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(json["맵정보"]["몹"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("반고")));
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        crate::world::rank::rank_clear("무적선인");
        if let Some(contents) = saved_rank_file {
            std::fs::write(rank_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(rank_path);
        }
        let _ = std::fs::remove_file("data/user/반고완료회귀.json");
    }

    #[test]
    fn mighty_vajra_grasp_keyword_chain_uses_source_npcs_and_materials_in_order() {
        let room = format!("대력금나수연쇄회귀-{}", std::process::id());
        let (gwangmu_key, _) = place_event_mob("낙양성", "기타맨", &room);
        let (wini_key, _) = place_event_mob("육룡신전", "위니", &room);
        let (param_key, _) = place_event_mob("육룡신전", "파람", &room);
        let (simpson_key, _) = place_event_mob("육룡신전", "심슨", &room);
        let mut body = Body::new();
        body.set("이름", "대력금나수연쇄회귀");

        super::try_mob_event(&mut body, "낙양성", &room, "광무 대력금나수 대화")
            .expect("Gwangmu must give the source first keyword");
        assert!(!get_user_event(&body, "육룡초면").is_empty());

        super::try_mob_event(&mut body, "육룡신전", &room, "위니 육룡초면 대화")
            .expect("Wini must accept the source first keyword");
        assert!(!get_user_event(&body, "용우혈도").is_empty());
        add_test_items(&mut body, "120", 1);
        super::try_mob_event(&mut body, "육룡신전", &room, "위니 우혈도 줘")
            .expect("Wini must consume the source proof item");
        assert!(!body_has_item_spec(&body, "120"));
        assert!(get_user_event(&body, "용우혈도").is_empty());
        assert!(!get_user_event(&body, "대엽기행").is_empty());

        super::try_mob_event(&mut body, "육룡신전", &room, "파람 대엽기행 대화")
            .expect("Param must accept the source second keyword");
        assert!(!get_user_event(&body, "장천독단").is_empty());
        add_test_items(&mut body, "천독내단", 1);
        super::try_mob_event(&mut body, "육룡신전", &room, "파람 천독내단 줘")
            .expect("Param must consume the source inner pill");
        assert!(!body_has_item_spec(&body, "천독내단"));
        assert!(!get_user_event(&body, "군주대면").is_empty());

        super::try_mob_event(&mut body, "육룡신전", &room, "심슨 만사형통 대화")
            .expect("Simpson must grant the source final weapon");
        assert!(body_has_item_spec(&body, "무혈정검"));
        assert!(!get_user_event(&body, "대력금나수끝").is_empty());
        for cleared in ["군주대면", "대엽기행", "육룡초면", "장천독단"] {
            assert!(get_user_event(&body, cleared).is_empty(), "{cleared}");
        }

        for (zone, room_id, source_key) in [
            ("낙양성", "4008", "기타맨"),
            ("육룡신전", "121", "위니"),
            ("육룡신전", "126", "파람"),
            ("육룡신전", "123", "심슨"),
        ] {
            let path = format!("data/map/{zone}/{room_id}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(source_key)));
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        for mob_key in [gwangmu_key, wini_key, param_key, simpson_key] {
            world.mob_cache.remove_mob(&mob_key);
        }
        drop(world);
        let _ = std::fs::remove_file("data/user/대력금나수연쇄회귀.json");
    }

    #[test]
    fn scarlet_blood_whirlwind_gwangmu_yes_and_no_branches_apply_source_penalties() {
        let room = format!("전암광무응답회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "기타맨", &room);

        let mut accepted = Body::new();
        accepted.set("이름", "전암수락회귀");
        accepted.set("체력", 150_000_i64);
        super::set_user_event(&mut accepted, "전암조건", "1");
        super::try_mob_event(&mut accepted, "낙양성", &room, "광무 예 대화")
            .expect("source Gwangmu yes branch must select the NPC");
        assert_eq!(accepted.get_int("체력"), 150_000);
        assert!(get_user_event(&accepted, "전암조건").is_empty());
        assert!(!get_user_event(&accepted, "소개장").is_empty());

        let mut declined = Body::new();
        declined.set("이름", "전암거절회귀");
        declined.set("체력", 150_000_i64);
        super::set_user_event(&mut declined, "전암조건", "1");
        super::try_mob_event(&mut declined, "낙양성", &room, "광무 아니오 대화")
            .expect("source Gwangmu no branch must select the NPC");
        assert_eq!(declined.get_int("체력"), 50_000);
        assert!(!get_user_event(&declined, "전암조건").is_empty());

        let mut repeated = Body::new();
        repeated.set("이름", "전암반복수락회귀");
        repeated.set("체력", 150_000_i64);
        super::set_user_event(&mut repeated, "소개장", "1");
        super::try_mob_event(&mut repeated, "낙양성", &room, "광무 예 대화")
            .expect("source repeated yes branch must select the NPC");
        assert_eq!(repeated.get_int("체력"), 50_000);

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        for name in ["전암수락회귀", "전암거절회귀", "전암반복수락회귀"] {
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }
    }

    #[test]
    fn scarlet_blood_whirlwind_chain_keeps_source_introduction_and_tomb_rewards() {
        let room = format!("전암전회연쇄회귀-{}", std::process::id());
        let (gwangmu_key, _) = place_event_mob("낙양성", "기타맨", &room);
        let (mirang_key, _) = place_event_mob("육룡신전", "미랑", &room);
        let (param_key, _) = place_event_mob("육룡신전", "파람", &room);
        let (coffin_key, _) = place_event_mob("육룡신전", "석관", &room);
        let mut body = Body::new();
        body.set("이름", "전암전회연쇄회귀");

        let entered = super::try_mob_event(&mut body, "낙양성", &room, "광무 전암전회 대화")
            .expect("Gwangmu must start the source introduction dialogue");
        let CommandResult::MobEventEnter {
            event_key, words, ..
        } = entered
        else {
            panic!("source Gwangmu introduction must wait for enter");
        };
        super::try_mob_event_resume(
            &mut body,
            "낙양성",
            &room,
            &gwangmu_key,
            &event_key,
            words,
            0,
            Some("step1".into()),
        )
        .expect("Gwangmu enter must complete the source introduction");
        assert!(!get_user_event(&body, "미랑소개").is_empty());

        super::try_mob_event(&mut body, "육룡신전", &room, "미랑 대화")
            .expect("Mirang must convert the source introduction state");
        assert!(get_user_event(&body, "미랑소개").is_empty());
        assert!(!get_user_event(&body, "혈미인").is_empty());
        super::try_mob_event(&mut body, "낙양성", &room, "광무 전암전회 대화")
            .expect("Gwangmu must offer the source introduction condition");
        assert!(get_user_event(&body, "혈미인").is_empty());
        assert!(!get_user_event(&body, "전암조건").is_empty());
        super::try_mob_event(&mut body, "낙양성", &room, "광무 예 대화")
            .expect("Gwangmu must accept the source condition");
        assert!(get_user_event(&body, "전암조건").is_empty());
        assert!(!get_user_event(&body, "소개장").is_empty());

        add_test_items(&mut body, "yak11", 1);
        super::try_mob_event(&mut body, "낙양성", &room, "광무 전암전회 대화")
            .expect("Gwangmu must exchange the source inner pill");
        assert!(!body_has_item_spec(&body, "yak11"));
        assert!(body_has_item_spec(&body, "소개장"));
        assert!(body_has_item_spec(&body, "합성10"));
        assert!(get_user_event(&body, "소개장").is_empty());

        super::try_mob_event(&mut body, "육룡신전", &room, "미랑 대화")
            .expect("Mirang must consume the source introduction letter");
        assert!(!body_has_item_spec(&body, "소개장"));
        assert!(!get_user_event(&body, "엽기비동").is_empty());
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "육룡신전", &room, "파람 대화")
                .expect("Param must open the source tomb route")
        else {
            panic!("source tomb route was not an event");
        };
        assert_eq!(
            set_position,
            Some(("육룡신전".to_string(), "1000".to_string()))
        );

        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "육룡신전", &room, "석관 삼배")
                .expect("source tomb bow must select the coffin")
        else {
            panic!("source tomb reward was not an event");
        };
        assert!(body_has_item_spec(&body, "혈루인"));
        assert!(get_user_event(&body, "엽기비동").is_empty());
        assert!(!get_user_event(&body, "전암전회끝").is_empty());
        assert_eq!(set_position, Some(("낙양성".to_string(), "1".to_string())));

        for (zone, room_id, source_key) in [
            ("낙양성", "4008", "기타맨"),
            ("육룡신전", "125", "미랑"),
            ("육룡신전", "126", "파람"),
            ("육룡신전", "1000", "석관"),
        ] {
            let path = format!("data/map/{zone}/{room_id}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert!(json["맵정보"]["몹"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(source_key)));
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        for mob_key in [gwangmu_key, mirang_key, param_key, coffin_key] {
            world.mob_cache.remove_mob(&mob_key);
        }
        drop(world);
        let _ = std::fs::remove_file("data/user/전암전회연쇄회귀.json");
    }
}
