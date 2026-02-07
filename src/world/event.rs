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
use crate::world::{get_world_state, EventScript, MobInstance, RawMobData};

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
    let cnt = if l >= 3 {
        parse_int(tok[l - 1])
    } else {
        1
    };
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
        let score = all.iter().filter(|k| words.contains(&k)).count();
        if best.as_ref().map_or(true, |(_, s)| *s < score) {
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
    body.object
        .attr
        .insert("이벤트설정리스트".to_string(), Value::String(format_event_string(&m)));
    let path = format!("data/user/{}.json", body.get_name());
    let _ = save_body_to_json(body, &path);
}

fn del_user_event(body: &mut Body, key: &str) {
    let mut m = parse_event_string(&body.get_string("이벤트설정리스트"));
    m.remove(key);
    body.object
        .attr
        .insert("이벤트설정리스트".to_string(), Value::String(format_event_string(&m)));
    let path = format!("data/user/{}.json", body.get_name());
    let _ = save_body_to_json(body, &path);
}

/// $무림별호조건: getTendency. 완성=무림별호 있음, 정파/사파=성격 일치.
fn get_tendency(body: &Body, t: &str) -> bool {
    let t = t.trim();
    if t.is_empty() {
        return false;
    }
    match t {
        "완성" => !body.get_string("무림별호").is_empty(),
        "정파" | "사파" => body.get_string("성격") == t,
        _ => true,
    }
}

/// $출력 / 일반 줄: [공], [사용자이름], [공](이/가) 치환.
fn substitute_line(line: &str, player_name: &str) -> String {
    let r = line
        .replace("[사용자이름]", player_name)
        .replace("[공](이/가)", &format!("{}{}", player_name, han_iga(player_name)));
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
                sline_mut = sline_mut.replace("$변수:1", words.get(1).map(|s| s.as_str()).unwrap_or(""));
            }

            let func = sline_mut.split_whitespace().next().unwrap_or("");
            let next_words = get_next_words(&sline_mut);

            match func {
                "$엔터$" => {
                    return CommandResult::MobEventEnter {
                        output_lines: output,
                        set_position: set_position.clone(),
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
                            body.object.objs.push(arc);
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
        }
    } else {
        CommandResult::MobEvent {
            output_lines: output,
            set_position: None,
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
    let script_path = Path::new("data/script").join(data.zone.as_str()).join(&with_ext);
    let src = match std::fs::read_to_string(&script_path) {
        Ok(s) => s,
        Err(_) => return CommandResult::Output("(이벤트 스크립트 파일 없음)".to_string()),
    };

    let mut out_lines: Vec<String> = Vec::new();
    let mut out_set_position: Option<(String, String)> = None;
    let out_ptr = &mut out_lines as *mut Vec<String>;
    let pos_ptr = &mut out_set_position as *mut Option<(String, String)>;
    let body_ptr = body as *mut Body;
    let player_name_out = player_name.clone();

    let mut engine = Engine::new();

    // end_event()는 Rhai에서 throw로 종료. 사용자 스크립트와 같은 컴파일 단위에 넣어야 call_fn 시 노출됨.
    const END_EVENT_PREAMBLE: &str = r#"fn end_event() { throw #{ type: "event_complete" }; }"#;
    let src_with_preamble = format!("{}\n\n{}", END_EVENT_PREAMBLE, src);

    engine.register_fn("output", move |msg: &str| {
        let line = event_substitute(msg, &player_name_out);
        unsafe { (*out_ptr).push(line); }
    });
    engine.register_fn("set_position", move |zone: &str, room: &str| {
        unsafe { *pos_ptr = Some((zone.to_string(), room.to_string())); }
    });
    engine.register_fn("check_event", move |key: &str| -> bool {
        let b = unsafe { &*body_ptr };
        !get_user_event(b, key).is_empty()
    });
    engine.register_fn("set_event", move |key: &str, val: &str| {
        let b = unsafe { &mut *body_ptr };
        set_user_event(b, key, if val.is_empty() { "1" } else { val });
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
                b.object.objs.push(arc);
            }
        }
    });
    engine.register_fn("get_tendency", move |t: &str| -> bool {
        let b = unsafe { &*body_ptr };
        get_tendency(b, t)
    });
    engine.register_fn("has_item", move |index: &str| -> bool {
        let b = unsafe { &*body_ptr };
        if index == "은전" {
            return b.get_int("은전") > 0;
        }
        if index == "금전" {
            return b.get_int("금전") > 0;
        }
        if b.object.find_by_index(index).is_some() {
            return true;
        }
        *b.object.inv_stack.get(index).unwrap_or(&0) > 0
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
    engine.register_fn("words", move |i: i64| -> String {
        words_vec
            .get(i as usize)
            .cloned()
            .unwrap_or_default()
    });
    engine.register_fn("wait_enter", move |next_func: &str, prompt: &str| -> Result<(), Box<EvalAltResult>> {
        let mut m = Map::new();
        m.insert("type".into(), Dynamic::from("event_enter"));
        m.insert("next_func".into(), Dynamic::from(next_func.to_string()));
        m.insert("prompt".into(), Dynamic::from(prompt.to_string()));
        Err(Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(m),
            Position::default(),
        )))
    });

    let ast = match engine.compile(&src_with_preamble) {
        Ok(a) => a,
        Err(e) => return CommandResult::Output(format!("(이벤트 스크립트 컴파일 오류: {})", e)),
    };
    let mut scope = Scope::new();
    let entry = resume_func
        .clone()
        .unwrap_or_else(|| "event".to_string());
    let r = engine.call_fn::<Dynamic>(&mut scope, &ast, &entry, ());

    match r {
        Ok(_) => CommandResult::MobEvent {
            output_lines: out_lines,
            set_position: out_set_position,
        },
        Err(e) => {
            // end_event()의 throw는 ErrorInFunctionCall로 감싸져 올 수 있음. 안쪽 ErrorRuntime까지 풀어서 확인.
            let mut err: &EvalAltResult = &*e;
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
                        };
                    }
                }
            }
            if let EvalAltResult::ErrorFunctionNotFound(name, _) = err {
                if name == "event" && resume_func.is_none() {
                    return CommandResult::MobEvent {
                        output_lines: out_lines,
                        set_position: out_set_position,
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

    let world = get_world_state().read().ok()?;
    let name = words[0].as_str();

    let mut candidates: Vec<(&MobInstance, &RawMobData)> = Vec::new();
    for inst in world.mob_cache.get_mobs_in_room(zone, room) {
        let data = match world.mob_cache.get_instance_data(inst) {
            Some(d) => d,
            None => continue,
        };
        let ok = inst.name == name
            || inst.name.starts_with(name)
            || data.reaction_names.iter().any(|n| *n == name || n.starts_with(name));
        if ok {
            candidates.push((inst, data));
        }
    }
    candidates.sort_by_key(|(inst, _)| std::cmp::Reverse(inst.name.len()));

    let words_ref: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
    if candidates.is_empty() {
        info!("[try_mob_event] no candidates words={:?} zone={} room={}", words_ref, zone, room);
    }
    for (inst, data) in candidates {
        let event_key = match check_event_key(data, &words_ref) {
            Some(k) => k,
            None => {
                let ev: Vec<&str> = data.events.keys().filter(|k| k.starts_with("이벤트")).map(String::as_str).collect();
                info!("[try_mob_event] check_event_key=None words={:?} mob_key={} ev_keys={:?}", words_ref, inst.mob_key, ev);
                return None;
            }
        };
        return Some(do_event(
            body,
            data,
            &event_key,
            &words,
            &inst.mob_key,
            None,
            None,
        ));
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
    let mat_idx = mat_arc.lock().ok().map(|o| o.getString("인덱스")).unwrap_or_default();
    let 올숙키 = format!("{}_올숙무기", body.get_name());
    let my_arc = body.object.find_by_index(&올숙키)
        .ok_or_else(|| "☞ 무기를 벗고 하세요.".to_string())?;
    if my_arc.lock().map(|o| o.getBool("inUse")).unwrap_or(false) {
        return Err("☞ 무기를 벗고 하세요.".to_string());
    }
    let mut my_op = my_arc.lock().ok().and_then(|o| o.get_option()).unwrap_or_default();
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
            let nw = if let Some(rest) = line.splitn(2, |c: char| c.is_whitespace()).nth(1) {
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
            if line.starts_with("$키입력") || line.starts_with("$단어입력") || line.starts_with("$한줄입력") {
                let prompt = if nw.is_empty() {
                    "입력: ".to_string()
                } else {
                    format!("{} ", nw)
                };
                return (out, ScriptNext::Wait { line_num: i + 1, prompt, persist_temp: None, from_confirm: false, script_ob: None, script_resume_op: None });
            }
            if line.starts_with("$입력확인") {
                out.push("입력하신 내용이 맞습니까? (네/취소) : ".to_string());
                let persist = input.clone();
                return (out, ScriptNext::Wait { line_num: i + 1, prompt: String::new(), persist_temp: persist, from_confirm: true, script_ob: None, script_resume_op: None });
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
    let mut ob = script_ob
        .map(script_hashmap_to_ob)
        .unwrap_or_else(Map::new);

    let res_op = script_resume_op.as_deref().unwrap_or("");
    let res_in = input.as_deref().unwrap_or("");
    let persist = temp_input.clone().unwrap_or_default();

    let mut engine = Engine::new();
    let out_ptr = &mut out as *mut Vec<String>;
    let body_ptr = body as *mut Body;

    engine.register_fn("send_line", move |_ob: Dynamic, msg: &str| {
        let line = if msg.is_empty() { "\r\n".to_string() } else { format!("{}\r\n", msg) };
        unsafe { (*out_ptr).push(line) };
    });

    // end_script는 lib/script/common.rhai에 정의 (throw script_complete)

    let temp_clone = temp_input.clone();
    engine.register_fn("get_persisted", move || temp_clone.clone().unwrap_or_default());

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
        let Some(ref arc) = b.script_temp_item else { return String::new() };
        arc.lock().ok().map(|o| o.get_option_str()).unwrap_or_default()
    });

    let temp_for_oc = temp_input.clone();
    engine.register_fn("option_confirm", move |_ob: Dynamic| -> String {
        let b = unsafe { &mut *body_ptr };
        let op = temp_for_oc.clone().unwrap_or_default();
        let mat = match b.script_temp_item.clone() {
            Some(m) => m,
            None => return "* 무기강화를 종료합니다.".to_string(),
        };
        weapon_upgrade_do_option(b, &mat, &op).err().unwrap_or_default()
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
                let t: String = m.get("type").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                if t == "script_suspend" {
                    let op: String = m.get("op").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                    let prompt: String = m.get("prompt").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                    let persist_temp = m.get("persist").and_then(|v: &Dynamic| v.clone().into_string().ok());
                    return (out, ScriptNext::Wait {
                        line_num: 0,
                        prompt,
                        persist_temp,
                        from_confirm: op == "confirm",
                        script_ob: Some(script_ob_to_hashmap(ob)),
                        script_resume_op: Some(op),
                    });
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
    use super::check_event_key;
    use crate::world::{EventScript, RawMobData};

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
}
