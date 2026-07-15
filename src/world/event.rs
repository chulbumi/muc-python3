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
use crate::player::{ActiveSkill, Body};
use crate::script::{
    event_list_remove, event_list_set, load_script_file, mark_body_attr_as_json_array,
    mark_item_field_as_json_array, object_from_item_json, parse_event_string, save_body_to_json,
};
use crate::world::{get_world_state, EventScript, RawMobData};

/// Internal event-output boundary for Python `$특성치변경`'s immediate
/// `lpPrompt()`.  This is not player-visible text: the network writer expands
/// it with the current HP/MP prompt only when that prompt is enabled.
pub const EVENT_LP_PROMPT_MARKER: &str = "\u{001e}event-lp-prompt\u{001e}";
/// Signals the command boundary to run the existing `__combat_tick` and
/// `__death` Rhai presentation/drop sequence immediately after a lethal
/// event directive, matching Python `Player.die()`.
pub const EVENT_DEATH_FINISH_REQUEST: &str = "_event_death_finish_request";

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
    let updated = event_list_set(&body.get_string("이벤트설정리스트"), key, value);
    mark_body_attr_as_json_array(body, "이벤트설정리스트");
    body.object
        .attr
        .insert("이벤트설정리스트".to_string(), Value::String(updated));
}

fn del_user_event(body: &mut Body, key: &str) {
    let updated = event_list_remove(&body.get_string("이벤트설정리스트"), key);
    mark_body_attr_as_json_array(body, "이벤트설정리스트");
    body.object
        .attr
        .insert("이벤트설정리스트".to_string(), Value::String(updated));
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
        // Python falls through with None for an unknown condition; event
        // checks treat that as false rather than granting a pass-through.
        _ => false,
    }
}

/// $출력 / 일반 줄: [공], [사용자이름], [공](이/가) 치환.
fn substitute_line(line: &str, player_name: &str) -> String {
    let r = line
        .replace("[사용자이름]", player_name)
        .replace("[공]", player_name);
    crate::hangul::post_position1(&r)
}

/// Rhai 이벤트용 output efun: [공], [사용자이름], [공](이/가) 치환.
fn event_substitute(line: &str, player_name: &str) -> String {
    substitute_line(line, player_name)
}

/// Python event self output is not the same as a room/global recipient.
/// `$출력` and `$순위갱신` replace `[공]` with `당신` before applying the
/// Korean particle marker, while observer messages use the player's name.
fn event_self_substitute(line: &str, player_name: &str) -> String {
    let rendered = line
        .replace("[사용자이름]", player_name)
        .replace("[공]", "당신");
    crate::hangul::post_position1(&rendered)
}

/// Python Player.printScript()'s same-room recipient form.  It substitutes
/// `[공]` with `getNameA()` (bold ANSI name), whereas the caller form uses
/// the literal `당신` and ordinary event substitution uses a plain name.
fn event_room_substitute(line: &str, player_name: &str) -> String {
    // Python Player.getNameA(): '\x1b[1m' + name + '\x1b[0;37m'.
    // This must not use the yellow NPC-name palette (`\x1b[33m...\x1b[37m`).
    let colored_name = format!("\x1b[1m{player_name}\x1b[0;37m");
    let rendered = line
        .replace("[사용자이름]", player_name)
        .replace("[공]", &colored_name);
    crate::hangul::post_position1(&rendered)
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
    let mut room_broadcast_lines: Vec<String> = Vec::new();
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
                        room_broadcast_lines,
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
                    // Python `setEvent(nextWords)` appends the whole
                    // remainder as one `이벤트설정리스트` element.  In
                    // particular, `오소리가죽 이벤트` and `무당산우물 끝`
                    // are single flag names, not a key/value pair.
                    if !next_words.is_empty() {
                        set_user_event(body, &next_words, "1");
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
                    // Python Player.printScript() renders `$출력` twice:
                    // the caller receives `[공]` as `당신`, while same-room
                    // observers receive Player.getNameA().  Keep this
                    // legacy-array fallback identical to the Rhai path.
                    output.push(event_self_substitute(&next_words, &player_name));
                    room_broadcast_lines.push(event_room_substitute(&next_words, &player_name));
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
                    // Python compares `words[c]`: `words` still includes
                    // the mob target at index 0, so `$변수확인 1 242`
                    // checks the first argument (`words[1]`), not the event
                    // verb at `words[2]`.
                    if words_ref.get(c).copied() != v.get(2).copied() {
                        search_end = true;
                    }
                }
                "$아이템주기" => {
                    let (index, cnt) = get_str_cnt(&sline_mut);
                    if index.is_empty() {
                        continue;
                    }
                    let mut roll = |min: i64, max: i64| fastrand::i64(min..=max);
                    give_event_item_with_roll(body, &index, cnt, 0, &mut roll);
                }
                "$아이템삭제" => {
                    let (index, cnt) = get_str_cnt(&sline_mut);
                    if !index.is_empty() {
                        del_item_from_body(body, &index, cnt);
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
            room_broadcast_lines,
        }
    } else {
        CommandResult::MobEvent {
            output_lines: output,
            set_position: None,
            broadcast_lines: Vec::new(),
            room_broadcast_lines,
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

    do_event_rhai_source(body, data, event_key, words, mob_key, &src, resume_func)
}

/// 이미 읽은 Rhai 이벤트 소스를 실행한다. 파일 기반 경로와 동일한 efun 등록을
/// 사용하므로, 테스트에서도 임시 스크립트 파일 없이 실제 이벤트 경계를 검증할 수 있다.
fn do_event_rhai_source(
    body: &mut Body,
    data: &RawMobData,
    event_key: &str,
    words: &[String],
    mob_key: &str,
    src: &str,
    resume_func: Option<String>,
) -> CommandResult {
    let player_name = body.get_name().to_string();
    let words_vec = words.to_vec();

    let mut out_lines: Vec<String> = Vec::new();
    let mut out_broadcast_lines: Vec<String> = Vec::new();
    let mut out_room_broadcast_lines: Vec<String> = Vec::new();
    let mut out_set_position: Option<(String, String)> = None;
    let out_ptr = &mut out_lines as *mut Vec<String>;
    let broadcast_ptr = &mut out_broadcast_lines as *mut Vec<String>;
    let room_broadcast_ptr = &mut out_room_broadcast_lines as *mut Vec<String>;
    let pos_ptr = &mut out_set_position as *mut Option<(String, String)>;
    let body_ptr = body as *mut Body;
    let player_name_out = player_name.clone();
    // Python `$위치이동` prefixes the destination zone with the selected
    // mob's raw 난이도 value (`zone:room` -> `zone{난이도}:room`).  Most
    // current mobs do not carry that attribute, but the handler is part of
    // the source contract and must keep working for a hot-reloaded mob.
    let mob_difficulty = data
        .attributes
        .get("난이도")
        .and_then(|value| match value {
            serde_json::Value::String(value) => Some(value.clone()),
            serde_json::Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
        .unwrap_or_default();

    let mut engine = Engine::new();

    // end_event()는 Rhai에서 throw로 종료. 사용자 스크립트와 같은 컴파일 단위에 넣어야 call_fn 시 노출됨.
    const END_EVENT_PREAMBLE: &str = r#"fn end_event() { throw #{ type: "event_complete" }; }"#;
    // Keep reusable event presentation in Rhai rather than Rust.  This file
    // is read for every invocation so it follows the same hot-reload rule as
    // the selected event script.
    let common = std::fs::read_to_string("data/script/event_common.rhai")
        .unwrap_or_else(|error| panic!("missing shared event Rhai helpers: {error}"));
    let src_with_preamble = format!("{}\n\n{}\n\n{}", END_EVENT_PREAMBLE, common, src);

    engine.register_fn("output", move |msg: &str| {
        let line = event_substitute(msg, &player_name_out);
        unsafe {
            (*out_ptr).push(line);
        }
    });
    let player_name_self = player_name.clone();
    engine.register_fn("self_output", move |msg: &str| {
        let line = event_self_substitute(msg, &player_name_self);
        unsafe {
            (*out_ptr).push(line);
        }
    });
    // Ordinary legacy event text is not `$출력`: Python only substitutes
    // `[사용자이름]` before it reaches this path and leaves a literal `[공]`
    // untouched.  The few converted scripts that carry such text opt in to
    // this raw sink instead of the recipient-aware output helpers above.
    engine.register_fn("literal_output", move |msg: &str| unsafe {
        (*out_ptr).push(msg.to_string());
    });
    engine.register_fn("post_position_once", crate::hangul::post_position1);
    let player_name_broadcast = player_name.clone();
    engine.register_fn("broadcast_output", move |msg: &str| {
        let line = event_substitute(msg, &player_name_broadcast);
        unsafe {
            (*broadcast_ptr).push(line);
        }
    });
    let player_name_room_broadcast = player_name.clone();
    engine.register_fn("room_broadcast_output", move |msg: &str| {
        let line = event_room_substitute(msg, &player_name_room_broadcast);
        unsafe {
            (*room_broadcast_ptr).push(line);
        }
    });
    let player_name_for_rank = player_name.clone();
    engine.register_fn("event_player_name", move || player_name_for_rank.clone());
    // Event directives call Python `getInt()`: `1번` means 1, while a
    // non-numeric name (and a zero-prefixed non-number such as `0번`) stays
    // on the name-lookup branch.  Rank-board Rhai uses this for its own
    // rendered templates too.
    engine.register_fn("event_to_int", parse_int);
    engine.register_fn("get_stat", move |key: &str| -> i64 {
        let b = unsafe { &*body_ptr };
        b.get_int(key)
    });
    // Python `$무공시전`/`$무공시전2` applies the named defense skill once.
    // The event Rhai script owns the corresponding presentation.
    engine.register_fn("apply_defense_skill", move |name: &str| -> bool {
        let b = unsafe { &mut *body_ptr };
        if b.active_skills.iter().any(|effect| effect.name == name) {
            return false;
        }
        let Some(skill) = crate::world::get_skill(name) else {
            return false;
        };

        let mut effect = ActiveSkill::new(skill.name.clone(), skill.defense_time as i32);
        effect.str_bonus = skill.str_bonus as i32;
        effect.dex_bonus = skill.dex_bonus as i32;
        effect.arm_bonus = skill.arm_bonus as i32;
        effect.mp_bonus = skill.mp_bonus as i32;
        effect.max_mp_bonus = skill.max_mp_bonus as i32;
        effect.hp_bonus = skill.hp_bonus as i32;
        effect.max_hp_bonus = skill.max_hp_bonus as i32;
        effect.anti_type = skill.deny;
        effect.category = skill.category;
        effect.recovery_percent = skill.recovery_percent;
        effect.recovery_script = skill.recovery_script;
        effect.release_script = skill.release_script;

        b._str += effect.str_bonus;
        b._dex += effect.dex_bonus;
        b._arm += effect.arm_bonus;
        b._mp += effect.mp_bonus;
        b._maxmp += effect.max_mp_bonus;
        b._hp += effect.hp_bonus;
        b._maxhp += effect.max_hp_bonus;
        b.active_skills.push(effect);
        b.sync_active_skills_to_attrs();
        true
    });
    engine.register_fn("rank_write", crate::world::rank::rank_write);
    engine.register_fn("rank_read", crate::world::rank::rank_read);
    engine.register_fn("rank_get_num", |ty: &str, position: i64| {
        crate::world::rank::rank_get_num(ty, position).unwrap_or_default()
    });
    engine.register_fn("rank_get_all", crate::world::rank::rank_get_all);
    // Python `$순위기록` keeps the old and new positions for following
    // `$순위갱신` lines.  The text itself remains in Rhai, where it can be
    // emitted to the caller and broadcast only when this returns true.
    let rank_record_player_name = player_name.clone();
    engine.register_fn("rank_record", move |limit: i64, ty: &str| -> bool {
        let b = unsafe { &*body_ptr };
        let old_rank = crate::world::rank::rank_read(ty, &rank_record_player_name);
        let raw_value = b.get_string(ty);
        let value = if raw_value.is_empty() {
            -1
        } else {
            b.get_int(ty)
        };
        let new_rank = crate::world::rank::rank_write(ty, &rank_record_player_name, value, limit);
        old_rank != new_rank && new_rank == 1
    });
    // `$순위기록` itself controls the following braced block by whether the
    // player remained inside the requested limit, not by whether they became
    // first.  `$순위갱신` then separately uses the old/new first-place rule.
    let rank_recorded_player_name = player_name.clone();
    engine.register_fn("rank_recorded", move |ty: &str| -> bool {
        crate::world::rank::rank_read(ty, &rank_recorded_player_name) > 0
    });
    // Python `$순위확인 최대순위 종류`: resolve the optional `[대상|숫자|모두]`
    // argument into data only. Each Rhai event owns its own rendered text.
    let rank_words = words_vec.clone();
    let rank_player_name = player_name.clone();
    engine.register_fn("rank_query", move |limit: i64, ty: &str| -> Map {
        let mut result = Map::new();
        let mut name = rank_player_name.clone();
        let mut position = 0_i64;
        let mut all = false;
        let mut all_text = String::new();

        if rank_words.len() == 3 {
            let requested = rank_words[1].trim();
            if requested == "모두" {
                all = true;
                all_text = crate::world::rank::rank_get_all(ty);
            } else {
                let number = parse_int(requested);
                if number > 0 {
                    position = number.min(limit);
                    name = crate::world::rank::rank_get_num(ty, position)
                        .unwrap_or_else(|| format!("[{position}위]"));
                } else {
                    name = requested.to_string();
                    position = crate::world::rank::rank_read(ty, &name);
                }
            }
        } else {
            position = crate::world::rank::rank_read(ty, &name);
        }

        result.insert("all".into(), Dynamic::from(all));
        result.insert("all_text".into(), Dynamic::from(all_text));
        result.insert("name".into(), Dynamic::from(name));
        result.insert("position".into(), Dynamic::from(position));
        result.insert("found".into(), Dynamic::from(!all && position > 0));
        result
    });
    // Templates are owned by the Rhai event. This helper only applies the
    // Python rank-selection rule and replaces its two data placeholders.
    let rank_render_words = words_vec.clone();
    let rank_render_player_name = player_name.clone();
    engine.register_fn(
        "rank_render",
        move |limit: i64, ty: &str, success: &str, missing: &str| -> String {
            let mut name = rank_render_player_name.clone();
            let position: i64;
            if rank_render_words.len() == 3 {
                let requested = rank_render_words[1].trim();
                if requested == "모두" {
                    return crate::world::rank::rank_get_all(ty);
                }
                let number = parse_int(requested);
                if number > 0 {
                    position = number.min(limit);
                    name = crate::world::rank::rank_get_num(ty, position)
                        .unwrap_or_else(|| format!("[{position}위]"));
                } else {
                    name = requested.to_string();
                    position = crate::world::rank::rank_read(ty, &name);
                }
            } else {
                position = crate::world::rank::rank_read(ty, &name);
            }
            let template = if position > 0 { success } else { missing };
            template
                .replace("[순위자]", &name)
                .replace("[순위]", &position.to_string())
        },
    );
    engine.register_fn("set_position", move |zone: &str, room: &str| unsafe {
        let zone = if mob_difficulty.is_empty() {
            zone.to_string()
        } else {
            format!("{zone}{mob_difficulty}")
        };
        *pos_ptr = Some((zone, room.to_string()));
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
        let mut roll = |min: i64, max: i64| fastrand::i64(min..=max);
        give_event_item_with_roll(b, index, cnt, 0, &mut roll);
    });
    // Python getStrCnt() selects one candidate with randint when an event
    // `$아이템주기` directive contains several item indices.  Event Rhai has
    // its own engine, so expose the same inclusive range helper here too.
    engine.register_fn("random", |min: i64, max: i64| -> i64 {
        fastrand::i64(min..=max)
    });
    engine.register_fn("get_tendency", move |t: &str| -> bool {
        let b = unsafe { &*body_ptr };
        get_tendency(b, t)
    });
    engine.register_fn("has_item", move |index: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_has_item_spec(b, index)
    });
    // Python `$아이템착용확인` delegates to
    // `checkItemIndex(index, count, checkInUse=True)`: money keeps its
    // normal balance rule, while ordinary items must be individually worn.
    engine.register_fn("has_equipped_item", move |index: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_has_equipped_item_spec(b, index)
    });
    // The remaining legacy item directives use Python Body.getItemName(): an
    // exact, ANSI-stripped display-name lookup that includes equipped items.
    // Keep selection/state here; the event's success and failure prose stays
    // in Rhai.
    engine.register_fn("item_kind_is", move |name: &str, kind: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_item_named(b, name)
            .is_some_and(|item| item.lock().is_ok_and(|item| item.getString("종류") == kind))
    });
    engine.register_fn("item_is_equipped", move |name: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_item_named(b, name)
            .is_some_and(|item| item.lock().is_ok_and(|item| item.getBool("inUse")))
    });
    engine.register_fn("item_has_extension", move |name: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_item_named(b, name).is_some_and(|item| {
            item.lock()
                .is_ok_and(|item| !item.getString("확장 이름").is_empty())
        })
    });
    engine.register_fn("clear_item_extension", move |name: &str| -> bool {
        let b = unsafe { &mut *body_ptr };
        let Some(item_arc) = body_item_named_for_mutation(b, name) else {
            return false;
        };
        let Ok(mut item) = item_arc.lock() else {
            return false;
        };
        let extension = item.getString("확장 이름");
        if extension.is_empty() {
            drop(item);
            if crate::script::inventory_compat::absorb_pristine_object(&mut b.object, &item_arc) {
                b.object.remove(&item_arc);
            }
            return false;
        }
        // Python `list.remove()` removes only the first matching alias.
        // Do not filter all equal aliases: old saves (or a pre-existing
        // alias equal to the engraving) may legitimately contain a second
        // identical value that must remain after the engraving is erased.
        let mut names = item
            .getString("반응이름")
            .split('\n')
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if let Some(position) = names.iter().position(|candidate| candidate == &extension) {
            names.remove(position);
        }
        let names = names.join("\n");
        item.set("반응이름", names);
        item.set("확장 이름", "");
        true
    });
    engine.register_fn(
        "set_item_extension",
        move |name: &str, extension: &str| -> bool {
            let b = unsafe { &mut *body_ptr };
            if extension.is_empty() {
                return false;
            }
            let Some(item) = body_item_named_for_mutation(b, name) else {
                return false;
            };
            let Ok(mut item) = item.lock() else {
                return false;
            };
            item.set("확장 이름", extension);
            item.setAttr("아이템속성", "팔지못함");
            // Python `$아이템확장설정` uses Object.setAttr/`list.append`,
            // so both fields must persist as JSON arrays even though Rust
            // keeps their values newline-delimited at runtime.
            mark_item_field_as_json_array(&mut item, "아이템속성");
            // Python writes `item['반응이름'].append(extension)`, rather
            // than Object.setAttr().  It deliberately preserves duplicate
            // aliases; the matching `clear_item_extension` above removes
            // only the appended first match.
            let names = item.getString("반응이름");
            let names = if names.is_empty() {
                extension.to_string()
            } else {
                format!("{names}\n{extension}")
            };
            item.set("반응이름", names);
            mark_item_field_as_json_array(&mut item, "반응이름");
            true
        },
    );
    engine.register_fn("item_exists_named", move |name: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_item_named(b, name).is_some()
    });
    // Dynamic `$아이템착용확인[!] $변수:1` is intentionally unlike the
    // ordinary index form.  Python calls `checkItemName(name, count, True)`,
    // whose `True` means *do not exclude* worn objects; it therefore counts
    // both worn and unworn display-name matches.
    engine.register_fn("has_named_item", move |name: &str, count: i64| -> bool {
        if count < 1 {
            return false;
        }
        let b = unsafe { &*body_ptr };
        if name == "은전" || name == "금전" {
            return b.get_int(name) >= count;
        }
        let individual = b
            .object
            .objs
            .iter()
            .filter(|item| {
                item.lock()
                    .is_ok_and(|item| strip_event_ansi(&item.getName()) == name)
            })
            .count() as i64;
        individual.saturating_add(body_stack_named_count(b, name)) >= count
    });
    // Python `$아이템확인! $변수:1` calls `checkItemName(name, cnt)` with
    // its default `checkInUse=False`; unlike `checkItemIndex`, that excludes
    // worn objects.  `$아이템착용확인!` is the distinct directive that passes
    // `True` and accepts both states.
    engine.register_fn(
        "item_exists_unworn_named",
        move |name: &str, count: i64| -> bool {
            if count < 1 {
                return false;
            }
            let b = unsafe { &*body_ptr };
            if name == "은전" || name == "금전" {
                return b.get_int(name) >= count;
            }
            let individual = b
                .object
                .objs
                .iter()
                .filter(|item| {
                    item.lock().is_ok_and(|item| {
                        !item.getBool("inUse") && strip_event_ansi(&item.getName()) == name
                    })
                })
                .count() as i64;
            individual.saturating_add(body_stack_named_count(b, name)) >= count
        },
    );
    engine.register_fn("item_use_count", move |name: &str| -> i64 {
        let b = unsafe { &*body_ptr };
        body_item_use_count(b, name)
    });
    engine.register_fn("learnable_item_skill_count", move |name: &str| -> i64 {
        let b = unsafe { &*body_ptr };
        body_learnable_item_skill_count(b, name)
    });
    engine.register_fn("clear_item_options", move |name: &str| -> bool {
        let b = unsafe { &mut *body_ptr };
        let Some(item) = body_item_named_for_mutation(b, name) else {
            return false;
        };
        let Ok(mut item) = item.lock() else {
            return false;
        };
        // Python Item.delOption deletes both keys when the item exists.  Do
        // not reconstruct presentation or equipment text in this helper.
        item.attr.remove("아이템속성");
        item.attr.remove("옵션");
        true
    });
    engine.register_fn("item_has_options", move |name: &str| -> bool {
        let b = unsafe { &*body_ptr };
        body_item_named(b, name).is_some_and(|item| {
            item.lock()
                .is_ok_and(|item| !item.getString("옵션").is_empty())
        })
    });
    engine.register_fn("delete_item_named", move |name: &str| -> bool {
        let b = unsafe { &mut *body_ptr };
        let Some(item) = body_item_named_for_mutation(b, name) else {
            return false;
        };
        // Python `$아이템삭제 $변수:1` is `self.remove(item)`, not a
        // discard command.  In particular it does *not* release ONEITEM's
        // global owner record when an event consumes a unique item.
        b.object.remove(&item);
        true
    });
    // `$속성템주기` uses Python getStrCnt's full random candidate list, then
    // Body.addItem(index, 1, gamble=1).  The source list remains data-owned
    // in the original mob definition instead of being shortened in Rust.
    engine.register_fn("give_lottery_attribute_item", move || -> bool {
        let b = unsafe { &mut *body_ptr };
        let Some(index) = lottery_attribute_item_index() else {
            return false;
        };
        let Some((item, _)) = object_from_item_json(&index) else {
            return false;
        };
        let mut one_item_index = None;
        if let Ok(mut item_value) = item.lock() {
            let mut roll = |min: i64, max: i64| fastrand::i64(min..=max);
            let _ = crate::script::apply_item_magic_with_roll(
                &mut item_value,
                b.get_int("레벨"),
                0,
                true,
                &mut roll,
            );
            item_value.setAttr("아이템속성", "버리지못함");
            item_value.setAttr("아이템속성", "줄수없음");
            mark_item_field_as_json_array(&mut item_value, "아이템속성");
            if item_value.checkAttr("아이템속성", "단일아이템") {
                one_item_index = Some(item_value.getString("인덱스"));
            }
        }
        if !crate::script::inventory_compat::store_acquired_object(&mut b.object, item, true) {
            return false;
        }
        if let Some(index) = one_item_index.filter(|index| !index.is_empty()) {
            let _ = crate::oneitem::oneitem_have(&index, &b.get_name());
        }
        crate::script::item_effects::refresh(b);
        true
    });
    engine.register_fn("get_body_text", move |key: &str| -> String {
        let b = unsafe { &*body_ptr };
        b.get_string(key)
    });
    engine.register_fn("set_body_text", move |key: &str, value: &str| {
        let b = unsafe { &mut *body_ptr };
        b.set(key, value);
    });
    let word_count = words_vec.len() as i64;
    engine.register_fn("word_count", move || word_count);
    // `$별호변경` uses the same global registry as the ordinary nickname
    // command.  The event script retains Python's validation and every user
    // message; these functions only commit registry state.
    engine.register_fn("nickname_exists", crate::world::nickname::nickname_exists);
    engine.register_fn("nickname_reserve", crate::world::nickname::nickname_reserve);
    engine.register_fn("nickname_release", crate::world::nickname::nickname_release);
    engine.register_fn("request_event_command", move |command: &str| {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut().insert(
            crate::script::EVENT_COMMAND_REQUEST.to_string(),
            Value::String(command.to_string()),
        );
    });
    engine.register_fn(
        "item_attack_below",
        move |index: &str, limit: i64| -> bool {
            let b = unsafe { &*body_ptr };
            b.object
                .find_by_index(index)
                .and_then(|item| item.lock().ok().map(|item| item.getInt("공격력")))
                .or_else(|| {
                    (b.object.inv_stack.get(index).copied().unwrap_or(0) > 0)
                        .then(|| object_from_item_json(index))
                        .flatten()
                        .and_then(|(item, _)| item.lock().ok().map(|item| item.getInt("공격력")))
                })
                .is_some_and(|attack| attack < limit)
        },
    );
    engine.register_fn("is_olsuk_complete", move || -> bool {
        let b = unsafe { &*body_ptr };
        b.get_int("올숙완료") == 1
    });
    engine.register_fn("has_olsuk_qualification", move || -> bool {
        let b = unsafe { &*body_ptr };
        (1..=5).all(|weapon_type| b.get_int(&format!("{weapon_type} 숙련도")) >= 1_000)
    });
    // Python `$무공확인 이름`: 이벤트 분기에서 습득한 무공 여부를 확인한다.
    // 아이템 조건과 달리 `skill_list`가 저장 순서와 중복 없는 실제 무공 목록이다.
    engine.register_fn("has_skill", move |name: &str| -> bool {
        let b = unsafe { &*body_ptr };
        b.skill_list.iter().any(|skill| skill == name)
    });
    engine.register_fn("has_all_skills", move |names: &str| -> bool {
        let b = unsafe { &*body_ptr };
        let mut requested = names.split_whitespace();
        requested.clone().next().is_some()
            && requested.all(|name| b.skill_list.iter().any(|skill| skill == name))
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
            // Python `addMugong()` changes only skillList.  In particular a
            // later re-teach after `$무공회수` must retain the old skillMap
            // training record rather than resetting it to 초급/0.
            b.sync_skill_state_to_attrs();
        }
    });
    engine.register_fn("remove_skill", move |name: &str| {
        let b = unsafe { &mut *body_ptr };
        // Python `delMugong()` is list.remove(): remove one occurrence only.
        // It deliberately leaves skillMap untouched, so the old training
        // record survives a later re-teach/save cycle.
        if let Some(index) = b.skill_list.iter().position(|skill| skill == name) {
            b.skill_list.remove(index);
        }
        b.sync_skill_state_to_attrs();
    });
    engine.register_fn("vision_training_allowed", |name: &str, allowed: &str| {
        allowed
            .split_whitespace()
            .any(|candidate| candidate == name)
    });
    // These cover the remaining Python vision directives even where the
    // current source data has no active caller.  Keep all branch prose in
    // Rhai; the helpers are only the exact Body attribute transitions.
    engine.register_fn("add_vision_name", move |name: &str| {
        let b = unsafe { &mut *body_ptr };
        b.add_secret_skill(name);
    });
    engine.register_fn("vision_training_is_empty", move || -> bool {
        let b = unsafe { &*body_ptr };
        b.get_string("비전수련").is_empty()
    });
    engine.register_fn(
        "vision_training_equals",
        move |left: &str, right: &str| -> bool { left == right },
    );
    engine.register_fn("set_vision_training_name", move |name: &str| {
        let b = unsafe { &mut *body_ptr };
        b.set_vision_training(name, 0);
    });
    engine.register_fn("clear_vision_training_name", move || {
        let b = unsafe { &mut *body_ptr };
        b.clear_vision_training();
    });
    // Python `$난이도재진입확인` treats a missing value as zero and compares
    // against a five-minute wall-clock window.  Expose the predicate so the
    // Rhai event owns the normal/inverted branch layout.
    engine.register_fn("record_difficulty_entry", move || {
        let b = unsafe { &mut *body_ptr };
        b.set("난이도진입시간", chrono::Utc::now().timestamp());
    });
    engine.register_fn("difficulty_reentry_expired", move || -> bool {
        let b = unsafe { &mut *body_ptr };
        let saved = b.get_string("난이도진입시간");
        let entered = if saved.is_empty() {
            b.set("난이도진입시간", 0_i64);
            0
        } else {
            saved.parse::<i64>().unwrap_or(0)
        };
        chrono::Utc::now().timestamp() > entered.saturating_add(60 * 5)
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
    // Python `$기연확인[!]` stores the bare owner token in
    // `sub['[기연소지자]']`.  Keep this separate from `one_item_owner()`,
    // whose particle-bearing value is for converted `(이/가)` prose.
    engine.register_fn("one_item_owner_raw", move |index: &str| -> String {
        crate::oneitem::oneitem_get(index)
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
    });
    engine.register_fn("one_item_exists_name", move |name: &str| -> bool {
        let index = crate::oneitem::oneitem_get_index_by_name(name);
        !index.is_empty() && !crate::oneitem::oneitem_get(&index).is_empty()
    });
    engine.register_fn("one_item_owner_name", move |name: &str| -> String {
        let index = crate::oneitem::oneitem_get_index_by_name(name);
        // `$기연존재확인` puts the bare owner in `[기연소지자]`; the
        // converted prose supplies its own following particle (`에게`).
        crate::oneitem::oneitem_get(&index)
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
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
    // Python `Mob.setAct("리젠후생성")` calls doRegen() immediately.  This
    // differs from `리젠`, which only moves a corpse to the regen wait state.
    engine.register_fn("respawn_selected_mob", move || {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut()
            .insert("_event_selected_mob_respawn".to_string(), Value::Int(1));
    });
    engine.register_fn("start_selected_mob_combat", move || {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut().insert(
            "_event_selected_mob_start_combat".to_string(),
            Value::Int(1),
        );
    });
    // Python `$전투시작` distinguishes a dead target, an already selected
    // target, a target fighting someone else, and the caller fighting another
    // target.  The Rhai script renders each message; this helper only returns
    // the state transition outcome and sets the deferred combat marker.
    engine.register_fn("try_start_selected_mob_combat", move || -> String {
        let b = unsafe { &mut *body_ptr };
        let selected_is_corpse = matches!(
            b.temp().get("_event_selected_mob_corpse"),
            Some(Value::Int(1))
        );
        if selected_is_corpse {
            return "dead".to_string();
        }
        let selected_is_target = matches!(
            b.temp().get("_event_selected_mob_targeted_by_player"),
            Some(Value::Int(1))
        );
        let selected_is_fighting = matches!(
            b.temp().get("_event_selected_mob_fighting"),
            Some(Value::Int(1))
        );
        if selected_is_fighting {
            return if selected_is_target {
                "already_attacking"
            } else {
                "target_busy"
            }
            .to_string();
        }
        if b.act == crate::player::ActState::Fight {
            return if selected_is_target {
                "already_attacking"
            } else {
                "self_busy"
            }
            .to_string();
        }
        b.temp_mut().insert(
            "_event_selected_mob_start_combat".to_string(),
            Value::Int(1),
        );
        "started".to_string()
    });
    // `$전투강제시작` is deliberately less restrictive than `$전투시작`:
    // Python does not reject a target merely because it is already fighting
    // someone else.  It only applies the corpse/self-fight guards below.
    engine.register_fn("force_start_selected_mob_combat", move || -> String {
        let b = unsafe { &mut *body_ptr };
        if matches!(
            b.temp().get("_event_selected_mob_corpse"),
            Some(Value::Int(1))
        ) {
            return "dead".to_string();
        }
        let selected_is_target = matches!(
            b.temp().get("_event_selected_mob_targeted_by_player"),
            Some(Value::Int(1))
        );
        if b.act == crate::player::ActState::Fight {
            return if selected_is_target {
                "already_attacking"
            } else {
                "self_busy"
            }
            .to_string();
        }
        b.temp_mut().insert(
            "_event_selected_mob_start_combat".to_string(),
            Value::Int(1),
        );
        "started".to_string()
    });
    engine.register_fn("set_selected_mob_corpse", move || {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut()
            .insert("_event_selected_mob_set_corpse".to_string(), Value::Int(1));
    });
    let stat_prompt_out_ptr = out_ptr;
    engine.register_fn("change_stat", move |key: &str, amount: i64| {
        let b = unsafe { &mut *body_ptr };
        b.set(key, b.get_int(key).saturating_add(amount));
        // Python `$특성치변경` calls `lpPrompt()` after every individual
        // attribute update. Keep only the wire boundary here; client.rs owns
        // the actual prompt rendering.
        unsafe {
            (*stat_prompt_out_ptr).push(EVENT_LP_PROMPT_MARKER.to_string());
        }
    });
    engine.register_fn("set_stat", move |key: &str, value: i64| {
        let b = unsafe { &mut *body_ptr };
        b.set(key, value);
    });
    // Python `$속성설정` writes the literal key as integer 1 when nonempty.
    engine.register_fn("set_body_flag", move |key: &str| {
        if key.is_empty() {
            return;
        }
        let b = unsafe { &mut *body_ptr };
        b.set(key, 1_i64);
    });
    // `$특성치복사저/복사/복사고` changes the selected runtime mob, not
    // its immutable template.  Store only the requested variant here; the
    // caller applies it to the selected instance after Rhai returns.
    engine.register_fn("copy_player_stats_to_selected_mob", move |variant: &str| {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut().insert(
            "_event_selected_mob_copy_stats".to_string(),
            Value::String(variant.to_string()),
        );
    });
    // Python `$중급수련` is intentionally not one of the full stat-copy
    // variants: only strength, level and evasion on the selected sparring
    // mob change.
    engine.register_fn("apply_intermediate_training_to_selected_mob", move || {
        let b = unsafe { &mut *body_ptr };
        b.temp_mut().insert(
            "_event_selected_mob_copy_stats".to_string(),
            Value::String("intermediate".to_string()),
        );
    });
    engine.register_fn("consume_hp", move |amount: i64| {
        let b = unsafe { &mut *body_ptr };
        // Python `$체력소모`/`$체력감소` delegates to `minusHP()`.  A
        // lethal event damage immediately enters Player.die() state, rather
        // than merely leaving a zero-HP standing character until a later
        // combat heartbeat notices it.
        if b.minus_hp(amount) {
            b.act = crate::player::ActState::Death;
            b.unwear_all();
            b.clear_targets_death();
            b.clear_skills();
            b.set_death_step(0);
            crate::script::combat_commands::queue_combat_presentation_event(
                b,
                serde_json::json!({ "kind": "player_death" }),
            );
            b.temp_mut()
                .insert(EVENT_DEATH_FINISH_REQUEST.to_string(), Value::Int(1));
        }
    });
    engine.register_fn("consume_mp", move |amount: i64| {
        let b = unsafe { &mut *body_ptr };
        // This deliberately retains the Python `Body.minusMP()` assignment
        // order: it briefly writes zero then writes `cc`, so a directive can
        // leave negative current MP.  Combat costs use their own guards and
        // do not call this event-only helper.
        b.set("내공", b.get_int("내공").saturating_sub(amount));
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
        if r > 0 {
            // `objs/player.py:setGiIn()` runs under Python 3, so `/` keeps
            // the fractional value in the persisted attribute.  `getInt()`
            // truncates it only at later integer consumers; truncating here
            // would make the immediately saved Rust character differ.
            b.set("맷집", b.get_int("맷집") as f64 * 2.0 / 3.0);
        } else {
            b.set("맷집", 15_i64);
        }
        b.set("레벨", 1);
        b.set("현재경험치", 0);
        b.set("힘경험치", 0);
        b.set("맷집경험치", 0);
        b.set("기존성격", b.get_string("성격"));
        b.set("성격", "기인");
        b.set("내공증진아이템리스트", "");
        // Player.setGiIn() assigns this field directly.  Do not retain
        // unrelated quest flags that preceded `$소오강호설정`; the following
        // Rhai `$이벤트설정 소오강호진짜끝` may then append its one source
        // mandated completion flag.
        b.set("이벤트설정리스트", "소오강호끝");
    });
    engine.register_fn("set_sunin", move || {
        let b = unsafe { &mut *body_ptr };
        b.set("기존성격", b.get_string("성격"));
        b.set("성격", "선인");
        b.set("내공증진아이템리스트", "");
        // Python Player.setSunIn() replaces, rather than appends to, the
        // event list at ascension time.
        b.set("이벤트설정리스트", "우화등선끝");
    });
    // Python Player.setEunDun(): unlike `set_giin`, 맷집 is retained and the
    // transfer count advances once.  The event still owns the later movement
    // to 전직:1.
    engine.register_fn("set_eundun", move || {
        let b = unsafe { &mut *body_ptr };
        b.set("힘", (b.get_int("힘") - 2_000).max(15));
        b.set("레벨", 1);
        b.set("현재경험치", 0);
        b.set("힘경험치", 0);
        b.set("맷집경험치", 0);
        b.set("기존성격", b.get_string("성격"));
        b.set("성격", "은둔칩거");
        b.set("내공증진아이템리스트", "");
        b.set("이벤트설정리스트", "은둔칩거끝");
        b.set("전직", b.get_int("전직").saturating_add(1));
        b.set("위치각인", "낙양성:1");
    });
    engine.register_fn("words", move |i: i64| -> String {
        words_vec.get(i as usize).cloned().unwrap_or_default()
    });
    // Python `$스크립트호출` hands the interactive flow to a data/script
    // program.  Rhai events use the same result path instead of embedding
    // prompt/output handling in the event engine.
    engine.register_fn(
        "start_script",
        |name: &str| -> Result<(), Box<EvalAltResult>> {
            let mut m = Map::new();
            m.insert("type".into(), Dynamic::from("event_start_script"));
            m.insert("script_name".into(), Dynamic::from(name.to_string()));
            Err(Box::new(EvalAltResult::ErrorRuntime(
                Dynamic::from(m),
                Position::default(),
            )))
        },
    );
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
            room_broadcast_lines: out_room_broadcast_lines,
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
                            room_broadcast_lines: out_room_broadcast_lines,
                            mob_key: mob_key.to_string(),
                            event_key: event_key.to_string(),
                            words: words.to_vec(),
                            line_num: 0,
                            prompt,
                            resume_func: Some(next_func),
                        };
                    }
                    if t == "event_start_script" {
                        let script_name: String = m
                            .get("script_name")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                            .unwrap_or_default();
                        return CommandResult::StartScript {
                            script_name,
                            lines: vec![],
                            use_rhai: true,
                        };
                    }
                    if t == "event_complete" {
                        return CommandResult::MobEvent {
                            output_lines: out_lines,
                            set_position: out_set_position,
                            broadcast_lines: out_broadcast_lines,
                            room_broadcast_lines: out_room_broadcast_lines,
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
                        room_broadcast_lines: out_room_broadcast_lines,
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
        let mut candidates: Vec<(u64, String, String, bool, bool, RawMobData)> = Vec::new();
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
                    inst.act == 1,
                    data.clone(),
                ));
            }
        }
        candidates
    };
    if let Some(number) = corpse_number {
        candidates = candidates.into_iter().nth(number - 1).into_iter().collect();
    } else {
        candidates.sort_by_key(|(_, _, mob_name, _, _, _)| std::cmp::Reverse(mob_name.len()));
    }

    let words_ref: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
    if candidates.is_empty() {
        info!(
            "[try_mob_event] no candidates words={:?} zone={} room={}",
            words_ref, zone, room
        );
    }
    if let Some((instance_id, mob_key, mob_name, corpse, fighting, data)) = candidates.first() {
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
        if *fighting {
            body.temp_mut()
                .insert("_event_selected_mob_fighting".to_string(), Value::Int(1));
        }
        if crate::script::combat_commands::combat_target_instance_ids(body).contains(instance_id) {
            body.temp_mut().insert(
                "_event_selected_mob_targeted_by_player".to_string(),
                Value::Int(1),
            );
        }
        let result = do_event(body, data, &event_key, &words, mob_key, None, None);
        body.temp_mut().remove("_event_selected_mob_corpse");
        body.temp_mut().remove("_event_selected_mob_fighting");
        body.temp_mut()
            .remove("_event_selected_mob_targeted_by_player");
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
        let respawn = body
            .temp_mut()
            .remove("_event_selected_mob_respawn")
            .is_some();
        let copied_stats = body
            .temp_mut()
            .remove("_event_selected_mob_copy_stats")
            .and_then(|value| value.as_str().map(str::to_string));
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
        if let Some(variant) = copied_stats {
            if variant == "intermediate" {
                if let Ok(mut world) = get_world_state().write() {
                    if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) {
                        if let Some(mob) =
                            mobs.iter_mut().find(|mob| mob.instance_id == *instance_id)
                        {
                            mob.strength = body.get_int("힘").div_euclid(100).saturating_add(1);
                            mob.level = body.get_int("레벨").saturating_add(150);
                            mob.miss = 0;
                            mob.runtime_attrs.insert("회피".to_string(), Value::Int(0));
                        }
                    }
                }
            }
            let hp_multiplier = match variant.as_str() {
                "low" => 1,
                "normal" => 20,
                "high" => 30,
                _ => 0,
            };
            if hp_multiplier > 0 {
                let strength_multiplier = match variant.as_str() {
                    "low" => 1,
                    "normal" => 3,
                    "high" => 8,
                    _ => unreachable!(),
                };
                let arm_multiplier = if variant == "high" { 3 } else { 1 };
                let hp = body.get_int("최고체력").saturating_mul(hp_multiplier);
                let strength = body.get_int("힘").saturating_mul(strength_multiplier);
                let level = body.get_int("레벨").saturating_add(150);
                let arm = body.get_int("맷집").saturating_mul(arm_multiplier);
                let agility = body.get_int("민첩성");
                if let Ok(mut world) = get_world_state().write() {
                    if let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) {
                        if let Some(mob) =
                            mobs.iter_mut().find(|mob| mob.instance_id == *instance_id)
                        {
                            mob.hp = hp;
                            mob.max_hp = hp;
                            mob.strength = strength;
                            mob.level = level;
                            mob.arm = arm;
                            mob.agility = agility;
                            mob.miss = 0;
                            mob.hit = 400;
                            mob.luck = 100;
                            mob.critical = 100;
                            for (key, value) in
                                [("회피", 0), ("명중", 400), ("운", 100), ("필살", 100)]
                            {
                                mob.runtime_attrs.insert(key.to_string(), Value::Int(value));
                            }
                        }
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
        if respawn {
            if let Ok(mut world) = get_world_state().write() {
                // Clone first so `respawn` can mutate the instance without
                // overlapping an immutable cache borrow.
                let data = world
                    .mob_cache
                    .get_all_mobs_in_room(zone, room)
                    .into_iter()
                    .find(|mob| mob.instance_id == *instance_id)
                    .and_then(|mob| world.mob_cache.get_instance_data(mob).cloned());
                if let (Some(data), Some(mobs)) =
                    (data, world.mob_cache.get_all_mobs_in_room_mut(zone, room))
                {
                    if let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == *instance_id) {
                        mob.respawn(&data);
                    }
                }
            }
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

/// Python `Body.addItem`을 사용하는 `$아이템주기`의 상태 변경 부분.
///
/// Python은 요청한 전체 수량이 정확히 하나일 때만 `applyMagic(level, 0, 1)`을
/// 적용한다. `gamble`은 `$속성템주기`가 쓰는 버리지/거래 불가 표식이다.
fn give_event_item_with_roll(
    body: &mut Body,
    index: &str,
    cnt: i64,
    gamble: i64,
    roll: &mut dyn FnMut(i64, i64) -> i64,
) {
    if index == "은전" || index == "금전" {
        body.set(index, body.get_int(index) + cnt);
        return;
    }
    for _ in 0..cnt {
        let Some((arc, _)) = object_from_item_json(index) else {
            continue;
        };
        let is_one_item = if let Ok(mut item) = arc.lock() {
            if cnt == 1 {
                let _ = crate::script::apply_item_magic_with_roll(
                    &mut item,
                    body.get_int("레벨"),
                    0,
                    true,
                    roll,
                );
                if gamble != 0 {
                    item.setAttr("아이템속성", "버리지못함");
                    item.setAttr("아이템속성", "줄수없음");
                    mark_item_field_as_json_array(&mut item, "아이템속성");
                }
            }
            item.checkAttr("아이템속성", "단일아이템")
        } else {
            false
        };
        let accepted =
            crate::script::inventory_compat::store_acquired_object(&mut body.object, arc, true);
        if accepted && is_one_item {
            let _ = crate::oneitem::oneitem_have(index, &body.get_name());
        }
    }
    crate::script::item_effects::refresh(body);
}

/// $아이템삭제: 은전/금전이면 속성 감소, objs에서 인덱스 일치 cnt개 제거, 부족하면 inv_stack에서 차감.
fn del_item_from_body(body: &mut Body, index: &str, cnt: i64) {
    if index == "은전" {
        body.set("은전", body.get_int("은전") - cnt);
        return;
    }
    if index == "금전" {
        body.set("금전", body.get_int("금전") - cnt);
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
    crate::script::item_effects::refresh(body);
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

/// Python `Body.checkItemIndex(index, cnt, checkInUse=True)` for event
/// directives.  Stacked inventory entries cannot be equipped; currency keeps
/// the early-return balance behavior of the Python method.
fn body_has_equipped_item_spec(body: &Body, spec: &str) -> bool {
    let mut parts = spec.rsplitn(2, char::is_whitespace);
    let tail = parts.next().unwrap_or_default();
    let (index, required) = match (parts.next(), tail.parse::<i64>()) {
        (Some(index), Ok(required)) if required > 0 => (index.trim_end(), required),
        _ => (spec, 1),
    };
    if index == "은전" || index == "금전" {
        return body.get_int(index) >= required;
    }
    body.object
        .objs
        .iter()
        .filter(|item| {
            item.lock()
                .is_ok_and(|item| item.getString("인덱스") == index && item.getBool("inUse"))
        })
        .count() as i64
        >= required
}

/// Python `Body.getItemName` is deliberately narrower than the general
/// command selector: it scans inventory insertion order and compares the
/// stripped display name exactly, while still seeing worn equipment.
fn body_item_named(body: &Body, name: &str) -> Option<Arc<Mutex<Object>>> {
    body.object
        .objs
        .iter()
        .find_map(|item| {
            let item_name = item.lock().ok()?.getName();
            (strip_event_ansi(&item_name) == name).then(|| item.clone())
        })
        .or_else(|| {
            let mut keys = body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys.into_iter().find_map(|key| {
                if body.object.inv_stack.get(&key).copied().unwrap_or(0) <= 0 {
                    return None;
                }
                let (item, _) = object_from_item_json(&key)?;
                let matches = item
                    .lock()
                    .ok()
                    .is_some_and(|item| strip_event_ansi(&item.getName()) == name);
                matches.then_some(item)
            })
        })
}

fn body_item_named_for_mutation(body: &mut Body, name: &str) -> Option<Arc<Mutex<Object>>> {
    if let Some(item) = body.object.objs.iter().find_map(|item| {
        let item_name = item.lock().ok()?.getName();
        (strip_event_ansi(&item_name) == name).then(|| item.clone())
    }) {
        return Some(item);
    }
    let mut keys = body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    let key = keys.into_iter().find(|key| {
        body.object.inv_stack.get(key).copied().unwrap_or(0) > 0
            && object_from_item_json(key).is_some_and(|(item, _)| {
                item.lock()
                    .ok()
                    .is_some_and(|item| strip_event_ansi(&item.getName()) == name)
            })
    })?;
    crate::script::inventory_compat::materialize_one(&mut body.object, &key, true)
}

fn body_stack_named_count(body: &Body, name: &str) -> i64 {
    body.object
        .inv_stack
        .iter()
        .filter_map(|(key, count)| {
            object_from_item_json(key).and_then(|(item, _)| {
                item.lock()
                    .ok()
                    .and_then(|item| (strip_event_ansi(&item.getName()) == name).then_some(*count))
            })
        })
        .sum()
}

/// Python's `[아이템사용횟수]` event placeholder: find the first unworn item
/// by its stripped display name, then read `itemSkillMap` using that item's
/// original (ANSI-preserving) name.
fn body_item_use_count(body: &Body, name: &str) -> i64 {
    body.object
        .objs
        .iter()
        .find_map(|item| {
            let item = item.lock().ok()?;
            (!item.getBool("inUse") && strip_event_ansi(&item.getName()) == name)
                .then(|| item.getName())
        })
        .or_else(|| {
            let mut keys = body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys.into_iter().find_map(|key| {
                if body.object.inv_stack.get(&key).copied().unwrap_or(0) <= 0 {
                    return None;
                }
                object_from_item_json(&key).and_then(|(item, _)| {
                    item.lock().ok().and_then(|item| {
                        (strip_event_ansi(&item.getName()) == name).then(|| item.getName())
                    })
                })
            })
        })
        .and_then(|raw_name| body.item_skill_map.get(&raw_name).copied())
        .map(i64::from)
        .unwrap_or(0)
}

/// Preserve array boundaries from the authoritative item JSON for Python's
/// `[배울무공이름갯수]` placeholder.  Runtime items saved by older servers may
/// only retain a single declaration, so keep that string as a fallback.
fn item_skill_declarations(item: &Object) -> Vec<String> {
    let index = item.getString("인덱스");
    let path = format!("data/item/{index}.json");
    if let Ok(source) = std::fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&source) {
            if let Some(value) = json.pointer("/아이템정보/무공이름") {
                if let Some(values) = value.as_array() {
                    return values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect();
                }
                if let Some(value) = value.as_str() {
                    return vec![value.to_string()];
                }
            }
        }
    }
    let raw = item.getString("무공이름");
    if raw.is_empty() {
        Vec::new()
    } else {
        raw.split('|').map(str::to_string).collect()
    }
}

/// Python `[배울무공이름갯수]`: count the selected unworn weapon's declared
/// skills which are not learned and are compatible with the player's 성격.
fn body_learnable_item_skill_count(body: &Body, name: &str) -> i64 {
    let item = body.object.findObjInven(name, 1).or_else(|| {
        let key =
            crate::script::inventory_compat::find_counted_item_key(&body.object.inv_stack, name)?;
        object_from_item_json(&key).map(|(item, _)| item)
    });
    let Some(item) = item else {
        return 0;
    };
    let Ok(item) = item.lock() else {
        return 0;
    };
    let personality = body.get_string("성격");
    item_skill_declarations(&item)
        .into_iter()
        .filter(|declaration| {
            let words = declaration.split_whitespace().collect::<Vec<_>>();
            let Some(skill_name) = words.first() else {
                return false;
            };
            let Some(kind) = words.get(1) else {
                return false;
            };
            !body.skill_list.iter().any(|skill| skill == *skill_name)
                && (*kind == "정사"
                    || personality == *kind
                    || personality == "기인"
                    || personality == "선인")
        })
        .count() as i64
}

/// Read the authoritative legacy `$속성템주기` candidate list. Python's
/// `getStrCnt` picks one token from positions 1..-2 and treats the final `1`
/// as the count, so retain every preceding item key exactly as authored.
fn lottery_attribute_item_index() -> Option<String> {
    let source = std::fs::read_to_string("data/mob/낙양성/복권맨.mob").ok()?;
    let line = source
        .lines()
        .find(|line| line.starts_with(":$속성템주기 "))?;
    let candidates = line
        .trim_start_matches(":$속성템주기 ")
        .split_whitespace()
        .collect::<Vec<_>>();
    if candidates.len() < 2 {
        return None;
    }
    Some(candidates[fastrand::usize(..candidates.len() - 1)].to_string())
}

fn strip_event_ansi(value: &str) -> String {
    let mut out = String::new();
    let mut escape = false;
    for ch in value.chars() {
        if escape {
            if ch == 'm' {
                escape = false;
            }
        } else if ch == '\x1b' {
            escape = true;
        } else if ch != '\u{009b}' {
            out.push(ch);
        }
    }
    out
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
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    };

    use super::{
        body_has_item_spec, check_event_key, del_item_from_body, do_event, do_event_rhai,
        get_tendency, get_user_event, give_event_item_with_roll,
    };
    use crate::command::CommandResult;
    use crate::object::{Object, Value};
    use crate::player::Body;
    use crate::world::{EventScript, MobCache, MobInstance, RawMobData, RoomCache};

    // Rank storage is process-global and persisted.  Tests that deliberately
    // fill/clear the same legacy board must not interleave under libtest.
    static RANK_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn add_test_items(body: &mut Body, index: &str, count: usize) {
        for _ in 0..count {
            let item = crate::script::object_from_item_json(index)
                .unwrap_or_else(|| panic!("missing item fixture {index}"))
                .0;
            assert!(crate::script::inventory_compat::store_acquired_object(
                &mut body.object,
                item,
                false,
            ));
        }
    }

    /// Unique-reward event tests must not inherit a prior test run's owner
    /// from the persistent ONEITEM registry.  Each test lists only the
    /// reward indices it creates, so unrelated configured unique items stay
    /// intact.
    fn clear_test_oneitems(indices: &[&str]) {
        for index in indices {
            let _ = crate::oneitem::oneitem_destroy(index);
        }
    }

    #[test]
    fn event_item_grant_preserves_python_single_count_magic_and_money_subtraction() {
        // Python Body.addItem(index, cnt) only calls applyMagic when the
        // requested cnt is exactly one.  The fixed low roll keeps the result
        // deterministic while still exercising the forced-magic path.
        let mut single = Body::new();
        single.set("레벨", 1);
        give_event_item_with_roll(&mut single, "31", 1, 0, &mut |low, _| low);
        assert_eq!(single.object.objs.len(), 1);
        assert!(
            !single.object.objs[0]
                .lock()
                .unwrap()
                .getString("옵션")
                .is_empty(),
            "a one-item event reward must receive Python's forced magic roll"
        );

        // `$속성템주기` calls addItem(..., gamble=1). Python saves both
        // Item.setOption() and the new non-trade attributes as JSON lists,
        // never newline-delimited scalars.
        let mut lottery = Body::new();
        lottery.set("이름", "이벤트속성저장회귀");
        give_event_item_with_roll(&mut lottery, "31", 1, 1, &mut |low, _| low);
        let lottery_path = "data/user/이벤트속성저장회귀.json";
        assert!(crate::script::save_body_to_json(&mut lottery, lottery_path));
        let lottery_save: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(lottery_path).expect("attribute reward save"),
        )
        .expect("valid attribute reward save");
        assert!(lottery_save["아이템"][0]["변경"]["옵션"].is_array());
        assert_eq!(
            lottery_save["아이템"][0]["변경"]["아이템속성"],
            serde_json::json!(["버리지못함", "줄수없음"])
        );
        let _ = std::fs::remove_file(lottery_path);

        let mut multiple = Body::new();
        multiple.set("레벨", 10_000);
        give_event_item_with_roll(&mut multiple, "31", 2, 0, &mut |_, high| high);
        assert_eq!(multiple.object.inv_stack.get("31"), Some(&2));
        assert!(multiple.object.objs.is_empty());

        // Python Body.delItem deliberately permits event scripts to take a
        // balance below zero; it does not clamp 은전/금전.
        multiple.set("은전", 3);
        multiple.set("금전", 2);
        del_item_from_body(&mut multiple, "은전", 5);
        del_item_from_body(&mut multiple, "금전", 5);
        assert_eq!(multiple.get_int("은전"), -2);
        assert_eq!(multiple.get_int("금전"), -3);
    }

    #[test]
    fn currently_unused_python_directive_efuns_keep_their_state_predicates() {
        // Some Python `$` handlers currently have no authored `.mob` caller,
        // but must remain executable for a future data reload.  Exercise the
        // source-level predicates through Rhai instead of treating absence of
        // a call site as proof of implementation.
        let mut body = Body::new();
        body.set("이름", "미사용동작회귀");
        body.set("내공", 10_i64);
        body.set("은전", 3_i64);
        body.set("난이도진입시간", 0_i64);
        body.set("비전이름", "기존비전");
        body.skill_list.push("시험무공".to_string());
        let mut weapon = Object::new();
        weapon.set("이름", "시험검");
        weapon.set("인덱스", "시험검");
        weapon.set("공격력", 3_999_i64);
        weapon.set("inUse", 1_i64);
        body.object.objs.push(Arc::new(Mutex::new(weapon)));
        let mut unworn_weapon = Object::new();
        unworn_weapon.set("이름", "시험검");
        unworn_weapon.set("인덱스", "시험검");
        unworn_weapon.set("inUse", 0_i64);
        body.object.objs.push(Arc::new(Mutex::new(unworn_weapon)));

        let (output, _) = run_zone_event_source(
            &mut body,
            "낙양성",
            r#"
fn event() {
    if item_attack_below("시험검", 4000) { output("below"); }
    if item_is_equipped("시험검") { output("equipped"); }
    if has_equipped_item("시험검 1") { output("equipped-count"); }
    if !has_equipped_item("시험검 2") { output("equipped-missing"); }
    // `$아이템착용확인[!] $변수:1` uses checkItemName(..., True): both
    // copies count even though only the first one is worn.
    if has_named_item("시험검", 2) { output("named-both-states"); }
    if !has_named_item("시험검", 3) { output("named-count-missing"); }
    if has_named_item("은전", 3) { output("named-money"); }
    if !has_named_item("은전", 0) { output("named-zero-rejected"); }
    if has_equipped_item("은전 3") { output("money"); }
    if has_skill("시험무공") { output("skill"); }
    // The currently-unused `$무공확인!` skips its following block only
    // when the named ordinary skill is already learned.
    if !has_skill("없는무공") { output("skill-inverse"); }
    set_body_flag("원본속성");
    consume_mp(7);
    // Python `$비전설정` calls setAttr(): it appends a new vision but
    // preserves the existing list and ignores a duplicate.
    add_vision_name("기존비전");
    add_vision_name("시험비전");
    set_vision_training_name("시험수련");
    if !vision_training_is_empty() { output("training"); }
    clear_vision_training_name();
    // `$난이도재진입확인!` continues only after the five-minute window,
    // while the non-! directive continues only while it is still fresh.
    if difficulty_reentry_expired() { output("inverse-expired"); }
    record_difficulty_entry();
    if !difficulty_reentry_expired() { output("normal-fresh"); }
    end_event();
}
"#,
            None,
        );
        assert_eq!(
            output,
            vec![
                "below",
                "equipped",
                "equipped-count",
                "equipped-missing",
                "named-both-states",
                "named-count-missing",
                "named-money",
                "named-zero-rejected",
                "money",
                "skill",
                "skill-inverse",
                "training",
                "inverse-expired",
                "normal-fresh",
            ]
        );
        assert_eq!(body.get_int("원본속성"), 1);
        assert_eq!(body.get_int("내공"), 3);
        assert_eq!(
            body.get_secret_skills(),
            vec!["기존비전".to_string(), "시험비전".to_string()],
            "Python setAttr preserves earlier visions and de-duplicates the requested one"
        );
        // Python `$비전설정` calls Object.setAttr(), so the persisted value
        // is a JSON list rather than Rust's internal separator string.
        let save_path = "data/user/미사용비전동작회귀.json";
        assert!(crate::script::save_body_to_json(&mut body, save_path));
        let saved: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(save_path).expect("saved unused vision directive body"),
        )
        .expect("valid unused vision directive body");
        assert_eq!(
            saved["사용자오브젝트"]["비전이름"],
            serde_json::json!(["기존비전", "시험비전"])
        );
        let _ = std::fs::remove_file(save_path);
        assert!(body.get_string("비전수련").is_empty());
    }

    #[test]
    fn event_position_move_prefixes_the_zone_with_the_python_mob_difficulty() {
        // `objs/event.py` inserts the selected mob's 난이도 immediately
        // before the colon in `zone:room`, i.e. it changes the zone name,
        // not the room number.  No current data mob takes this branch, so
        // keep the source-level handler covered for hot-reloaded data.
        let mut body = Body::new();
        for difficulty in [
            serde_json::Value::String("3".to_string()),
            serde_json::Value::Number(serde_json::Number::from(3)),
        ] {
            let mut data = RawMobData::new();
            data.zone = "낙양성".to_string();
            data.attributes.insert("난이도".to_string(), difficulty);
            let result = super::do_event_rhai_source(
                &mut body,
                &data,
                "test",
                &[],
                "test",
                "fn event() { set_position(\"낙양성\", \"42\"); end_event(); }",
                None,
            );
            let CommandResult::MobEvent { set_position, .. } = result else {
                panic!("difficulty position event returned a non-event result");
            };
            assert_eq!(
                set_position,
                Some(("낙양성3".to_string(), "42".to_string()))
            );
        }
    }

    #[test]
    fn event_room_output_uses_python_get_name_a_for_the_actor() {
        // `$출력` reaches Player.printScript(): the caller sees `당신`, but
        // room observers see the actor through getNameA(), including yellow
        // ANSI before the Korean subject-particle conversion.
        let mut body = Body::new();
        body.set("이름", "가람");
        let mut data = RawMobData::new();
        data.zone = "낙양성".to_string();
        let result = super::do_event_rhai_source(
            &mut body,
            &data,
            "test",
            &[],
            "test",
            r#"fn event() { room_broadcast_output("[공](이/가) 절을 합니다."); end_event(); }"#,
            None,
        );
        let CommandResult::MobEvent {
            output_lines,
            room_broadcast_lines,
            ..
        } = result
        else {
            panic!("room-output event returned a non-event result");
        };
        assert!(output_lines.is_empty());
        assert_eq!(
            room_broadcast_lines,
            vec!["\x1b[1m가람\x1b[0;37m이 절을 합니다."]
        );
    }

    #[test]
    fn event_state_directives_do_not_persist_before_the_python_save_tick() {
        // Python `doEvent()` calls setEvent/delEvent and the three transition
        // methods in memory only; Player.update() performs the later periodic
        // save.  A quest command must not overwrite an existing save file at
        // the event helper boundary.
        let name = format!("이벤트즉시저장회귀-{}", std::process::id());
        let path = format!("data/user/{name}.json");
        let sentinel = b"python-save-must-remain-unchanged";
        std::fs::write(&path, sentinel).expect("write event save sentinel");

        let scripts = [
            "fn event() { set_event(\"시험완료\", \"1\"); end_event(); }",
            "fn event() { del_event(\"시험완료\"); end_event(); }",
            "fn event() { set_giin(); end_event(); }",
            "fn event() { set_sunin(); end_event(); }",
            "fn event() { set_eundun(); end_event(); }",
        ];
        for source in scripts {
            let mut body = Body::new();
            body.set("이름", name.as_str());
            body.set("힘", 3_000_i64);
            body.set("맷집", 90_i64);
            body.set("성격", "정파");
            run_zone_event_source(&mut body, "낙양성", source, None);
            assert_eq!(
                std::fs::read(&path).expect("event helper must not replace save"),
                sentinel,
                "{source}"
            );
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stat_change_keeps_python_lp_prompt_boundary_before_later_event_text() {
        let mut body = Body::new();
        body.set("최고내공", 20_i64);
        let (output, _) = run_zone_event_source(
            &mut body,
            "낙양성",
            r#"
fn event() {
    change_stat("최고내공", 10);
    output("뒤문장");
    end_event();
}
"#,
            None,
        );

        assert_eq!(body.get_int("최고내공"), 30);
        assert_eq!(
            output,
            vec![
                super::EVENT_LP_PROMPT_MARKER.to_string(),
                "뒤문장".to_string()
            ]
        );
    }

    #[test]
    fn event_mp_loss_keeps_python_minus_mp_negative_assignment() {
        let mut body = Body::new();
        body.set("내공", 10_i64);
        run_zone_event_source(
            &mut body,
            "낙양성",
            r#"
fn event() {
    consume_mp(17);
    end_event();
}
"#,
            None,
        );
        assert_eq!(body.get_int("내공"), -7);
    }

    #[test]
    fn event_flags_round_trip_as_python_arrays_and_keep_all_loaded_entries() {
        // Python Player.save() writes `이벤트설정리스트` as a list after
        // setEvent(). Rust internally receives that list as `다|나`; both
        // entries must remain independently visible to `$이벤트확인`, and a
        // later Rust mutation must save a list again for Python.
        let name = format!("이벤트배열왕복{}", std::process::id());
        let path = format!("data/user/{name}.json");
        let mut body = Body::new();
        body.set("이름", name.as_str());
        body.set("이벤트설정리스트", "다|나");
        let (output, _) = run_zone_event_source(
            &mut body,
            "낙양성",
            r#"
fn event() {
    if check_event("다") { output("first"); }
    if check_event("나") { output("second"); }
    set_event("가", "1");
    del_event("다");
    end_event();
}
"#,
            None,
        );
        assert_eq!(output, vec!["first", "second"]);
        assert!(crate::script::save_body_to_json(&mut body, &path));

        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            saved["사용자오브젝트"]["이벤트설정리스트"],
            serde_json::json!(["나", "가"])
        );

        let verify = r#"
from client import Player
import os
name = os.environ['MUC_EVENT_ARRAY_USER']
player = Player()
assert player.load(name)
assert type(player['이벤트설정리스트']) is list
assert player.checkEvent('나') and player.checkEvent('가')
assert not player.checkEvent('다')
player.delEvent('나')
assert player['이벤트설정리스트'] == ['가']
"#;
        let result = std::process::Command::new("python3")
            .arg("-c")
            .arg(verify)
            .env("MUC_EVENT_ARRAY_USER", &name)
            .output()
            .expect("python event-array round-trip must launch");
        assert!(
            result.status.success(),
            "Python event-array round-trip failed:\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn event_rank_self_output_uses_python_dangsin_while_observers_receive_the_name() {
        // Python `$순위갱신` uses a global sendToAll1 recipient with a plain
        // character name, whereas `$출력` uses printScript/sendRoom and
        // therefore getNameA() for same-room observers. Keep all three
        // recipient renderings distinct.
        let mut body = Body::new();
        body.set("이름", "회귀자");
        let CommandResult::MobEvent {
            output_lines,
            broadcast_lines,
            room_broadcast_lines,
            ..
        } = super::do_event_rhai_source(
            &mut body,
            &RawMobData::new(),
            "self-output-regression",
            &[],
            "test",
            r#"
fn event() {
    self_output("[공](이/가) 시험합니다");
    literal_output("[공]이 원문입니다");
    broadcast_output("[공](이/가) 시험합니다");
    room_broadcast_output("[공](이/가) 시험합니다");
    end_event();
}
"#,
            None,
        )
        else {
            panic!("event output regression must complete as MobEvent");
        };
        // Python's ordinary-event path preserves the literal `[공]`, but
        // still calls postPosition1() because this text carries `(이/가)`.
        assert_eq!(output_lines, vec!["당신이 시험합니다", "[공]이 원문입니다"]);
        assert_eq!(broadcast_lines, vec!["회귀자가 시험합니다"]);
        assert_eq!(
            room_broadcast_lines,
            vec!["\x1b[1m회귀자\x1b[0;37m가 시험합니다"]
        );

        for (path, expected) in [
            ("data/script/낙양성/93_돌려_돌.rhai", 1),
            ("data/script/강소성/육자홍_절.rhai", 3),
        ] {
            let source = std::fs::read_to_string(path).expect("literal legacy event script");
            assert_eq!(
                source
                    .lines()
                    .filter(|line| line.contains("literal_output(") && line.contains("[공]"))
                    .count(),
                expected,
                "{path} must keep Python's literal ordinary-event [공] text"
            );
        }

        // These are not merely source templates: Python's ordinary doEvent
        // path preserves `[공]` but resolves its following `(이/가)` through
        // postPosition1().  Execute both authored paths to lock that odd but
        // player-visible legacy result down.
        let mut statue = Body::new();
        statue.set("이름", "석상원문회귀");
        let (statue_output, statue_position) = run_luoyang_event(&mut statue, "93_돌려_돌.rhai");
        assert_eq!(
            statue_position,
            Some(("낙양성".to_string(), "1455".to_string()))
        );
        assert!(
            statue_output
                .iter()
                .any(|line| line.contains("[공]") && line.ends_with("이 지하로 내려갑니다")),
            "ordinary statue text must retain Python's literal [공] plus resolved particle: {statue_output:?}"
        );

        clear_test_oneitems(&["황룡마조"]);
        let mut elder = Body::new();
        elder.set("이름", "육자홍원문회귀");
        for event in ["진마혁끝", "황룡마조1"] {
            super::set_user_event(&mut elder, event, "1");
        }
        let (elder_output, _) = run_zone_event(&mut elder, "강소성", "육자홍_절.rhai", None);
        assert!(
            elder_output.iter().any(|line| {
                line.contains("[공]이") && line.contains("육자홍") && line.contains("먼지가되어")
            }),
            "ordinary Yuk Jahong text must retain Python's literal [공] plus resolved particle: {elder_output:?}"
        );
        clear_test_oneitems(&["황룡마조"]);
    }

    #[test]
    fn craft_name_extension_events_keep_python_item_and_money_order() {
        // `이름맨.mob` uses `$아이템확장확인`, `$아이템종류확인`,
        // `$아이템확장설정[지움]`, and `$아이템삭제` as one transaction.
        // Execute the authored Rhai scripts with the same input layout as
        // Python's `words`: target, item, requested extension, command.
        let mut body = Body::new();
        body.set("이름", "이름새김회귀");
        body.set("은전", 1_000_000_i64);
        add_test_items(&mut body, "31", 1);

        let engrave = std::fs::read_to_string("data/script/낙양성/이름맨_이름새김.rhai")
            .expect("craft engraving script");
        let engraving_words = vec![
            "크래프트".to_string(),
            "명왕검".to_string(),
            "회귀명".to_string(),
            "이름새김".to_string(),
        ];
        let (output, _) =
            run_zone_event_source_with_words(&mut body, "낙양성", &engrave, &engraving_words, None);
        assert!(output.iter().any(|line| line.contains("이름을 새겨")));
        assert_eq!(body.get_int("은전"), 0);
        {
            let item = body.object.objs[0].lock().unwrap();
            assert_eq!(item.getString("확장 이름"), "회귀명");
            assert!(item.checkAttr("아이템속성", "팔지못함"));
            assert!(item.checkAttr("반응이름", "명왕검"));
            assert!(item.checkAttr("반응이름", "회귀명"));
        }
        // `Player.save()` must retain Python's list shape after the event:
        // `$아이템확장설정` calls setAttr for 아이템속성 and append for
        // 반응이름, neither of which is a scalar in the Python save format.
        let engrave_save_path = "data/user/이름새김회귀.json";
        assert!(crate::script::save_body_to_json(
            &mut body,
            engrave_save_path
        ));
        let engraved_save: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(engrave_save_path).expect("engraved item save"),
        )
        .expect("valid engraved item save");
        assert_eq!(
            engraved_save["아이템"][0]["변경"]["반응이름"],
            serde_json::json!(["명왕검", "회귀명"])
        );
        assert_eq!(
            engraved_save["아이템"][0]["변경"]["아이템속성"],
            serde_json::json!(["팔지못함"])
        );
        let _ = std::fs::remove_file(engrave_save_path);

        // `$아이템확장확인 이름` must reject a second engraving before it
        // mutates the item or takes another fee.
        body.set("은전", 1_000_000_i64);
        let (output, _) =
            run_zone_event_source_with_words(&mut body, "낙양성", &engrave, &engraving_words, None);
        assert!(output
            .iter()
            .any(|line| line.contains("이미 이름이 새겨진")));
        assert_eq!(body.get_int("은전"), 1_000_000);

        let erase = std::fs::read_to_string("data/script/낙양성/이름맨_이름지움_이름삭제.rhai")
            .expect("craft erasing script");
        let erase_words = vec![
            "크래프트".to_string(),
            "명왕검".to_string(),
            "이름지움".to_string(),
        ];

        // `$아이템확장확인!` enters the following rejection block when a
        // matching item has no extension.  This must happen before the
        // million-silver guard, so an ordinary unengraved weapon is neither
        // changed nor charged.
        let mut unengraved = Body::new();
        unengraved.set("이름", "이름지움미새김회귀");
        unengraved.set("은전", 1_000_000_i64);
        add_test_items(&mut unengraved, "31", 1);
        let (output, _) =
            run_zone_event_source_with_words(&mut unengraved, "낙양성", &erase, &erase_words, None);
        assert!(output
            .iter()
            .any(|line| line.contains("이름이 새겨지지 않은 장비")));
        assert_eq!(unengraved.get_int("은전"), 1_000_000);

        let (output, _) =
            run_zone_event_source_with_words(&mut body, "낙양성", &erase, &erase_words, None);
        assert!(output.iter().any(|line| line.contains("이름을 지워")));
        assert_eq!(body.get_int("은전"), 0);
        let item = body.object.objs[0].lock().unwrap();
        assert!(item.getString("확장 이름").is_empty());
        assert!(item.checkAttr("반응이름", "명왕검"));
        assert!(!item.checkAttr("반응이름", "회귀명"));

        // Python `$아이템확장설정` has its own `len(words) == 4` guard.
        // The subsequent `$아이템삭제` and source prose remain unconditional,
        // so an overlong input spends the fee and reports completion without
        // mutating the item's extension.
        let mut overlong = Body::new();
        overlong.set("이름", "이름새김초과회귀");
        overlong.set("은전", 1_000_000_i64);
        add_test_items(&mut overlong, "31", 1);
        let overlong_words = vec![
            "크래프트".to_string(),
            "명왕검".to_string(),
            "회귀명".to_string(),
            "이름새김".to_string(),
            "추가인자".to_string(),
        ];
        let (output, _) = run_zone_event_source_with_words(
            &mut overlong,
            "낙양성",
            &engrave,
            &overlong_words,
            None,
        );
        assert!(output.iter().any(|line| line.contains("이름을 새겨")));
        assert_eq!(overlong.get_int("은전"), 0);
        let overlong_item = super::body_item_named(&overlong, "명왕검").unwrap();
        let item = overlong_item.lock().unwrap();
        assert!(item.getString("확장 이름").is_empty());
        assert!(!item.checkAttr("반응이름", "회귀명"));

        // `$아이템확장설정지움` has the symmetric `len(words) == 3`
        // condition in Python.  An overlong erase command still consumes the
        // fee and emits the source prose, but retains the engraved name.
        let mut overlong_erase = Body::new();
        overlong_erase.set("이름", "이름지움초과회귀");
        overlong_erase.set("은전", 1_000_000_i64);
        add_test_items(&mut overlong_erase, "31", 1);
        {
            let selected = super::body_item_named_for_mutation(&mut overlong_erase, "명왕검")
                .expect("engraving must materialize one counted weapon");
            let mut item = selected.lock().unwrap();
            item.set("확장 이름", "회귀명");
            item.setAttr("반응이름", "회귀명");
        }
        let overlong_erase_words = vec![
            "크래프트".to_string(),
            "명왕검".to_string(),
            "이름지움".to_string(),
            "추가인자".to_string(),
        ];
        let (output, _) = run_zone_event_source_with_words(
            &mut overlong_erase,
            "낙양성",
            &erase,
            &overlong_erase_words,
            None,
        );
        assert!(output.iter().any(|line| line.contains("이름을 지워")));
        assert_eq!(overlong_erase.get_int("은전"), 0);
        let item = overlong_erase.object.objs[0].lock().unwrap();
        assert_eq!(item.getString("확장 이름"), "회귀명");
        assert!(item.checkAttr("반응이름", "회귀명"));

        // `$아이템확장설정` is Python list.append(), not setAttr(): a
        // pre-existing identical reaction name stays duplicated, and the
        // later Python list.remove() clears only one occurrence.
        let mut duplicate_alias = Body::new();
        duplicate_alias.set("이름", "이름새김중복별칭회귀");
        duplicate_alias.set("은전", 1_000_000_i64);
        add_test_items(&mut duplicate_alias, "31", 1);
        {
            let selected = super::body_item_named_for_mutation(&mut duplicate_alias, "명왕검")
                .expect("alias mutation must materialize one counted weapon");
            let mut item = selected.lock().unwrap();
            item.set("반응이름", "명왕검\n회귀명");
        }
        run_zone_event_source_with_words(
            &mut duplicate_alias,
            "낙양성",
            &engrave,
            &engraving_words,
            None,
        );
        {
            let item = duplicate_alias.object.objs[0].lock().unwrap();
            assert_eq!(item.getString("반응이름"), "명왕검\n회귀명\n회귀명");
        }
        duplicate_alias.set("은전", 1_000_000_i64);
        run_zone_event_source_with_words(
            &mut duplicate_alias,
            "낙양성",
            &erase,
            &erase_words,
            None,
        );
        let item = duplicate_alias.object.objs[0].lock().unwrap();
        assert!(item.getString("확장 이름").is_empty());
        assert_eq!(item.getString("반응이름"), "명왕검\n회귀명");
    }

    #[test]
    fn information_clerk_item_fee_gate_uses_python_inverted_item_check() {
        let room = format!("진영정보회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "정보맨", &room);

        let mut poor = Body::new();
        poor.set("이름", "진영정보부족회귀");
        poor.set("은전", 999_i64);
        add_test_items(&mut poor, "31", 1);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut poor, "낙양성", &room, "진영 명왕검 정보")
                .expect("information event must select the clerk")
        else {
            panic!("insufficient-fee information event must not wait for enter");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("은전 1000개가 필요")));
        assert_eq!(poor.get_int("은전"), 999);

        for (verb, expected) in [("정보", "깨우칠 수 있는"), ("감정", "물건에 난 흠집")]
        {
            let mut rich = Body::new();
            rich.set("이름", format!("진영{verb}회귀"));
            rich.set("은전", 1_000_i64);
            add_test_items(&mut rich, "31", 1);
            let command = format!("진영 명왕검 {verb}");
            let CommandResult::MobEventEnter {
                event_key,
                words,
                line_num,
                resume_func,
                ..
            } = super::try_mob_event(&mut rich, "낙양성", &room, &command)
                .expect("sufficient-fee clerk event must select")
            else {
                panic!("sufficient-fee {verb} must wait for enter");
            };
            let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event_resume(
                &mut rich,
                "낙양성",
                &room,
                &mob_key,
                &event_key,
                words,
                line_num,
                resume_func,
            )
            .expect("clerk event must resume after enter") else {
                panic!("resumed {verb} event must complete");
            };
            assert!(output_lines.iter().any(|line| line.contains(expected)));
            assert_eq!(rich.get_int("은전"), 0);
        }

        // The source has four `$아이템종류확인` gates, all with the same
        // rejection prose.  Each must stop before `$엔터$`; otherwise the
        // player could pay the fee for food/armor/miscellaneous items.
        for kind in ["호위", "먹는것", "방어구", "기타"] {
            for verb in ["정보", "감정"] {
                let mut rejected = Body::new();
                rejected.set("이름", format!("진영{kind}{verb}회귀"));
                rejected.set("은전", 1_000_i64);
                let mut item = Object::new();
                item.set("이름", "검사물");
                item.set("인덱스", "검사물");
                item.set("종류", kind);
                rejected.object.objs.push(Arc::new(Mutex::new(item)));
                let command = format!("진영 검사물 {verb}");
                let CommandResult::MobEvent { output_lines, .. } =
                    super::try_mob_event(&mut rejected, "낙양성", &room, &command)
                        .expect("non-weapon clerk event must select")
                else {
                    panic!("{verb}/{kind} must end before the Enter pause");
                };
                assert!(
                    output_lines
                        .iter()
                        .any(|line| line.contains("확인이 가능한것은 무기뿐이네요")),
                    "{verb}/{kind}: {output_lines:?}"
                );
                assert_eq!(rejected.get_int("은전"), 1_000, "{verb}/{kind}");
            }
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn all_source_item_kind_directives_keep_their_rhai_rejection_predicates() {
        // Keep the legacy directive inventory tied to its JSON/Rhai targets.
        // This catches a partial conversion such as retaining only the first
        // 정보맨 kind check while silently allowing the later kinds through.
        let cases = [
            (
                "data/mob/낙양성/이름맨.mob",
                "data/script/낙양성/이름맨_이름새김.rhai",
                ["호위", "먹는것"].as_slice(),
            ),
            (
                "data/mob/낙양성/정보맨.mob",
                "data/script/낙양성/정보맨_정보_정.rhai",
                ["호위", "먹는것", "방어구", "기타"].as_slice(),
            ),
            (
                "data/mob/낙양성/정보맨.mob",
                "data/script/낙양성/정보맨_감_감정.rhai",
                ["호위", "먹는것", "방어구", "기타"].as_slice(),
            ),
        ];
        for (mob_path, script_path, kinds) in cases {
            let mob_source = std::fs::read_to_string(mob_path).expect("legacy mob source");
            let script_source = std::fs::read_to_string(script_path).expect("Rhai event source");
            for kind in kinds {
                let directive = format!("$아이템종류확인 {kind} $변수:1");
                assert!(
                    mob_source.contains(&directive),
                    "{mob_path}: original directive inventory changed: {directive}"
                );
                let predicate = format!("item_kind_is(words(1), \"{kind}\")");
                assert!(
                    script_source.contains(&predicate),
                    "{script_path}: missing Python item-kind predicate {predicate}"
                );
            }
        }
    }

    #[test]
    fn all_source_inverted_skill_list_checks_keep_their_rhai_branches() {
        // Python `$무공리스트확인!` skips its immediately following block
        // only when every listed ordinary skill is present.  The 비전노인
        // conversion therefore rejects `!has_all_skills`, while each
        // 옥황상제 single-skill block rejects `has_skill` (already learned).
        let trainer_mob = std::fs::read_to_string("data/mob/낙양성/무공맨.mob")
            .expect("vision trainer legacy source");
        let trainer_script = std::fs::read_to_string("data/script/낙양성/무공맨_수련_수.rhai")
            .expect("vision trainer Rhai source");
        let trainer_requirements = [
            "분근착골수 장안기공 사량발천근 금나수",
            "대력금나수",
            "투골타혈법 전암전회",
            "이화접목 차기미기",
            "전이대법 격체전공",
            "건곤대나이 흡성대법",
            "공수탈백인",
            "음양귀혼",
        ];
        for names in trainer_requirements {
            assert!(
                trainer_mob.contains(&format!("$무공리스트확인! {names}")),
                "vision trainer legacy directive missing: {names}"
            );
            assert!(
                trainer_script.contains(&format!("!has_all_skills(\"{names}\")")),
                "vision trainer Rhai predicate missing: {names}"
            );
        }

        let jade_mob = std::fs::read_to_string("data/mob/선인/옥황상제.mob")
            .expect("Jade Emperor legacy source");
        let jade_scripts = [
            ("역근경", "옥황상제_대화_대_역근경.rhai"),
            ("태극강기", "옥황상제_대화_대_태극강기.rhai"),
            ("고영신공", "옥황상제_대화_대_고영신공.rhai"),
            ("가의신공", "옥황상제_대화_대_가의신공.rhai"),
            ("명옥공", "옥황상제_대화_대_명옥공.rhai"),
            ("북명신공", "옥황상제_대화_대_북명신공.rhai"),
            ("천외비선", "옥황상제_대화_대_천외비선.rhai"),
        ];
        for (skill, script) in jade_scripts {
            assert!(
                jade_mob.contains(&format!("$무공리스트확인! {skill}")),
                "Jade Emperor legacy directive missing: {skill}"
            );
            let source = std::fs::read_to_string(format!("data/script/선인/{script}"))
                .expect("Jade Emperor Rhai source");
            assert!(
                source.contains(&format!("has_skill(\"{skill}\")")),
                "{script}: already-learned Python !-branch missing"
            );
        }
    }

    #[test]
    fn information_clerk_restores_python_item_usage_and_learnable_skill_placeholders() {
        // These are ordinary legacy text lines, not `$출력` directives.
        // Python doEvent fills them immediately before sendLine, so Rhai must
        // compose the numbers instead of leaving the placeholders literal.
        let appraisal = std::fs::read_to_string("data/script/낙양성/정보맨_감_감정.rhai")
            .expect("information-clerk appraisal script");
        let information = std::fs::read_to_string("data/script/낙양성/정보맨_정보_정.rhai")
            .expect("information-clerk information script");
        let words = vec![
            "진영".to_string(),
            "독각신창".to_string(),
            "감정".to_string(),
        ];

        let make_body = |name: &str| {
            let mut body = Body::new();
            body.set("이름", name);
            body.set("은전", 1_000_i64);
            body.set("성격", "정파");
            body.object.objs.push(
                super::object_from_item_json("216")
                    .expect("source weapon")
                    .0,
            );
            body.item_skill_map.insert("독각신창".to_string(), 17);
            body.skill_list.push("철포삼".to_string());
            body
        };

        let mut appraisal_body = make_body("정보감정회귀");
        let (output, _) = run_zone_event_source_with_words(
            &mut appraisal_body,
            "낙양성",
            &appraisal,
            &words,
            Some("step1"),
        );
        assert!(output.iter().any(|line| line.contains("17번 정도 사용")));
        assert!(output.iter().all(|line| !line.contains("[아이템사용횟수]")));
        assert_eq!(appraisal_body.get_int("은전"), 0);

        let mut information_body = make_body("정보무공회귀");
        let (output, _) = run_zone_event_source_with_words(
            &mut information_body,
            "낙양성",
            &information,
            &words,
            Some("step1"),
        );
        // 독각신창의 원본 열 무공 중 사파 둘과 이미 배운 철포삼을 제외한다.
        assert!(output.iter().any(|line| line.contains("7개 정도 되는군요")));
        assert!(output
            .iter()
            .all(|line| !line.contains("[배울무공이름갯수]")));
        assert_eq!(information_body.get_int("은전"), 0);

        let riddle =
            std::fs::read_to_string("data/script/산서성/46_답.rhai").expect("riddle answer script");
        let wrong_words = vec!["불혼곡주".to_string(), "241".to_string(), "답".to_string()];
        let mut riddle_body = Body::new();
        super::set_user_event(&mut riddle_body, "불혼곡", "1");
        let (output, _) = run_zone_event_source_with_words(
            &mut riddle_body,
            "산서성",
            &riddle,
            &wrong_words,
            None,
        );
        assert!(output
            .iter()
            .any(|line| line.contains("그것은 241개 아닌가")));
        assert!(output.iter().all(|line| !line.contains("[변수]")));

        clear_test_oneitems(&["해왕조"]);
        assert!(crate::oneitem::oneitem_have("해왕조", "해왕조선점회귀"));
        let correct_words = vec!["불혼곡주".to_string(), "242".to_string(), "답".to_string()];
        let mut claimed_riddle_body = Body::new();
        super::set_user_event(&mut claimed_riddle_body, "불혼곡", "1");
        let (output, _) = run_zone_event_source_with_words(
            &mut claimed_riddle_body,
            "산서성",
            &riddle,
            &correct_words,
            None,
        );
        assert!(output
            .iter()
            .any(|line| line.contains("해왕조선점회귀가 먼저 왔었다네")));
        assert!(output
            .iter()
            .any(|line| line.contains("이름만 똑같은 가짜를 주겠네")));
        assert!(body_has_item_spec(&claimed_riddle_body, "해왕조-5"));
        clear_test_oneitems(&["해왕조"]);
    }

    #[test]
    fn porter_deposit_dynamic_item_check_uses_the_requested_amount_and_negation() {
        let source = std::fs::read_to_string("data/script/낙양성/길쌈이_입금.rhai").unwrap();
        let words = vec!["길쌈이".into(), "10".into(), "입금".into()];

        let mut poor = Body::new();
        poor.set("이름", "길쌈이입금부족회귀");
        poor.set("은전", 9_i64);
        let (output, _) =
            run_zone_event_source_with_words(&mut poor, "낙양성", &source, &words, None);
        assert!(output.iter().any(|line| line.contains("돈이 모자라지")));

        let mut enough = Body::new();
        enough.set("이름", "길쌈이입금충분회귀");
        enough.set("은전", 10_i64);
        let (output, _) =
            run_zone_event_source_with_words(&mut enough, "낙양성", &source, &words, None);
        assert_eq!(output, vec!["입금을 하겠는가?"]);
    }

    #[test]
    fn event_defense_skill_directives_apply_once_and_keep_their_python_branches() {
        // Python `$무공시전` always applies the event-defense skill and
        // `$무공시전2` always applies Santa's blessing.  Both suppress a
        // duplicate effect before asking Mob.makeFightScript() for its
        // player-facing text.  Exercise the authored Rhai events rather than
        // only the shared efun.
        crate::world::skill::reload_skill_cache().expect("event skill definitions");

        let guard_script = std::fs::read_to_string("data/script/낙양성/포졸_축하.rhai")
            .expect("guard celebration script");
        let mut guard = Body::new();
        guard.set("이름", "이벤트무공회귀");
        guard.set("레벨", 299_i64);
        let (first, _) = run_luoyang_event(&mut guard, "포졸_축하.rhai");
        assert!(first.iter().any(|line| line.contains("힘을 불어 넣습니다")));
        assert_eq!(
            guard
                .active_skills
                .iter()
                .filter(|skill| skill.name == "이벤트")
                .count(),
            1
        );
        let (second, _) = run_luoyang_event(&mut guard, "포졸_축하.rhai");
        assert!(!second
            .iter()
            .any(|line| line.contains("힘을 불어 넣습니다")));
        assert_eq!(
            guard
                .active_skills
                .iter()
                .filter(|skill| skill.name == "이벤트")
                .count(),
            1
        );

        // At level 300 the source's `$특성치확인 레벨 300` branch ends
        // before `$무공시전`; no defense effect is added.
        let mut high_level = Body::new();
        high_level.set("이름", "이벤트무공고레벨회귀");
        high_level.set("레벨", 300_i64);
        let (blocked, _) = run_luoyang_event(&mut high_level, "포졸_축하.rhai");
        assert!(blocked.iter().any(|line| line.contains("300레벨 이상")));
        assert!(high_level.active_skills.is_empty());

        let santa_script = std::fs::read_to_string("data/script/낙양성/산타통닭_인사.rhai")
            .expect("Santa greeting script");
        let mut santa = Body::new();
        santa.set("이름", "산타축복회귀");
        super::set_user_event(&mut santa, "2020년크리스마스", "1");
        let (first, _) = run_luoyang_event(&mut santa, "산타통닭_인사.rhai");
        assert!(first.iter().any(|line| line.contains("산타의축복을")));
        assert_eq!(
            santa
                .active_skills
                .iter()
                .filter(|skill| skill.name == "산타의축복")
                .count(),
            1
        );
        let (second, _) = run_luoyang_event(&mut santa, "산타통닭_인사.rhai");
        assert!(!second.iter().any(|line| line.contains("산타의축복을")));
        assert_eq!(
            santa
                .active_skills
                .iter()
                .filter(|skill| skill.name == "산타의축복")
                .count(),
            1
        );

        // Keep the reads above anchored to the authored files: a future
        // script rename must not turn this into an efun-only test.
        assert!(guard_script.contains("apply_defense_skill(\"이벤트\")"));
        assert!(santa_script.contains("apply_defense_skill(\"산타의축복\")"));
    }

    #[test]
    fn fortune_teller_unique_owner_check_charges_only_when_python_condition_matches() {
        // `$기연존재확인 $변수:1` enters its block only for an existing
        // unique item.  The surrounding `$아이템확인 은전 30000000` keeps
        // the poor-player branch ahead of that lookup; an unknown unique name
        // reports absence without taking the fee.
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();
        let script = std::fs::read_to_string("data/script/낙양성/기연맨_위치확인_위치_확인.rhai")
            .expect("fortune teller event script");
        let words = vec![
            "팔괘노야".to_string(),
            "황룡마조".to_string(),
            "위치확인".to_string(),
        ];

        let mut poor = Body::new();
        poor.set("이름", "기연복채부족회귀");
        poor.set("은전", 29_999_999_i64);
        let (output, _) =
            run_zone_event_source_with_words(&mut poor, "낙양성", &script, &words, None);
        assert!(output.iter().any(|line| line.contains("돈 가져와")));
        assert_eq!(poor.get_int("은전"), 29_999_999);

        let mut missing = Body::new();
        missing.set("이름", "기연부재회귀");
        missing.set("은전", 30_000_000_i64);
        let (output, _) =
            run_zone_event_source_with_words(&mut missing, "낙양성", &script, &words, None);
        assert!(output.iter().any(|line| line.contains("강호에 없다네")));
        assert_eq!(missing.get_int("은전"), 30_000_000);

        let owner = "기연소유자회귀";
        let index = crate::oneitem::oneitem_get_index_by_name("황룡마조");
        assert!(!index.is_empty(), "source unique item name must resolve");
        assert!(crate::oneitem::oneitem_have(&index, owner));
        let mut found = Body::new();
        found.set("이름", "기연존재회귀");
        found.set("은전", 30_000_000_i64);
        let (output, _) =
            run_zone_event_source_with_words(&mut found, "낙양성", &script, &words, None);
        assert!(output.iter().any(|line| line.contains(owner)));
        assert_eq!(found.get_int("은전"), 0);

        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).expect("restore unique-item registry");
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(script.contains("one_item_exists_name(words(1))"));
    }

    #[test]
    fn blacksmith_decomposition_keeps_python_dynamic_item_guard_order() {
        // The legacy `$분해` event chains dynamic-name item existence,
        // unequipped state, option presence, removal, and reward creation.
        // Check every guard in the authored NPC event so a helper with the
        // right standalone predicate cannot silently change its order.
        let room = format!("대장장이분해회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "합체맨", &room);

        let mut missing = Body::new();
        missing.set("이름", "분해없음회귀");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut missing, "낙양성", &room, "대장장이 시험검 분해")
                .expect("missing item decomposition event")
        else {
            panic!("missing item decomposition returned a non-event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("그런 아이템이 없")));

        let make_item = |equipped: bool, options: &str| {
            let mut item = Object::new();
            item.set("이름", "시험검");
            item.set("인덱스", "시험검");
            item.set("옵션", options);
            item.set("inUse", i64::from(equipped));
            Arc::new(Mutex::new(item))
        };

        let mut equipped = Body::new();
        equipped.set("이름", "분해착용회귀");
        equipped.object.objs.push(make_item(true, "힘 10"));
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut equipped, "낙양성", &room, "대장장이 시험검 분해")
                .expect("equipped item decomposition event")
        else {
            panic!("equipped item decomposition returned a non-event result");
        };
        // `$아이템확인! $변수:1` uses the default `checkInUse=False`, so a
        // lone worn object is rejected before `$착용확인!` is reached.
        assert!(output_lines
            .iter()
            .any(|line| line.contains("그런 아이템이 없")));
        assert_eq!(equipped.object.objs.len(), 1);

        // An unworn duplicate makes the first guard pass, then `getItemName`
        // inspects the first matching object; a worn first entry therefore
        // takes the `$착용확인!` rejection.
        let mut mixed = Body::new();
        mixed.set("이름", "분해혼합회귀");
        mixed.object.objs.push(make_item(true, "힘 10"));
        mixed.object.objs.push(make_item(false, "힘 10"));
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut mixed, "낙양성", &room, "대장장이 시험검 분해")
                .expect("mixed item decomposition event")
        else {
            panic!("mixed item decomposition returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("착용 중인건")));
        assert_eq!(mixed.object.objs.len(), 2);

        let mut plain = Body::new();
        plain.set("이름", "분해무옵션회귀");
        plain.object.objs.push(make_item(false, ""));
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut plain, "낙양성", &room, "대장장이 시험검 분해")
                .expect("plain item decomposition event")
        else {
            panic!("plain item decomposition returned a non-event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("속성 아이템이 아니")));
        assert_eq!(plain.object.objs.len(), 1);

        let mut optioned = Body::new();
        optioned.set("이름", "분해성공회귀");
        optioned.object.objs.push(make_item(false, "힘 10"));
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut optioned, "낙양성", &room, "대장장이 시험검 분해")
                .expect("optioned item decomposition event")
        else {
            panic!("optioned item decomposition returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("강철조각")));
        assert!(!optioned
            .object
            .objs
            .iter()
            .any(|item| item.lock().is_ok_and(|item| item.getName() == "시험검")));
        assert!(body_has_item_spec(&optioned, "강철조각"));

        // There are five current legacy `$아이템확인! $변수:1` call sites.
        // All route through Python `checkItemName(..., False)` by default,
        // which excludes worn objects.
        let dynamic_checks = [
            "data/script/낙양성/합체맨_분해.rhai",
            "data/script/낙양성/이름맨_이름지움_이름삭제.rhai",
            "data/script/낙양성/이름맨_이름새김.rhai",
            "data/script/낙양성/정보맨_감_감정.rhai",
            "data/script/낙양성/정보맨_정보_정.rhai",
        ];
        assert_eq!(dynamic_checks.len(), 5);
        for path in dynamic_checks {
            let source = std::fs::read_to_string(path).unwrap_or_else(|error| {
                panic!("missing dynamic item-check script {path}: {error}")
            });
            assert!(
                source.contains("item_exists_unworn_named(words(1), 1)"),
                "{path} must preserve Python checkItemName(..., False)"
            );
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn dynamic_unworn_name_check_keeps_python_quantity_and_currency_rules() {
        // Python `$아이템확인! $변수:1` invokes
        // `checkItemName(name, count, False)`: worn objects are excluded,
        // but the authored count still matters and money follows the normal
        // balance check.  Current source calls request one item, yet the
        // helper must retain this contract for hot-reloaded event data too.
        let source = r#"
fn event() {
    if item_exists_unworn_named(words(1), 2) { output("two-items"); }
    if item_exists_unworn_named("은전", 20) { output("enough-silver"); }
    if item_exists_unworn_named("금전", 1) { output("enough-gold"); }
    end_event();
}
"#;
        let words = vec![
            "시험맨".to_string(),
            "시험검".to_string(),
            "확인".to_string(),
        ];

        let make_item = |worn: bool| {
            let mut item = Object::new();
            item.set("이름", "시험검");
            item.set("인덱스", "시험검");
            item.set("inUse", i64::from(worn));
            Arc::new(Mutex::new(item))
        };
        let mut one_unworn_one_worn = Body::new();
        one_unworn_one_worn.set("은전", 19_i64);
        one_unworn_one_worn.set("금전", 1_i64);
        one_unworn_one_worn.object.objs.push(make_item(false));
        one_unworn_one_worn.object.objs.push(make_item(true));
        let (output, _) = run_zone_event_source_with_words(
            &mut one_unworn_one_worn,
            "낙양성",
            source,
            &words,
            None,
        );
        assert_eq!(output, vec!["enough-gold"]);

        let mut enough = Body::new();
        enough.set("은전", 20_i64);
        enough.object.objs.push(make_item(false));
        enough.object.objs.push(make_item(false));
        let (output, _) =
            run_zone_event_source_with_words(&mut enough, "낙양성", source, &words, None);
        assert_eq!(output, vec!["two-items", "enough-silver"]);
    }

    #[test]
    fn dynamic_event_item_delete_keeps_python_unique_owner_record() {
        // Python `$아이템삭제 $변수:1` calls Object.remove(item).  That
        // consumes the particular inventory object but deliberately does not
        // turn a globally claimed ONEITEM back into an unclaimed reward.
        let index = crate::oneitem::oneitem_get_index_by_name("황룡마조");
        assert!(!index.is_empty(), "fixture unique item must have an index");
        clear_test_oneitems(&[&index]);
        let owner = "동적삭제기연소유회귀";
        assert!(crate::oneitem::oneitem_have(&index, owner));

        let mut body = Body::new();
        body.set("이름", owner);
        let mut item = Object::new();
        item.set("이름", "황룡마조");
        item.set("인덱스", index.clone());
        item.setAttr("아이템속성", "단일아이템");
        body.object.objs.push(Arc::new(Mutex::new(item)));

        let (output, _) = run_zone_event_source(
            &mut body,
            "낙양성",
            r#"
fn event() {
    delete_item_named("황룡마조");
    output("삭제완료");
    end_event();
}
"#,
            None,
        );
        assert_eq!(output, vec!["삭제완료"]);
        assert!(body.object.objs.is_empty());
        assert_eq!(
            crate::oneitem::oneitem_get(&index),
            owner,
            "event deletion must preserve Python's permanently claimed unique reward"
        );
        clear_test_oneitems(&[&index]);
    }

    #[test]
    fn wang_daehyup_tendency_switch_requires_the_source_head_and_toggles_once() {
        // Python `$정사전환` changes only 정파/사파 after the authored
        // `장문머리` exchange.  Keep the missing-item dialogue separate so
        // the state mutation cannot happen merely by selecting the event.
        let room = format!("정사전환회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "83", &room);

        let mut missing = Body::new();
        missing.set("이름", "정사전환부족회귀");
        missing.set("성격", "사파");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut missing, "낙양성", &room, "왕대협 정사전환 대화")
                .expect("missing-head tendency event")
        else {
            panic!("missing-head tendency event returned a non-event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("각오를 보이게")));
        assert_eq!(missing.get_string("성격"), "사파");
        assert!(get_user_event(&missing, "정사전환이벤트").is_empty());

        let mut switched = Body::new();
        switched.set("이름", "정사전환성공회귀");
        switched.set("성격", "사파");
        add_test_items(&mut switched, "장문머리", 1);
        super::try_mob_event(&mut switched, "낙양성", &room, "왕대협 정사전환 대화")
            .expect("head exchange tendency event");
        assert_eq!(switched.get_string("성격"), "정파");
        assert!(!get_user_event(&switched, "정사전환이벤트").is_empty());
        assert!(!body_has_item_spec(&switched, "장문머리"));
        assert!(switched.object.objs.is_empty());
        assert_eq!(switched.object.inv_stack.get("합성11"), Some(&2));

        // The source's first event check makes a repeat a message-only
        // branch; it must not toggle the restored 정파 state back to 사파.
        super::try_mob_event(&mut switched, "낙양성", &room, "왕대협 정사전환 대화")
            .expect("completed tendency event");
        assert_eq!(switched.get_string("성격"), "정파");

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn wang_daehyup_nickname_tendency_branches_match_all_three_source_conditions() {
        // `$무림별호조건` has three source forms in this event: completed
        // nickname, righteous qualification, and evil qualification.  Verify
        // the authored Rhai selects the same terminal branch and preserves
        // the two-token event name that Python stores verbatim.
        let script = "83_대화_대.rhai";
        let threshold = crate::script::get_murim_config_int("무림별호이벤트킬수");

        let mut complete = Body::new();
        complete.set("이름", "별호완료회귀");
        complete.set("무림별호", "청룡검객");
        let (output, _) = run_luoyang_event(&mut complete, script);
        assert!(output.iter().any(|line| line.contains("명성이 강호에")));
        assert!(get_user_event(&complete, "무림별호설정").is_empty());

        let mut righteous = Body::new();
        righteous.set("이름", "별호정파회귀");
        righteous.set("0 성격플킬", threshold);
        righteous.set("1 성격플킬", 1_i64);
        righteous.set("2 성격플킬", 0_i64);
        let (output, _) = run_luoyang_event(&mut righteous, script);
        assert!(output.iter().any(|line| line.contains("정의롭고")));
        assert!(!get_user_event(&righteous, "무림별호 정파").is_empty());
        assert!(!get_user_event(&righteous, "무림별호설정").is_empty());

        let mut evil = Body::new();
        evil.set("이름", "별호사파회귀");
        evil.set("0 성격플킬", threshold);
        evil.set("1 성격플킬", 0_i64);
        evil.set("2 성격플킬", 1_i64);
        let (output, _) = run_luoyang_event(&mut evil, script);
        assert!(output.iter().any(|line| line.contains("악랄하고")));
        assert!(!get_user_event(&evil, "무림별호 사파").is_empty());
        assert!(!get_user_event(&evil, "무림별호설정").is_empty());
    }

    #[test]
    fn nickname_change_event_releases_old_name_reserves_new_name_and_requests_return() {
        static NICKNAME_SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let sequence = NICKNAME_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let suffix = format!("{}-{sequence}", std::process::id());
        let owner = format!("별호변경회귀-{suffix}");
        // Python allows at most ten characters.  Keep the registry fixture
        // unique within the test process without accidentally taking the
        // source's overlength rejection branch.
        let old_nickname = format!("이전{:07x}", sequence);
        let new_nickname = format!("신규{:07x}", sequence);
        let room = format!("별호변경방-{suffix}");
        let (mob_key, _) = place_event_mob("호남성", "39", &room);

        // A normal Python player has its current title registered already.
        // `$별호변경` deletes this exact entry before registering the new one.
        assert_eq!(
            crate::world::nickname::nickname_reserve(&old_nickname, &owner),
            ""
        );
        let mut body = Body::new();
        body.set("이름", owner.as_str());
        body.set("무림별호", old_nickname.as_str());
        let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event(
            &mut body,
            "호남성",
            &room,
            &format!("천의검마 {new_nickname} 별호변경"),
        )
        .expect("nickname-change event") else {
            panic!("nickname-change must remain an event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("武林")));
        assert_eq!(body.get_string("무림별호"), new_nickname);
        assert!(!crate::world::nickname::nickname_exists(&old_nickname));
        assert_eq!(crate::world::nickname::nickname_owner(&new_nickname), owner);
        assert_eq!(
            crate::script::take_event_command_request(&mut body),
            Some("귀환".to_string())
        );

        assert!(crate::world::nickname::nickname_release(
            &new_nickname,
            &owner
        ));
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn kukhwa_manual_keeps_python_gender_check_and_both_setting_branches() {
        // Python gender check skips only the immediately following block
        // when the player is already male. The next bare block then changes
        // that player to female; a non-male player instead takes the first
        // male-setting block. Do not simplify this source quirk into a
        // female-to-male-only conversion.
        let room = format!("규화성별회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("무림맹", "대학사", &room);

        let mut female = Body::new();
        female.set("이름", "규화여성회귀");
        female.set("성별", "여");
        add_test_items(&mut female, "규화보전", 1);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut female, "무림맹", &room, "대학사 규화보전")
                .expect("female kukhwa event")
        else {
            panic!("female kukhwa returned a non-event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("남자가 되었습니다")));
        assert_eq!(female.get_string("성별"), "남");
        assert!(female.skill_list.iter().any(|skill| skill == "규화보전"));
        assert!(!body_has_item_spec(&female, "규화보전"));

        let mut male = Body::new();
        male.set("이름", "규화남성회귀");
        male.set("성별", "남");
        add_test_items(&mut male, "규화보전", 1);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut male, "무림맹", &room, "대학사 규화보전")
                .expect("male kukhwa event")
        else {
            panic!("male kukhwa returned a non-event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("여자가 되었습니다")));
        assert_eq!(male.get_string("성별"), "여");
        assert!(male.skill_list.iter().any(|skill| skill == "규화보전"));
        assert!(!body_has_item_spec(&male, "규화보전"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn legacy_script_output_keeps_python_self_and_same_room_renderings() {
        // Python Player.printScript() emits the same legacy $출력 line twice:
        // [공] becomes 당신 for the actor, while room observers receive the
        // bold `getNameA()` actor name with the authored particle.  Plain
        // Rhai output() is actor-only, so converted $출력 lines use
        // room_broadcast_output().
        let authored = std::fs::read_to_string("data/script/참회동/산딸기_벗겨_가발.rhai")
            .expect("worn-wig event source");
        assert!(
            authored.contains("output(post_position_once(\"당신(이/가) 가발을 훌러덩 벗깁니다\"))")
        );
        assert!(authored.contains("room_broadcast_output(\"[공](이/가) 가발을 훌러덩 벗깁니다\")"));
        let source = r#"
fn event() {
    output(post_position_once("당신(이/가) 가발을 훌러덩 벗깁니다"));
    room_broadcast_output("[공](이/가) 가발을 훌러덩 벗깁니다");
    end_event();
}
"#;
        let mut body = Body::new();
        body.set("이름", "출력회귀");
        let mut data = RawMobData::new();
        data.zone = "참회동".to_string();
        let CommandResult::MobEvent {
            output_lines,
            room_broadcast_lines,
            ..
        } = super::do_event_rhai_source(
            &mut body,
            &data,
            "산딸기",
            &[],
            "벗겨 가발",
            &source,
            None,
        )
        else {
            panic!("legacy script output source did not return a mob event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line == "당신이 가발을 훌러덩 벗깁니다"));
        assert!(room_broadcast_lines
            .iter()
            .any(|line| line == "\x1b[1m출력회귀\x1b[0;37m가 가발을 훌러덩 벗깁니다"));
    }

    #[test]
    fn resumed_event_keeps_legacy_script_output_for_self_and_room() {
        // `$엔터$` resumes in a fresh event evaluation.  Preserve the two
        // Player.printScript() renderings there as well; otherwise only the
        // actor would see legacy `$출력` after pressing Enter.
        let source = r#"
fn event() {
    wait_enter("step", "[엔터키를 누르세요]");
}

fn step() {
    output(post_position_once("당신(이/가) 재개된 가발을 벗깁니다"));
    room_broadcast_output("[공](이/가) 재개된 가발을 벗깁니다");
    end_event();
}
"#;
        let mut body = Body::new();
        body.set("이름", "재개출력회귀");
        let mut data = RawMobData::new();
        data.zone = "참회동".to_string();
        let CommandResult::MobEventEnter {
            resume_func: Some(resume_func),
            ..
        } = super::do_event_rhai_source(&mut body, &data, "산딸기", &[], "벗겨 가발", source, None)
        else {
            panic!("initial wait_enter source did not return an enter event");
        };
        let CommandResult::MobEvent {
            output_lines,
            room_broadcast_lines,
            ..
        } = super::do_event_rhai_source(
            &mut body,
            &data,
            "산딸기",
            &[],
            "벗겨 가발",
            source,
            Some(resume_func),
        )
        else {
            panic!("resumed wait_enter source did not return a mob event");
        };
        assert_eq!(output_lines, vec!["당신이 재개된 가발을 벗깁니다"]);
        assert_eq!(
            room_broadcast_lines,
            vec!["\x1b[1m재개출력회귀\x1b[0;37m가 재개된 가발을 벗깁니다"]
        );
    }

    #[test]
    fn frog_child_enter_sequence_finishes_after_the_legacy_interactive_end_marker() {
        // `안휘성/40.mob` brackets this dialogue with
        // `$입력대기출력시작$` and `$입력대기출력끝$`, with three `$엔터$`
        // pauses in between.  Python returns to normal command parsing only
        // after the last resumed call has executed `$아이템주기 철사` and the
        // closing interactive marker.  Exercise the authored Rhai sequence
        // rather than only counting `wait_enter()` occurrences.
        let mut body = Body::new();
        body.set("이름", "입력대기끝회귀");
        super::set_user_event(&mut body, "황소개구리끝", "1");
        let mut data = RawMobData::new();
        data.zone = "안휘성".to_string();

        let first = do_event_rhai(
            &mut body,
            &data,
            "대화",
            &[],
            "동네꼬마",
            "40_대화_대.rhai",
            None,
        );
        let CommandResult::MobEventEnter {
            resume_func: Some(step1),
            prompt,
            ..
        } = first
        else {
            panic!("frog child dialogue must pause at its first Python $엔터$");
        };
        assert_eq!(prompt, "[엔터키를 누르세요]");

        let second = do_event_rhai(
            &mut body,
            &data,
            "대화",
            &[],
            "동네꼬마",
            "40_대화_대.rhai",
            Some(step1),
        );
        let CommandResult::MobEventEnter {
            resume_func: Some(step2),
            ..
        } = second
        else {
            panic!("frog child dialogue must preserve its second Python $엔터$");
        };

        let third = do_event_rhai(
            &mut body,
            &data,
            "대화",
            &[],
            "동네꼬마",
            "40_대화_대.rhai",
            Some(step2),
        );
        let CommandResult::MobEventEnter {
            resume_func: Some(step3),
            ..
        } = third
        else {
            panic!("frog child dialogue must preserve its third Python $엔터$");
        };

        let completed = do_event_rhai(
            &mut body,
            &data,
            "대화",
            &[],
            "동네꼬마",
            "40_대화_대.rhai",
            Some(step3),
        );
        assert!(
            matches!(completed, CommandResult::MobEvent { .. }),
            "the final $입력대기출력끝$ continuation must not leave another Enter callback"
        );
        assert!(body_has_item_spec(&body, "철사"));
    }

    #[test]
    fn every_event_move_finishes_before_a_later_same_room_script_output() {
        // Python doEvent() applies `$위치이동` immediately, whereas the
        // Rust command result applies the final position after the Rhai
        // function returns.  A same-room `$출력` after a reachable move would
        // therefore need a position-tagged delivery rather than the current
        // pre-move sendRoom() route.  The converted legacy scripts all end
        // their reached move branch first; keep that source invariant exact.
        fn collect_rhai_files(directory: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("script directory") {
                let entry = entry.expect("script tree entry");
                let path = entry.path();
                if path.is_dir() {
                    collect_rhai_files(&path, files);
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("rhai") {
                    files.push(path);
                }
            }
        }

        let mut files = Vec::new();
        collect_rhai_files(std::path::Path::new("data/script"), &mut files);
        let mut move_count = 0usize;
        for path in files {
            let source = std::fs::read_to_string(&path).expect("event script source");
            let mut remaining = source.as_str();
            while let Some(offset) = remaining.find("set_position(") {
                move_count += 1;
                let after_move = &remaining[offset..];
                let end = after_move
                    .find(";")
                    .expect("set_position call must end with a semicolon");
                let after_call = &after_move[end + 1..];
                let next_boundary = [
                    (after_call.find("end_event()"), "end_event"),
                    (
                        after_call.find("room_broadcast_output("),
                        "room_broadcast_output",
                    ),
                    (after_call.find("\nfn "), "next function"),
                ]
                .into_iter()
                .filter_map(|(offset, kind)| offset.map(|offset| (offset, kind)))
                .min_by_key(|(offset, _)| *offset)
                .expect("move must finish its event branch or reach a boundary");
                assert_eq!(
                    next_boundary.1,
                    "end_event",
                    "{}: set_position may not reach a later same-room output without ending first",
                    path.display()
                );
                remaining = after_call;
            }
        }
        assert_eq!(move_count, 445, "converted event move inventory changed");
    }

    #[test]
    fn martial_arts_spectator_directives_keep_python_noop_state() {
        // These two legacy directives look stateful, but objs/event.py
        // deliberately handles both with `pass`.  The authored dialogue and
        // `$출력` stay observable; no observer/position/body state may be
        // introduced by the Rhai conversion.
        let python = std::fs::read_to_string("objs/event.py").expect("Python event source");
        assert!(python.contains("elif func == '$비무관람시작':\n                pass"));
        assert!(python.contains("elif func == '$비무관람끝':\n                pass"));

        for (script, expected) in [
            ("비무대1_관람_관_비무장_비무.rhai", "잠시동안 시력을"),
            ("비무대1_관람끝_비무장_비무.rhai", "시력을 원래대로"),
            ("비무대2_관람_관_비무장_비무.rhai", "잠시동안 시력을"),
            ("비무대2_관람끝_비무장_비무.rhai", "시력을 원래대로"),
        ] {
            let mut body = Body::new();
            body.set("이름", "비무관람회귀");
            body.set("체력", 777_i64);
            let before = body.clone();
            let (output, destination) = run_luoyang_event(&mut body, script);
            assert!(
                output.iter().any(|line| line.contains(expected)),
                "{script}"
            );
            assert_eq!(destination, None, "{script}");
            assert_eq!(body.get_int("체력"), before.get_int("체력"), "{script}");
            assert_eq!(body.object.objs.len(), before.object.objs.len(), "{script}");
        }
    }

    #[test]
    fn level_upper_checks_keep_python_tower_and_arena_boundary_selection() {
        // `$레벨상위확인 N` admits level N and above; the inverted form
        // `$레벨상위확인! N` admits only levels below N.  The arena chains ten
        // inverted guards, making it a compact check of every boundary.
        let mut tower_low = Body::new();
        tower_low.set("이름", "구층탑저레벨회귀");
        tower_low.set("레벨", 199_i64);
        let (output, destination) = run_luoyang_event(&mut tower_low, "구층탑_입장_입.rhai");
        assert!(output.iter().any(|line| line.contains("무형의 기운")));
        assert_eq!(destination, None);

        let mut tower_edge = Body::new();
        tower_edge.set("이름", "구층탑경계회귀");
        tower_edge.set("레벨", 200_i64);
        let (_, destination) = run_luoyang_event(&mut tower_edge, "구층탑_입장_입.rhai");
        assert_eq!(destination, Some(("구층탑".into(), "1".into())));

        for (level, room) in [
            (99_i64, "9001"),
            (100, "9002"),
            (199, "9002"),
            (200, "9003"),
            (899, "9009"),
            (900, "9010"),
            (999, "9010"),
            (1_000, "9000"),
        ] {
            let mut body = Body::new();
            body.set("이름", format!("비무장경계{level}회귀"));
            body.set("레벨", level);
            let (_, destination) = run_luoyang_event(&mut body, "비무장_입장_입.rhai");
            assert_eq!(
                destination,
                Some(("낙양성".to_string(), room.to_string())),
                "level {level}"
            );
        }
    }

    #[test]
    fn forced_combat_directive_keeps_python_corpse_and_self_fight_guards_only() {
        const SOURCE: &str = r#"
fn event() { output(force_start_selected_mob_combat()); end_event(); }
"#;
        let mut body = Body::new();
        body.set("이름", "강제전투회귀");

        let (output, _) = run_zone_event_source(&mut body, "낙양성", SOURCE, None);
        assert_eq!(output, vec!["started"]);
        assert!(body.temp().contains_key("_event_selected_mob_start_combat"));

        body.temp_mut().clear();
        body.act = crate::player::ActState::Fight;
        let (output, _) = run_zone_event_source(&mut body, "낙양성", SOURCE, None);
        assert_eq!(output, vec!["self_busy"]);

        body.temp_mut().insert(
            "_event_selected_mob_targeted_by_player".into(),
            Value::Int(1),
        );
        let (output, _) = run_zone_event_source(&mut body, "낙양성", SOURCE, None);
        assert_eq!(output, vec!["already_attacking"]);

        body.act = crate::player::ActState::Stand;
        body.temp_mut().clear();
        body.temp_mut()
            .insert("_event_selected_mob_corpse".into(), Value::Int(1));
        let (output, _) = run_zone_event_source(&mut body, "낙양성", SOURCE, None);
        assert_eq!(output, vec!["dead"]);
    }

    #[test]
    fn inverted_rank_check_continues_only_when_python_rank_is_zero() {
        const RANK_TYPE: &str = "미사용순위확인회귀";
        const SOURCE: &str = r#"
fn event() {
    let result = rank_query(10, "미사용순위확인회귀");
    // Python `$순위확인!` sets searchEnd when rank1 != 0.
    if result["found"] { output("skip"); end_event(); }
    output("continue");
    end_event();
}
"#;
        crate::world::rank::rank_clear(RANK_TYPE);
        assert_eq!(
            crate::world::rank::rank_write(RANK_TYPE, "순위보유자", 100, 10),
            1
        );

        let mut ranked = Body::new();
        ranked.set("이름", "순위보유자");
        let (output, _) = run_zone_event_source_with_words(
            &mut ranked,
            "낙양성",
            SOURCE,
            &["비석".into(), "순위보유자".into(), "봐".into()],
            None,
        );
        assert_eq!(output, vec!["skip"]);

        let mut unranked = Body::new();
        unranked.set("이름", "순위미보유자");
        let (output, _) = run_zone_event_source_with_words(
            &mut unranked,
            "낙양성",
            SOURCE,
            &["비석".into(), "순위미보유자".into(), "봐".into()],
            None,
        );
        assert_eq!(output, vec!["continue"]);
        crate::world::rank::rank_clear(RANK_TYPE);
    }

    #[test]
    fn legacy_variable_check_uses_python_argument_index_including_the_mob_target() {
        let mut data = RawMobData::new();
        data.events.insert(
            "이벤트 $답".to_string(),
            EventScript::Legacy(vec![
                "$변수확인 1 242".to_string(),
                "{".to_string(),
                "정답".to_string(),
                "$종료".to_string(),
                "}".to_string(),
                "오답".to_string(),
                "$종료".to_string(),
            ]),
        );
        let mut body = Body::new();
        body.set("이름", "변수확인회귀");

        let correct = do_event(
            &mut body,
            &data,
            "이벤트 $답",
            &["불혼곡주".into(), "242".into(), "답".into()],
            "46",
            None,
            None,
        );
        let CommandResult::MobEvent { output_lines, .. } = correct else {
            panic!("legacy variable check did not produce a mob event");
        };
        assert_eq!(output_lines, vec!["정답"]);

        let incorrect = do_event(
            &mut body,
            &data,
            "이벤트 $답",
            &["불혼곡주".into(), "241".into(), "답".into()],
            "46",
            None,
            None,
        );
        let CommandResult::MobEvent { output_lines, .. } = incorrect else {
            panic!("legacy variable check did not produce a mob event");
        };
        assert_eq!(output_lines, vec!["오답"]);
    }

    #[test]
    fn legacy_output_keeps_python_print_script_self_and_room_renderings() {
        // The current data set has no legacy arrays, but do_event() remains a
        // compatibility path for reloaded data.  Python `$출력` always goes
        // through Player.printScript(), not ordinary one-recipient output.
        let mut body = Body::new();
        body.set("이름", "고전출력회귀");
        let mut data = RawMobData::new();
        data.events.insert(
            "이벤트 $인사".to_string(),
            EventScript::Legacy(vec![
                "$출력 [공](이/가) 고개를 숙입니다".to_string(),
                "$종료".to_string(),
            ]),
        );
        let CommandResult::MobEvent {
            output_lines,
            room_broadcast_lines,
            ..
        } = super::do_event(
            &mut body,
            &data,
            "이벤트 $인사",
            &["대상".to_string(), "인사".to_string()],
            "고전출력몹",
            None,
            None,
        )
        else {
            panic!("legacy output must return a mob event");
        };
        assert_eq!(output_lines, vec!["당신이 고개를 숙입니다"]);
        assert_eq!(
            room_broadcast_lines,
            vec!["\x1b[1m고전출력회귀\x1b[0;37m가 고개를 숙입니다"]
        );

        data.events.insert(
            "이벤트 $대기".to_string(),
            EventScript::Legacy(vec![
                "$출력 [공](이/가) 다시 고개를 숙입니다".to_string(),
                "$엔터$ [엔터키를 누르세요]".to_string(),
            ]),
        );
        let CommandResult::MobEventEnter {
            output_lines,
            room_broadcast_lines,
            prompt,
            ..
        } = super::do_event(
            &mut body,
            &data,
            "이벤트 $대기",
            &["대상".to_string(), "대기".to_string()],
            "고전출력몹",
            None,
            None,
        )
        else {
            panic!("legacy output before enter must preserve both renderings");
        };
        assert_eq!(prompt, "[엔터키를 누르세요]");
        assert_eq!(output_lines, vec!["당신이 다시 고개를 숙입니다"]);
        assert_eq!(
            room_broadcast_lines,
            vec!["\x1b[1m고전출력회귀\x1b[0;37m가 다시 고개를 숙입니다"]
        );
    }

    #[test]
    fn legacy_event_set_keeps_the_complete_whitespace_flag_like_python_set_event() {
        let mut data = RawMobData::new();
        data.events.insert(
            "이벤트 $대화".to_string(),
            EventScript::Legacy(vec![
                "$이벤트설정 오소리가죽 이벤트".to_string(),
                "$이벤트설정 무당산우물 끝".to_string(),
                "$종료".to_string(),
            ]),
        );
        let mut body = Body::new();
        body.set("이름", "legacy-event-whitespace-regression");

        let _ = do_event(
            &mut body,
            &data,
            "이벤트 $대화",
            &["시험몹".into(), "대화".into()],
            "test",
            None,
            None,
        );

        assert!(!get_user_event(&body, "오소리가죽 이벤트").is_empty());
        assert!(!get_user_event(&body, "무당산우물 끝").is_empty());
        assert!(get_user_event(&body, "오소리가죽").is_empty());
        assert!(get_user_event(&body, "무당산우물").is_empty());
    }

    #[test]
    fn legacy_item_directives_share_python_add_and_delete_item_rules_with_rhai() {
        let mut data = RawMobData::new();
        data.events.insert(
            "이벤트 $보상".to_string(),
            EventScript::Legacy(vec![
                "$아이템주기 31".to_string(),
                "$아이템삭제 은전 5".to_string(),
                "$종료".to_string(),
            ]),
        );
        let mut body = Body::new();
        body.set("이름", "legacy-item-regression");
        body.set("레벨", 1);
        body.set("은전", 3);

        let _ = do_event(
            &mut body,
            &data,
            "이벤트 $보상",
            &["시험몹".into(), "보상".into()],
            "test",
            None,
            None,
        );
        assert_eq!(body.get_int("은전"), -2);
        assert_eq!(body.object.objs.len(), 1);
        // `$아이템주기` uses the production random roll.  The deterministic
        // `event_item_grant_preserves_python_single_count_magic_and_money_subtraction`
        // test above verifies the one-count magic rule itself; this test
        // verifies that the legacy directive reaches the shared helper.
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

    fn run_zone_event_source(
        body: &mut Body,
        zone: &str,
        source: &str,
        resume_func: Option<&str>,
    ) -> (Vec<String>, Option<(String, String)>) {
        run_zone_event_source_with_words(body, zone, source, &[], resume_func)
    }

    fn run_zone_event_source_with_words(
        body: &mut Body,
        zone: &str,
        source: &str,
        words: &[String],
        resume_func: Option<&str>,
    ) -> (Vec<String>, Option<(String, String)>) {
        let mut data = RawMobData::new();
        data.zone = zone.to_string();
        match super::do_event_rhai_source(
            body,
            &data,
            "test",
            words,
            "test",
            source,
            resume_func.map(str::to_string),
        ) {
            CommandResult::MobEvent {
                output_lines,
                set_position,
                ..
            } => (output_lines, set_position),
            other => panic!("{source} failed: {other:?}"),
        }
    }

    fn event_script_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
        let entries = std::fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        for entry in entries {
            let entry = entry.expect("event script directory entry");
            let path = entry.path();
            if path.is_dir() {
                event_script_paths(&path, paths);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "rhai")
            {
                paths.push(path);
            }
        }
    }

    #[test]
    fn every_referenced_event_rhai_file_has_valid_rhai_syntax() {
        let mut paths = Vec::new();
        for entry in std::fs::read_dir("data/script").expect("event script root") {
            let path = entry.expect("event script root entry").path();
            if path.is_dir() {
                event_script_paths(&path, &mut paths);
            }
        }
        assert_eq!(paths.len(), 1_095, "all JSON event scripts must be audited");

        const PREAMBLE: &str = r#"fn end_event() { throw #{ type: "event_complete" }; }"#;
        for path in paths {
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            rhai::Engine::new()
                .compile(format!("{PREAMBLE}\n\n{source}"))
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        }
    }

    #[test]
    fn every_legacy_mob_event_key_maps_to_an_existing_rhai_event_script() {
        fn normalize_event_key(key: &str) -> String {
            key.trim()
                .trim_start_matches("이벤트")
                .trim_start()
                .trim_start_matches(':')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }

        let mut source_event_count = 0;
        let mut mob_paths = Vec::new();
        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }
        collect_mob_paths(std::path::Path::new("data/mob"), &mut mob_paths);

        for mob_path in mob_paths {
            let json_path = mob_path.with_extension("json");
            if !json_path.exists() {
                continue;
            }
            let source = std::fs::read_to_string(&mob_path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", mob_path.display()));
            let event_keys = source
                .lines()
                .filter_map(|line| line.strip_prefix("#이벤트"))
                .map(normalize_event_key)
                .filter(|key| !key.is_empty())
                .collect::<Vec<_>>();
            if event_keys.is_empty() {
                continue;
            }
            let root: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
            )
            .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
            let info = root
                .get("몹정보")
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
            let zone = mob_path
                .parent()
                .expect("mob zone directory")
                .file_name()
                .unwrap();
            for legacy_key in event_keys {
                source_event_count += 1;
                // Source headers use both `#이벤트:` and `#이벤트 `, while
                // converted JSON preserved their inconsistent whitespace.
                // Compare command tokens, not spelling noise.
                let (key, script) = info
                    .iter()
                    .find_map(|(key, value)| {
                        (normalize_event_key(key) == legacy_key)
                            .then(|| value.as_str().map(|script| (key, script)))
                            .flatten()
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "{} event key {legacy_key:?} is not mapped to Rhai",
                            mob_path.display()
                        )
                    });
                assert!(
                    script.ends_with(".rhai"),
                    "{} event {key:?}: {script:?}",
                    mob_path.display()
                );
                assert!(
                    std::path::Path::new("data/script")
                        .join(zone)
                        .join(script)
                        .is_file(),
                    "{} event {key:?}: missing script {script}",
                    mob_path.display()
                );
            }
        }
        assert_eq!(
            source_event_count, 1_097,
            "legacy source event inventory changed"
        );
    }

    #[test]
    fn every_legacy_enter_pause_and_interactive_boundary_is_preserved_as_a_rhai_wait_enter() {
        fn normalize_event_key(key: &str) -> String {
            key.trim()
                .trim_start_matches("이벤트")
                .trim_start()
                .trim_start_matches(':')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }

        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }

        let mut mob_paths = Vec::new();
        collect_mob_paths(std::path::Path::new("data/mob"), &mut mob_paths);
        let mut pauses = 0_usize;
        let mut interactive_boundaries = 0_usize;

        for mob_path in mob_paths {
            let json_path = mob_path.with_extension("json");
            if !json_path.exists() {
                continue;
            }
            let source = std::fs::read_to_string(&mob_path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", mob_path.display()));
            let root: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
            )
            .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
            let info = root
                .get("몹정보")
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
            let zone = mob_path
                .parent()
                .expect("mob zone directory")
                .file_name()
                .expect("mob zone name");

            let mut current_key: Option<&str> = None;
            let mut has_enter = false;
            let mut has_interactive_start = false;
            let mut has_interactive_end = false;
            let mut check_block = |key: &str,
                                   has_enter: bool,
                                   has_interactive_start: bool,
                                   has_interactive_end: bool| {
                if !has_enter && !has_interactive_start && !has_interactive_end {
                    return;
                }
                if has_enter {
                    pauses += 1;
                }
                if has_interactive_start || has_interactive_end {
                    assert_eq!(
                        has_interactive_start,
                        has_interactive_end,
                        "{} event {key:?} has an unpaired legacy interactive boundary",
                        mob_path.display()
                    );
                    interactive_boundaries += 1;
                }
                let normalized = normalize_event_key(key);
                let script = info
                    .iter()
                    .find_map(|(event_key, value)| {
                        (normalize_event_key(event_key) == normalized)
                            .then(|| value.as_str())
                            .flatten()
                    })
                    .unwrap_or_else(|| {
                        panic!("{} event {key:?} is not mapped", mob_path.display())
                    });
                let path = std::path::Path::new("data/script").join(zone).join(script);
                let rhai = std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
                assert!(
                    rhai.contains("wait_enter("),
                    "{} event {key:?} has legacy $엔터$ but {} lacks wait_enter",
                    mob_path.display(),
                    path.display()
                );
                assert!(
                    !rhai.contains("\"누르세요]\""),
                    "{} event {key:?} truncates the source enter prompt",
                    path.display()
                );
            };

            for line in source.lines() {
                if let Some(key) = line.strip_prefix("#이벤트") {
                    if let Some(previous) = current_key.take() {
                        check_block(
                            previous,
                            has_enter,
                            has_interactive_start,
                            has_interactive_end,
                        );
                    }
                    let key = key.trim();
                    if key.is_empty() {
                        continue;
                    }
                    current_key = Some(key);
                    has_enter = false;
                    has_interactive_start = false;
                    has_interactive_end = false;
                } else if line.trim_start().starts_with(":$엔터$") {
                    has_enter = true;
                } else if line.trim_start().starts_with(":$입력대기출력시작") {
                    has_interactive_start = true;
                } else if line.trim_start().starts_with(":$입력대기출력끝$") {
                    has_interactive_end = true;
                }
            }
            if let Some(previous) = current_key {
                check_block(
                    previous,
                    has_enter,
                    has_interactive_start,
                    has_interactive_end,
                );
            }
        }

        assert_eq!(pauses, 22, "legacy enter-pause inventory changed");
        assert_eq!(
            interactive_boundaries, 22,
            "legacy interactive-boundary inventory changed"
        );
    }

    #[test]
    fn python_event_handler_and_legacy_directive_inventory_is_not_silently_narrowed() {
        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }

        // `objs/event.py` is the authority: retain every `func == '$…'`
        // handler, including currently-unused legacy branches.  This small
        // parser deliberately follows the source's stable single-quote form.
        let python = std::fs::read_to_string("objs/event.py").expect("Python event source");
        let handlers = python
            .split("func == '")
            .skip(1)
            .filter_map(|tail| tail.split_once('\'').map(|(name, _)| name))
            .filter(|name| name.starts_with('$'))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(handlers.len(), 87, "Python $ handler inventory changed");

        let mut paths = Vec::new();
        collect_mob_paths(std::path::Path::new("data/mob"), &mut paths);
        let mut used = std::collections::BTreeSet::new();
        let mut calls = 0_usize;
        for path in paths {
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            for line in source.lines() {
                let Some(token) = line
                    .trim()
                    .strip_prefix(':')
                    .filter(|directive| directive.starts_with('$'))
                    .and_then(|directive| directive.split_whitespace().next())
                else {
                    continue;
                };
                if handlers.contains(token) {
                    used.insert(token.to_string());
                    calls += 1;
                }
            }
        }
        assert_eq!(used.len(), 74, "actually-used Python $ handlers changed");
        assert_eq!(calls, 5_355, "legacy $ directive call inventory changed");
    }

    #[test]
    fn legacy_literal_move_resource_and_stat_directives_keep_their_rhai_calls() {
        fn normalize_event_key(key: &str) -> String {
            key.trim()
                .trim_start_matches("이벤트")
                .trim_start()
                .trim_start_matches(':')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }

        fn check_block(
            zone: &str,
            info: &serde_json::Map<String, serde_json::Value>,
            event_key: &str,
            lines: &[String],
            moves: &mut usize,
            hp: &mut usize,
            stat_changes: &mut usize,
            stat_sets: &mut usize,
        ) {
            let Some(script) = info.iter().find_map(|(key, value)| {
                (normalize_event_key(key) == event_key)
                    .then(|| value.as_str())
                    .flatten()
            }) else {
                return;
            };
            let source = std::fs::read_to_string(
                std::path::Path::new("data/script").join(zone).join(script),
            )
            .unwrap_or_else(|error| panic!("missing {zone}/{script}: {error}"));
            for line in lines {
                let trimmed = line.trim();
                let directive = trimmed.strip_prefix(':').unwrap_or(trimmed).trim_start();
                if let Some(spec) = directive.strip_prefix("$위치이동 ") {
                    let (target_zone, room) = spec
                        .trim()
                        .split_once(':')
                        .unwrap_or_else(|| panic!("invalid room spec {spec:?}"));
                    *moves += 1;
                    assert!(
                        source.contains(&format!("set_position(\"{target_zone}\", \"{room}\")")),
                        "{zone}/{script}: missing original move {spec}"
                    );
                }
                for directive in ["체력감소", "체력소모"] {
                    if let Some(amount) = trimmed
                        .strip_prefix(':')
                        .unwrap_or(trimmed)
                        .trim_start()
                        .strip_prefix(&format!("${directive} "))
                        .and_then(|value| value.trim().parse::<i64>().ok())
                    {
                        *hp += 1;
                        assert!(
                            source.contains(&format!("consume_hp({amount})")),
                            "{zone}/{script}: missing original {directive} {amount}"
                        );
                    }
                }
                for (directive, rhai_call, count) in [
                    ("특성치변경", "change_stat", &mut *stat_changes),
                    ("특성치설정", "set_stat", &mut *stat_sets),
                ] {
                    if let Some(spec) = trimmed
                        .strip_prefix(':')
                        .unwrap_or(trimmed)
                        .trim_start()
                        .strip_prefix(&format!("${directive} "))
                    {
                        let mut values = spec.split_whitespace();
                        let stat = values.next().unwrap_or_else(|| {
                            panic!("{zone}/{script}: invalid original {directive} {spec:?}")
                        });
                        let amount = values.next().unwrap_or_else(|| {
                            panic!("{zone}/{script}: invalid original {directive} {spec:?}")
                        });
                        assert!(
                            values.next().is_none(),
                            "{zone}/{script}: invalid original {directive} {spec:?}"
                        );
                        *count += 1;
                        assert!(
                            source.contains(&format!("{rhai_call}(\"{stat}\", {amount})")),
                            "{zone}/{script}: missing original {directive} {stat} {amount}"
                        );
                    }
                }
            }
        }

        let mut move_count = 0;
        let mut hp_count = 0;
        let mut stat_change_count = 0;
        let mut stat_set_count = 0;
        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }
        let mut mob_paths = Vec::new();
        collect_mob_paths(std::path::Path::new("data/mob"), &mut mob_paths);
        for path in mob_paths {
            let json_path = path.with_extension("json");
            if !json_path.exists() {
                continue;
            }
            let root: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
            )
            .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
            let info = root
                .get("몹정보")
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
            let zone = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let mut event_key: Option<String> = None;
            let mut lines = Vec::new();
            for line in std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
                .lines()
            {
                if let Some(header) = line.trim_start().strip_prefix("#이벤트") {
                    if let Some(key) = event_key.take() {
                        check_block(
                            zone,
                            info,
                            &key,
                            &lines,
                            &mut move_count,
                            &mut hp_count,
                            &mut stat_change_count,
                            &mut stat_set_count,
                        );
                    }
                    event_key = Some(normalize_event_key(header));
                    lines.clear();
                } else if event_key.is_some() && line.trim_start().starts_with("#END") {
                    check_block(
                        zone,
                        info,
                        event_key.take().as_deref().unwrap(),
                        &lines,
                        &mut move_count,
                        &mut hp_count,
                        &mut stat_change_count,
                        &mut stat_set_count,
                    );
                    lines.clear();
                } else if event_key.is_some() {
                    lines.push(line.to_string());
                }
            }
            if let Some(key) = event_key {
                check_block(
                    zone,
                    info,
                    &key,
                    &lines,
                    &mut move_count,
                    &mut hp_count,
                    &mut stat_change_count,
                    &mut stat_set_count,
                );
            }
        }
        assert_eq!(move_count, 448, "legacy move directive inventory changed");
        assert_eq!(hp_count, 52, "legacy hp directive inventory changed");
        assert_eq!(
            stat_change_count, 58,
            "legacy stat-change directive inventory changed"
        );
        assert_eq!(
            stat_set_count, 6,
            "legacy stat-set directive inventory changed"
        );
    }

    #[test]
    fn legacy_literal_item_grants_and_deletes_keep_their_rhai_calls() {
        fn normalize_event_key(key: &str) -> String {
            key.trim()
                .trim_start_matches("이벤트")
                .trim_start()
                .trim_start_matches(':')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }

        fn check_block(
            zone: &str,
            info: &serde_json::Map<String, serde_json::Value>,
            event_key: &str,
            lines: &[String],
            grants: &mut usize,
            deletes: &mut usize,
            dynamic: &mut usize,
        ) {
            let Some(script) = info.iter().find_map(|(key, value)| {
                (normalize_event_key(key) == event_key)
                    .then(|| value.as_str())
                    .flatten()
            }) else {
                return;
            };
            let source = std::fs::read_to_string(
                std::path::Path::new("data/script").join(zone).join(script),
            )
            .unwrap_or_else(|error| panic!("missing {zone}/{script}: {error}"));
            for line in lines {
                let trimmed = line.trim();
                let directive = trimmed.strip_prefix(':').unwrap_or(trimmed).trim_start();
                for (legacy_name, rhai_call, count) in [
                    ("아이템주기", "give_item", &mut *grants),
                    ("아이템삭제", "delete_item", &mut *deletes),
                ] {
                    let Some(spec) = directive.strip_prefix(&format!("${legacy_name} ")) else {
                        continue;
                    };
                    let spec = spec.split('；').next().unwrap().trim();
                    let values = spec.split_whitespace().collect::<Vec<_>>();
                    let item = *values.first().unwrap_or_else(|| {
                        panic!("{zone}/{script}: invalid original {legacy_name} {spec:?}")
                    });
                    if item.starts_with('$') {
                        *dynamic += 1;
                        continue;
                    }
                    let amount = if values.len() > 1 {
                        *values.last().unwrap()
                    } else {
                        "1"
                    };
                    assert!(
                        amount.parse::<i64>().is_ok(),
                        "{zone}/{script}: invalid original {legacy_name} {spec:?}"
                    );
                    *count += 1;
                    let item_count = if values.len() > 2 {
                        values.len() - 1
                    } else {
                        1
                    };
                    for item in values.iter().take(item_count) {
                        let exact_call = format!("{rhai_call}(\"{item}\", {amount})");
                        let conditional_call = source.lines().any(|line| {
                            line.contains(&format!("{rhai_call}("))
                                && line.contains(&format!(", {amount})"))
                        }) && source.contains(&format!("\"{item}\""));
                        assert!(
                            source.contains(&exact_call) || conditional_call,
                            "{zone}/{script}: missing original {legacy_name} {item} {amount}"
                        );
                    }
                }
            }
        }

        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }

        let mut grants = 0;
        let mut deletes = 0;
        let mut dynamic = 0;
        let mut mob_paths = Vec::new();
        collect_mob_paths(std::path::Path::new("data/mob"), &mut mob_paths);
        for path in mob_paths {
            let json_path = path.with_extension("json");
            if !json_path.exists() {
                continue;
            }
            let root: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
            )
            .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
            let info = root
                .get("몹정보")
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
            let zone = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let mut event_key: Option<String> = None;
            let mut lines = Vec::new();
            for line in std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
                .lines()
            {
                if let Some(header) = line.trim_start().strip_prefix("#이벤트") {
                    if let Some(key) = event_key.take() {
                        check_block(
                            zone,
                            info,
                            &key,
                            &lines,
                            &mut grants,
                            &mut deletes,
                            &mut dynamic,
                        );
                    }
                    event_key = Some(normalize_event_key(header));
                    lines.clear();
                } else if event_key.is_some() && line.trim_start().starts_with("#END") {
                    check_block(
                        zone,
                        info,
                        event_key.take().as_deref().unwrap(),
                        &lines,
                        &mut grants,
                        &mut deletes,
                        &mut dynamic,
                    );
                    lines.clear();
                } else if event_key.is_some() {
                    lines.push(line.to_string());
                }
            }
            if let Some(key) = event_key {
                check_block(
                    zone,
                    info,
                    &key,
                    &lines,
                    &mut grants,
                    &mut deletes,
                    &mut dynamic,
                );
            }
        }
        assert_eq!(grants, 585, "legacy literal item-grant inventory changed");
        assert_eq!(deletes, 410, "legacy literal item-delete inventory changed");
        assert_eq!(
            dynamic, 1,
            "legacy dynamic item directive inventory changed"
        );
    }

    #[test]
    fn legacy_output_directives_keep_their_rhai_text_or_python_self_rendering() {
        fn normalize_event_key(key: &str) -> String {
            key.trim()
                .trim_start_matches("이벤트")
                .trim_start()
                .trim_start_matches(':')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }

        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }

        fn check_block(
            zone: &str,
            info: &serde_json::Map<String, serde_json::Value>,
            event_key: &str,
            lines: &[String],
            outputs: &mut usize,
            rank_updates: &mut usize,
        ) {
            let Some(script) = info.iter().find_map(|(key, value)| {
                (normalize_event_key(key) == event_key)
                    .then(|| value.as_str())
                    .flatten()
            }) else {
                return;
            };
            // The legacy files contain literal ESC bytes, while a converted
            // Rhai file may retain them or spell them as `\\x1b`.  Compare
            // the rendered text, not its source-language escape spelling.
            let source = std::fs::read_to_string(
                std::path::Path::new("data/script").join(zone).join(script),
            )
            .unwrap_or_else(|error| panic!("missing {zone}/{script}: {error}"))
            .replace("\\x1b", "\x1b")
            .replace("\\\"", "\"");

            // Python initializes `rank1` and `rank2` to zero for every
            // doEvent invocation.  A `$순위갱신` can therefore emit only
            // after an earlier `$순위기록` in this same event block; orphaned
            // legacy directives (for example `가쇠종 $보`) are no-ops.
            let mut rank_recorded = false;
            for line in lines {
                let directive = line.trim().trim_start_matches(':').trim_start();
                if directive.starts_with("$순위기록 ") {
                    rank_recorded = true;
                    continue;
                }
                let (text, is_rank_update) = if let Some(text) = directive.strip_prefix("$출력 ")
                {
                    (text, false)
                } else if let Some(text) = directive.strip_prefix("$순위갱신 ") {
                    (text, true)
                } else {
                    continue;
                };
                let text = text.trim();
                if text.is_empty() {
                    continue;
                }
                if is_rank_update {
                    *rank_updates += 1;
                } else {
                    *outputs += 1;
                }
                if is_rank_update && !rank_recorded {
                    continue;
                }
                // `$순위갱신` has already rendered `[공]` for the caller in
                // Python before it reaches `sendLine`; converted Rhai keeps
                // those self-facing strings as `당신…`.  All other source
                // text remains literal in Rhai.
                let self_rendered = text
                    .replace("[공](이/가)", "당신이")
                    .replace("[공]", "당신")
                    .replace("[사용자이름]", "당신");
                assert!(
                    source.contains(text) || source.contains(&self_rendered),
                    "{zone}/{script}: missing original {} {text:?}",
                    if is_rank_update {
                        "$순위갱신"
                    } else {
                        "$출력"
                    }
                );
                if is_rank_update {
                    assert!(
                        source.contains("broadcast_output("),
                        "{zone}/{script}: original $순위갱신 must retain broadcast_output"
                    );
                    if text.contains("[공]") {
                        let self_rendered =
                            crate::hangul::post_position1(&text.replace("[공]", "당신"));
                        assert!(
                            source.lines().any(|line| {
                                (line.contains("self_output(") && line.contains(text))
                                    || line.contains(&self_rendered)
                            }),
                            "{zone}/{script}: original $순위갱신 self output must render [공] as 당신"
                        );
                    }
                } else {
                    // Python $출력 is Player.printScript(): one rendered
                    // line for the caller and the source line for every
                    // other player in the same room. Converted Rhai keeps
                    // the caller text in output() and owns the room text
                    // explicitly instead of widening it into a global
                    // rank-style broadcast.
                    let room_output = format!("room_broadcast_output(\"{text}\")");
                    assert!(
                        source.contains(&room_output),
                        "{zone}/{script}: original $출력 must retain same-room output {text:?}"
                    );
                    let self_text = text.replace("[공]", "당신");
                    let self_output = format!("output(post_position_once(\"{self_text}\"))");
                    assert!(
                        source.contains(&self_output),
                        "{zone}/{script}: original $출력 must retain Python caller rendering {text:?}"
                    );
                }
            }
        }

        let mut outputs = 0_usize;
        let mut rank_updates = 0_usize;
        let mut mob_paths = Vec::new();
        collect_mob_paths(std::path::Path::new("data/mob"), &mut mob_paths);
        for path in mob_paths {
            let json_path = path.with_extension("json");
            if !json_path.exists() {
                continue;
            }
            let root: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
            )
            .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
            let info = root
                .get("몹정보")
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
            let zone = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let mut event_key: Option<String> = None;
            let mut lines = Vec::new();
            for line in std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
                .lines()
            {
                if let Some(header) = line.trim_start().strip_prefix("#이벤트") {
                    if let Some(key) = event_key.take() {
                        check_block(zone, info, &key, &lines, &mut outputs, &mut rank_updates);
                    }
                    event_key = Some(normalize_event_key(header));
                    lines.clear();
                } else if event_key.is_some() && line.trim_start().starts_with("#END") {
                    check_block(
                        zone,
                        info,
                        event_key.take().as_deref().unwrap(),
                        &lines,
                        &mut outputs,
                        &mut rank_updates,
                    );
                    lines.clear();
                } else if event_key.is_some() {
                    lines.push(line.to_string());
                }
            }
            if let Some(key) = event_key {
                check_block(zone, info, &key, &lines, &mut outputs, &mut rank_updates);
            }
        }
        assert_eq!(outputs, 570, "legacy $출력 directive inventory changed");
        assert_eq!(
            rank_updates, 94,
            "legacy $순위갱신 directive inventory changed"
        );
    }

    #[test]
    fn legacy_event_checks_keep_their_rhai_predicates_and_negation() {
        fn normalize_event_key(key: &str) -> String {
            key.trim()
                .trim_start_matches("이벤트")
                .trim_start()
                .trim_start_matches(':')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }

        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }

        fn check_block(
            zone: &str,
            info: &serde_json::Map<String, serde_json::Value>,
            event_key: &str,
            lines: &[String],
            checks: &mut usize,
        ) {
            let Some(script) = info.iter().find_map(|(key, value)| {
                (normalize_event_key(key) == event_key)
                    .then(|| value.as_str())
                    .flatten()
            }) else {
                return;
            };
            let source = std::fs::read_to_string(
                std::path::Path::new("data/script").join(zone).join(script),
            )
            .unwrap_or_else(|error| panic!("missing {zone}/{script}: {error}"));
            for line in lines {
                let directive = line.trim().trim_start_matches(':').trim_start();
                let (key, predicate) = if let Some(key) = directive.strip_prefix("$이벤트확인! ")
                {
                    (key, "!check_event")
                } else if let Some(key) = directive.strip_prefix("$이벤트확인 ") {
                    (key, "check_event")
                } else {
                    continue;
                };
                let key = key.split('；').next().unwrap().trim();
                assert!(!key.is_empty(), "{zone}/{script}: empty original event key");
                *checks += 1;
                assert!(
                    source.contains(&format!("{predicate}(\"{key}\")")),
                    "{zone}/{script}: missing original ${} {key}",
                    if predicate.starts_with('!') {
                        "이벤트확인!"
                    } else {
                        "이벤트확인"
                    }
                );
            }
        }

        let mut checks = 0_usize;
        let mut mob_paths = Vec::new();
        collect_mob_paths(std::path::Path::new("data/mob"), &mut mob_paths);
        for path in mob_paths {
            let json_path = path.with_extension("json");
            if !json_path.exists() {
                continue;
            }
            let root: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
            )
            .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
            let info = root
                .get("몹정보")
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
            let zone = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let mut event_key: Option<String> = None;
            let mut lines = Vec::new();
            for line in std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
                .lines()
            {
                if let Some(header) = line.trim_start().strip_prefix("#이벤트") {
                    if let Some(key) = event_key.take() {
                        check_block(zone, info, &key, &lines, &mut checks);
                    }
                    event_key = Some(normalize_event_key(header));
                    lines.clear();
                } else if event_key.is_some() && line.trim_start().starts_with("#END") {
                    check_block(
                        zone,
                        info,
                        event_key.take().as_deref().unwrap(),
                        &lines,
                        &mut checks,
                    );
                    lines.clear();
                } else if event_key.is_some() {
                    lines.push(line.to_string());
                }
            }
            if let Some(key) = event_key {
                check_block(zone, info, &key, &lines, &mut checks);
            }
        }
        assert_eq!(
            checks, 1_012,
            "legacy $이벤트확인 directive inventory changed"
        );
    }

    fn place_event_mob(zone: &str, source_key: &str, room: &str) -> (String, u64) {
        static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let mut world = crate::world::get_world_state().write().unwrap();
        let data = world
            .mob_cache
            .load_mob(zone, source_key)
            .unwrap_or_else(|_| panic!("missing event mob fixture {zone}:{source_key}"))
            .clone();
        let fixture_id = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let key = format!(
            "{zone}:{source_key}-회귀-{}-{fixture_id}",
            std::process::id()
        );
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
    fn legacy_combat_start_keeps_python_failure_order_and_only_starts_once() {
        let room = format!("전투시작회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("백두산", "3400", &room);
        let mut body = Body::new();
        body.set("이름", "전투시작회귀");

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "백두산", &room, "음양소요 비무")
                .expect("combat event")
        else {
            panic!("combat start was not an event");
        };
        assert!(output_lines.iter().any(|line| line.contains("감히")));
        assert_eq!(body.act, crate::player::ActState::Fight);
        assert_eq!(
            crate::script::combat_commands::combat_target_instance_ids(&body),
            vec![instance_id]
        );

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "백두산", &room, "음양소요 비무")
                .expect("repeat combat event")
        else {
            panic!("repeat combat start was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("이미 공격중이에요")));

        crate::script::combat_commands::remove_combat_target_instance_id(&mut body, instance_id);
        body.act = crate::player::ActState::Stand;
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room_mut("백두산", &room)
                .unwrap()
                .iter_mut()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            mob.act = 1;
            mob.targets = vec!["다른무림인".into()];
        }
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "백두산", &room, "음양소요 비무")
                .expect("busy combat event")
        else {
            panic!("busy combat start was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("다른 사람과 전투중")));

        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room_mut("백두산", &room)
                .unwrap()
                .iter_mut()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            mob.act = 0;
            mob.targets.clear();
        }
        body.act = crate::player::ActState::Fight;
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "백두산", &room, "음양소요 비무")
                .expect("self busy combat event")
        else {
            panic!("self busy combat start was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("현재의 비무에 신경을 집중")));

        mark_event_mob_corpse("백두산", &room, instance_id);
        body.act = crate::player::ActState::Stand;
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "백두산", &room, "음양소요 비무")
                .expect("corpse combat event")
        else {
            panic!("corpse combat start was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("무슨 말인지 모르겠어요")));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn legacy_combat_start_directives_keep_a_rhai_combat_transition() {
        fn normalize_event_key(key: &str) -> String {
            key.trim()
                .trim_start_matches("이벤트")
                .trim_start()
                .trim_start_matches(':')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }

        fn collect_mob_paths(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(directory).expect("legacy mob directory") {
                let path = entry.expect("legacy mob entry").path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|name| name != "backup") {
                        collect_mob_paths(&path, paths);
                    }
                } else if path.extension().is_some_and(|extension| extension == "mob") {
                    paths.push(path);
                }
            }
        }

        let mut paths = Vec::new();
        collect_mob_paths(std::path::Path::new("data/mob"), &mut paths);
        let mut calls = 0_usize;
        for path in paths {
            let json_path = path.with_extension("json");
            if !json_path.exists() {
                continue;
            }
            let root: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
            )
            .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
            let info = root
                .get("몹정보")
                .and_then(serde_json::Value::as_object)
                .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
            let zone = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            let mut event_key: Option<String> = None;
            let mut lines = Vec::new();
            let mut check_block = |event_key: &str, lines: &[String]| {
                if !lines.iter().any(|line| line.trim() == ":$전투시작") {
                    return;
                }
                let script = info
                    .iter()
                    .find_map(|(key, value)| {
                        (normalize_event_key(key) == event_key)
                            .then(|| value.as_str())
                            .flatten()
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "{zone}/{}: missing Rhai mapping for {event_key}",
                            path.display()
                        )
                    });
                let rhai_path = std::path::Path::new("data/script").join(zone).join(script);
                let rhai = std::fs::read_to_string(&rhai_path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", rhai_path.display()));
                assert!(
                    ["try_start_selected_mob_combat()", "start_event_combat()"][..]
                        .iter()
                        .any(|call| rhai.contains(call)),
                    "{zone}/{script}: missing Rhai combat transition for original $전투시작"
                );
                calls += lines
                    .iter()
                    .filter(|line| line.trim() == ":$전투시작")
                    .count();
            };
            for line in source.lines() {
                if let Some(header) = line.trim_start().strip_prefix("#이벤트") {
                    if let Some(key) = event_key.take() {
                        check_block(&key, &lines);
                    }
                    event_key = Some(normalize_event_key(header));
                    lines.clear();
                } else if event_key.is_some() && line.trim_start().starts_with("#END") {
                    check_block(event_key.take().as_deref().unwrap(), &lines);
                    lines.clear();
                } else if event_key.is_some() {
                    lines.push(line.to_string());
                }
            }
            if let Some(key) = event_key {
                check_block(&key, &lines);
            }
        }
        assert_eq!(calls, 82, "legacy $전투시작 directive inventory changed");
    }

    #[test]
    fn direct_legacy_combat_start_scripts_keep_python_refusal_messages() {
        // These source scripts originally used a fire-and-forget helper.  A
        // real `$전투시작` must still reject an already-fighting caller and a
        // corpse, just as the Python dispatcher does.
        let room = format!("직접전투거절회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("감숙성", "1위", &room);

        let mut fighting = Body::new();
        fighting.set("이름", "직접전투자기전투회귀");
        fighting.act = crate::player::ActState::Fight;
        add_test_items(&mut fighting, "도전장9", 1);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut fighting, "감숙성", &room, "천마혈검 도전")
                .expect("source challenge event")
        else {
            panic!("source challenge event must finish immediately");
        };
        assert!(
            output_lines
                .iter()
                .any(|line| line.contains("현재의 비무에 신경")),
            "{output_lines:?}"
        );
        assert!(
            !crate::script::combat_commands::combat_target_instance_ids(&fighting)
                .contains(&instance_id)
        );

        mark_event_mob_corpse("감숙성", &room, instance_id);
        let mut corpse = Body::new();
        corpse.set("이름", "직접전투시체회귀");
        add_test_items(&mut corpse, "도전장9", 1);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut corpse, "감숙성", &room, "천마혈검 도전")
                .expect("corpse challenge event")
        else {
            panic!("corpse challenge event must finish immediately");
        };
        assert!(
            output_lines
                .iter()
                .any(|line| line.contains("무슨 말인지 모르겠어요")),
            "{output_lines:?}"
        );
        assert!(
            !crate::script::combat_commands::combat_target_instance_ids(&corpse)
                .contains(&instance_id)
        );

        crate::world::get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/직접전투자기전투회귀.json");
        let _ = std::fs::remove_file("data/user/직접전투시체회귀.json");
    }

    #[test]
    fn legacy_stat_change_updates_the_python_named_attribute() {
        let room = format!("특성치변경회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("사천성", "무면옹", &room);
        let mut body = Body::new();
        body.set("이름", "특성치변경회귀");
        body.set("힘", 99_i64);
        add_test_items(&mut body, "합성6-2", 1);

        super::try_mob_event(&mut body, "사천성", &room, "거지 대화").expect("stat-change event");
        assert_eq!(body.get_int("힘"), 100);
        assert!(!body_has_item_spec(&body, "합성6-2"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn legacy_item_option_and_gender_directives_change_only_the_selected_state() {
        let room = format!("아이템성별동작회귀-{}", std::process::id());
        let (smith_key, _) = place_event_mob("낙양성", "합체맨", &room);
        // Python `$아이템옵션삭제 $변수:1` rejects only a missing item.  An
        // existing item whose `옵션` is already the empty string still runs
        // `Item.delOption()` (because `'' != None`) and follows the success
        // prose.  Keep both sides of that slightly surprising rule tied to
        // the authored NPC event.
        let mut missing_item = Body::new();
        missing_item.set("이름", "속성삭제없음회귀");
        let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event(
            &mut missing_item,
            "낙양성",
            &room,
            "대장장이 없는검 속성삭제",
        )
        .expect("missing option-delete event") else {
            panic!("missing option-delete event returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("뭘 말인가")));

        let mut empty_option = Body::new();
        empty_option.set("이름", "속성삭제빈옵션회귀");
        let mut empty_item = Object::new();
        empty_item.set("이름", "빈옵션검");
        empty_item.set("인덱스", "빈옵션검");
        empty_option
            .object
            .objs
            .push(Arc::new(Mutex::new(empty_item)));
        let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event(
            &mut empty_option,
            "낙양성",
            &room,
            "대장장이 빈옵션검 속성삭제",
        )
        .expect("empty option-delete event") else {
            panic!("empty option-delete event returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("좋아!!")));

        let mut body = Body::new();
        body.set("이름", "아이템성별동작회귀");
        let mut item = Object::new();
        item.set("이름", "시험검");
        item.set("인덱스", "시험검");
        item.set("옵션", "힘 10");
        item.set("아이템속성", "버리지못함");
        body.object.objs.push(Arc::new(Mutex::new(item)));

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "대장장이 시험검 속성삭제")
                .expect("option-delete event")
        else {
            panic!("option-delete event returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("좋아!!")));
        let item = body.object.objs[0].lock().unwrap();
        assert!(!item.attr.contains_key("옵션"));
        assert!(!item.attr.contains_key("아이템속성"));
        drop(item);

        let (scholar_key, _) = place_event_mob("무림맹", "대학사", &room);
        for (initial_sex, expected_sex, marker) in [
            ("여", "남", "남자가 되었습니다"),
            ("남", "여", "여자가 되었습니다"),
        ] {
            let mut sex_body = Body::new();
            sex_body.set("이름", format!("규화성별{initial_sex}회귀"));
            sex_body.set("성별", initial_sex);
            sex_body.set("힘", 100_i64);
            sex_body.set("최고내공", 50_i64);
            add_test_items(&mut sex_body, "규화보전", 1);
            let CommandResult::MobEvent { output_lines, .. } =
                super::try_mob_event(&mut sex_body, "무림맹", &room, "대학사 규화보전")
                    .expect("sunflower-manual event")
            else {
                panic!("sunflower-manual event returned a non-event result");
            };

            // Python `$성별확인` skips its first block only for 남.  Thus
            // the following `$남자설정` / `$여자설정` alternates the sex,
            // after the preceding skill, item, stat, and event directives.
            assert_eq!(sex_body.get_string("성별"), expected_sex);
            assert!(output_lines.iter().any(|line| line.contains(marker)));
            assert!(sex_body.skill_list.iter().any(|skill| skill == "규화보전"));
            assert!(!body_has_item_spec(&sex_body, "규화보전"));
            assert_eq!(sex_body.get_int("힘"), 400);
            assert_eq!(sex_body.get_int("최고내공"), 350);
            assert_eq!(get_user_event(&sex_body, "규화보전이벤트"), "1");
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&smith_key);
        world.mob_cache.remove_mob(&scholar_key);
    }

    #[test]
    fn legacy_stat_set_replaces_the_python_named_attribute() {
        let mut body = Body::new();
        body.set("이름", "특성치설정회귀");
        body.set("레벨", 99_i64);
        body.set("나이", 100_i64);
        run_zone_event(&mut body, "낙양성", "요월_대화_대_백세.rhai", None);
        assert_eq!(body.get_int("레벨"), 1_000);

        let mut too_young = Body::new();
        too_young.set("이름", "특성치설정미달회귀");
        too_young.set("레벨", 99_i64);
        too_young.set("나이", 99_i64);
        let (output, _) = run_zone_event(&mut too_young, "낙양성", "요월_대화_대_백세.rhai", None);
        assert_eq!(too_young.get_int("레벨"), 99);
        assert!(output.iter().any(|line| line.contains("100살 안된 어린이")));
    }

    #[test]
    fn legacy_stat_copy_changes_selected_mob_combat_attributes() {
        let room = format!("특성치복사회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("혈탑", "무림인00", &room);
        let mut body = Body::new();
        body.set("이름", "특성치복사회귀");
        body.set("최고체력", 1_000_i64);
        body.set("힘", 200_i64);
        body.set("레벨", 50_i64);
        body.set("맷집", 70_i64);
        body.set("민첩성", 80_i64);

        super::try_mob_event(&mut body, "혈탑", &room, "혈탑살수 공격")
            .expect("stat-copy combat event");
        let world = crate::world::get_world_state().read().unwrap();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room("혈탑", &room)
            .into_iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert_eq!(mob.hp, 20_000);
        assert_eq!(mob.max_hp, 20_000);
        assert_eq!(mob.strength, 600);
        assert_eq!(mob.level, 200);
        assert_eq!(mob.arm, 70);
        assert_eq!(mob.agility, 80);
        assert_eq!(
            (mob.miss, mob.hit, mob.luck, mob.critical),
            (0, 400, 100, 100)
        );
        drop(world);

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn low_and_high_stat_copy_variants_match_python_multipliers() {
        let mut body = Body::new();
        body.set("이름", "특성치복사저고회귀");
        body.set("최고체력", 1_000_i64);
        body.set("힘", 200_i64);
        body.set("레벨", 50_i64);
        body.set("맷집", 70_i64);
        body.set("민첩성", 80_i64);

        let low_room = format!("특성치복사저회귀-{}", std::process::id());
        let (low_key, low_instance) = place_event_mob("동정호", "적송", &low_room);
        super::try_mob_event(&mut body, "동정호", &low_room, "적송 약속 대화")
            .expect("low stat-copy event");
        let world = crate::world::get_world_state().read().unwrap();
        let low = world
            .mob_cache
            .get_all_mobs_in_room("동정호", &low_room)
            .into_iter()
            .find(|mob| mob.instance_id == low_instance)
            .unwrap();
        assert_eq!((low.hp, low.max_hp), (1_000, 1_000));
        assert_eq!(
            (low.strength, low.level, low.arm, low.agility),
            (200, 200, 70, 80)
        );
        assert_eq!(
            (low.miss, low.hit, low.luck, low.critical),
            (0, 400, 100, 100)
        );
        drop(world);

        // A fresh target is necessary because the first event placed the
        // player in combat with 적송.
        body.clear_target(None);
        body.act = crate::player::ActState::Stand;
        let high_room = format!("특성치복사고회귀-{}", std::process::id());
        let (high_key, high_instance) = place_event_mob("수정동굴", "반영", &high_room);
        super::try_mob_event(&mut body, "수정동굴", &high_room, "반영 공격")
            .expect("high stat-copy event");
        let world = crate::world::get_world_state().read().unwrap();
        let high = world
            .mob_cache
            .get_all_mobs_in_room("수정동굴", &high_room)
            .into_iter()
            .find(|mob| mob.instance_id == high_instance)
            .unwrap();
        assert_eq!((high.hp, high.max_hp), (30_000, 30_000));
        assert_eq!(
            (high.strength, high.level, high.arm, high.agility),
            (1_600, 200, 210, 80)
        );
        assert_eq!(
            (high.miss, high.hit, high.luck, high.critical),
            (0, 400, 100, 100)
        );
        drop(world);

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&low_key);
        world.mob_cache.remove_mob(&high_key);
    }

    #[test]
    fn intermediate_training_applies_only_python_training_stats_after_combat_starts() {
        let room = format!("중급수련회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("중급수련장", "교관", &room);
        let mut body = Body::new();
        body.set("이름", "중급수련회귀");
        body.set("힘", 25_099_i64);
        body.set("레벨", 77_i64);

        super::try_mob_event(&mut body, "중급수련장", &room, "교관 수련")
            .expect("intermediate training event");
        assert_eq!(body.act, crate::player::ActState::Fight);
        let world = crate::world::get_world_state().read().unwrap();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room("중급수련장", &room)
            .into_iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert_eq!(mob.strength, 251);
        assert_eq!(mob.level, 227);
        assert_eq!(mob.miss, 0);
        assert_eq!(mob.act, 1);
        drop(world);

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn eundun_event_keeps_python_reset_and_transfer_state_before_moving() {
        let room = format!("은둔칩거회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("백두산", "4100", &room);
        let mut body = Body::new();
        body.set("이름", "은둔칩거회귀");
        body.set("성격", "선인");
        body.set("힘", 2_100_i64);
        body.set("레벨", 999_i64);
        body.set("현재경험치", 100_i64);
        body.set("힘경험치", 100_i64);
        body.set("맷집경험치", 100_i64);
        body.set("전직", 3_i64);
        super::set_user_event(&mut body, "선인끝", "1");

        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "백두산", &room, "연남천 은둔칩거")
                .expect("eundun event")
        else {
            panic!("eundun returned a non-event result");
        };
        assert_eq!(set_position, Some(("전직".into(), "1".into())));
        assert_eq!(body.get_int("힘"), 100);
        assert_eq!(body.get_int("레벨"), 1);
        assert_eq!(body.get_int("현재경험치"), 0);
        assert_eq!(body.get_int("힘경험치"), 0);
        assert_eq!(body.get_int("맷집경험치"), 0);
        assert_eq!(body.get_int("전직"), 4);
        assert_eq!(body.get_string("기존성격"), "선인");
        assert_eq!(body.get_string("성격"), "은둔칩거");
        assert_eq!(body.get_string("이벤트설정리스트"), "은둔칩거끝");
        assert_eq!(body.get_string("위치각인"), "낙양성:1");

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/은둔칩거회귀.json");
    }

    #[test]
    fn lottery_event_uses_full_legacy_pool_and_marks_the_single_reward_untradeable() {
        let pool = super::lottery_attribute_item_index;
        let source = std::fs::read_to_string("data/mob/낙양성/복권맨.mob").unwrap();
        let line = source
            .lines()
            .find(|line| line.starts_with(":$속성템주기 "))
            .unwrap();
        let candidates = line
            .trim_start_matches(":$속성템주기 ")
            .split_whitespace()
            .collect::<Vec<_>>();
        for index in &candidates[..candidates.len() - 1] {
            assert!(
                super::object_from_item_json(index).is_some(),
                "Python getStrCnt may select every authored lottery candidate: {index}"
            );
        }
        let selected = pool().expect("legacy lottery candidate");
        assert!(candidates[..candidates.len() - 1].contains(&selected.as_str()));

        let room = format!("복권속성회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "복권맨", &room);
        let mut body = Body::new();
        body.set("이름", "복권속성회귀");
        body.set("레벨", 100_i64);
        add_test_items(&mut body, "강철판", 1);
        super::try_mob_event(&mut body, "낙양성", &room, "복권맨 겜블").expect("lottery event");
        assert!(!body_has_item_spec(&body, "강철판"));
        let reward = body.object.objs.first().unwrap().lock().unwrap();
        assert!(reward.checkAttr("아이템속성", "버리지못함"));
        assert!(reward.checkAttr("아이템속성", "줄수없음"));
        drop(reward);
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn lottery_herb_reward_keeps_python_get_str_cnt_candidate_pool() {
        let source = std::fs::read_to_string("data/mob/낙양성/복권맨.mob").unwrap();
        let line = source
            .lines()
            .find(|line| line.contains("$아이템주기 합성1 합성2"))
            .expect("legacy herb lottery directive");
        let words = line
            .split("$아이템주기")
            .nth(1)
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>();
        let candidates = &words[..words.len() - 1];
        let mut body = Body::new();
        body.set("이름", "복권약초회귀");
        add_test_items(&mut body, "복권3", 1);

        run_zone_event(&mut body, "낙양성", "복권맨_긁어_긁_복권.rhai", None);

        assert!(!body_has_item_spec(&body, "복권3"));
        assert!(body.object.objs.is_empty());
        let items = body
            .object
            .inv_stack
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(key, count)| (key.as_str(), *count))
            .collect::<Vec<_>>();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].1, 1);
        assert!(
            candidates.contains(&items[0].0),
            "unexpected lottery herb index {:?}",
            items[0]
        );
    }

    #[test]
    fn legacy_hp_loss_uses_python_minus_hp_zero_floor() {
        let room = format!("체력감소회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("사천성", "무면옹", &room);
        let mut body = Body::new();
        body.set("이름", "체력감소회귀");
        body.set("체력", 100_i64);

        super::try_mob_event(&mut body, "사천성", &room, "거지 대화").expect("hp-loss event");
        assert_eq!(body.get_int("체력"), 0);
        assert_eq!(body.act, crate::player::ActState::Death);
        assert!(body.targets.is_empty());
        assert!(body.active_skills.is_empty());
        let death_events =
            crate::script::combat_commands::take_combat_presentation_events(&mut body);
        assert!(death_events.iter().any(|event| {
            event
                .clone()
                .try_cast::<rhai::Map>()
                .and_then(|event| event.get("kind").cloned())
                .is_some_and(|kind| kind.to_string() == "player_death")
        }));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn migrated_hp_loss_directives_keep_the_python_fallback_branches() {
        let _rank_guard = RANK_TEST_LOCK.lock().unwrap();
        let mut body = Body::new();
        body.set("이름", "체력분기회귀");

        let lake_room = format!("천지호체력회귀-{}", std::process::id());
        let (lake_key, _) = place_event_mob("백두산", "천지호", &lake_room);
        body.set("체력", 500_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "백두산", &lake_room, "천지 허공답보")
                .expect("lake fallback event")
        else {
            panic!("lake fallback returned a non-event result");
        };
        assert!(
            output_lines
                .iter()
                .any(|line| line.contains("호수에 떨어져")),
            "{output_lines:?}"
        );
        assert_eq!(body.get_int("체력"), 0);

        let paper_room = format!("창호지체력회귀-{}", std::process::id());
        let (paper_key, _) = place_event_mob("낙양성", "1-2", &paper_room);
        body.set("체력", 500_i64);
        body.set("최고내공", 0_i64);
        crate::world::rank::rank_clear("최고내공");
        // Python `$순위기록 200 최고내공` accepts rank 2 as well.  Fill the
        // complete 200-slot board so this body really takes the rank-0
        // fallback branch rather than merely missing first place.
        for rank in 0..200 {
            crate::world::rank::rank_write(
                "최고내공",
                &format!("창호지기존순위자{rank}"),
                10_000 - rank,
                200,
            );
        }
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &paper_room, "창호지 건너")
                .expect("paper fallback event")
        else {
            panic!("paper fallback returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("철푸덕")));
        assert_eq!(body.get_int("체력"), 400);

        let master_room = format!("광무체력회귀-{}", std::process::id());
        let (master_key, _) = place_event_mob("낙양성", "기타맨", &master_room);
        body.set("체력", 500_i64);
        body.set("이벤트설정리스트", "무공삭제");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &master_room, "광무 없는무공 무공삭제")
                .expect("master missing-skill event")
        else {
            panic!("master missing-skill returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("우롱")));
        assert_eq!(body.get_int("체력"), 0);
        assert!(get_user_event(&body, "무공삭제").is_empty());

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in [lake_key, paper_key, master_key] {
            world.mob_cache.remove_mob(&key);
        }
        crate::world::rank::rank_clear("최고내공");
    }

    #[test]
    fn legacy_corpse_unique_rewards_keep_their_live_target_branches() {
        clear_test_oneitems(&["73", "66", "71"]);
        let mut body = Body::new();
        body.set("이름", "시체기연회귀");

        let sword_room = format!("패마검회귀-{}", std::process::id());
        let (sword_key, sword_instance) = place_event_mob("산서성", "8", &sword_room);
        super::try_mob_event(&mut body, "산서성", &sword_room, "수라혈마존 부셔")
            .expect("demon sword event");
        assert!(body_has_item_spec(&body, "73"));
        assert_eq!(get_user_event(&body, "패마검"), "1");
        {
            let world = crate::world::get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room("산서성", &sword_room)
                .into_iter()
                .find(|mob| mob.instance_id == sword_instance)
                .unwrap();
            assert!(!mob.alive && mob.act == 2);
        }

        clear_test_oneitems(&["66"]);
        let devil_room = format!("천마신검회귀-{}", std::process::id());
        let (devil_key, _) = place_event_mob("낙양성", "88", &devil_room);
        super::try_mob_event(&mut body, "낙양성", &devil_room, "백골 부셔")
            .expect("heavenly demon sword event");
        assert!(body_has_item_spec(&body, "66"));
        assert_eq!(get_user_event(&body, "아수라혈천마왕"), "1");

        clear_test_oneitems(&["71"]);
        let elder_room = format!("태상노군회귀-{}", std::process::id());
        let (elder_key, _) = place_event_mob("호북성", "36", &elder_room);
        super::try_mob_event(&mut body, "호북성", &elder_room, "태상노군 절")
            .expect("taesang elder event");
        assert!(body_has_item_spec(&body, "71"));
        assert!(body_has_item_spec(&body, "492"));
        assert_eq!(get_user_event(&body, "태상노군"), "1");

        let child_room = format!("개구리가죽회귀-{}", std::process::id());
        let (child_key, _) = place_event_mob("안휘성", "40", &child_room);
        body.set("이벤트설정리스트", "황소개구리");
        add_test_items(&mut body, "개구리가죽", 1);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "안휘성", &child_room, "꼬마 대화")
                .expect("frog skin delivery event")
        else {
            panic!("frog skin delivery returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("선물로")));
        assert!(!body_has_item_spec(&body, "개구리가죽"));
        assert!(get_user_event(&body, "황소개구리").is_empty());
        assert_eq!(get_user_event(&body, "황소개구리끝"), "1");

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in [sword_key, devil_key, elder_key, child_key] {
            world.mob_cache.remove_mob(&key);
        }
        clear_test_oneitems(&["73", "66", "71"]);
    }

    #[test]
    fn migrated_event_state_transitions_keep_the_python_followup_paths() {
        clear_test_oneitems(&["황룡마조"]);
        let mut body = Body::new();
        body.set("이름", "이벤트전이회귀");

        let guard_room = format!("문지기전이회귀-{}", std::process::id());
        let (guard_key, _) = place_event_mob("하북성", "24", &guard_room);
        add_test_items(&mut body, "649", 1);
        super::try_mob_event(&mut body, "하북성", &guard_room, "문지기 대화")
            .expect("guard pass event");
        assert_eq!(get_user_event(&body, "문지기"), "1");

        let elder_room = format!("육자홍전이회귀-{}", std::process::id());
        let (elder_key, _) = place_event_mob("강소성", "육자홍", &elder_room);
        super::try_mob_event(&mut body, "강소성", &elder_room, "육자홍 절")
            .expect("yuk jahong first bow event");
        assert_eq!(get_user_event(&body, "황룡마조1"), "1");
        assert!(get_user_event(&body, "황룡마조2").is_empty());

        let gate_room = format!("철문전이회귀-{}", std::process::id());
        let (gate_key, _) = place_event_mob("동정호", "23", &gate_room);
        body.set("이벤트설정리스트", "철비묵념");
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "동정호", &gate_room, "철문 열어")
                .expect("iron gate event")
        else {
            panic!("iron gate returned a non-event result");
        };
        assert_eq!(
            set_position,
            Some(("동정호".to_string(), "227".to_string()))
        );
        assert!(get_user_event(&body, "철비묵념").is_empty());

        let cave_room = format!("오지산전이회귀-{}", std::process::id());
        let (cave_key, _) = place_event_mob("귀주성", "7", &cave_room);
        body.set("레벨", 800_i64);
        body.set("이벤트설정리스트", "오지산초동끝");
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "귀주성", &cave_room, "초동 오지산 대화")
                .expect("cave guide event")
        else {
            panic!("cave guide returned a non-event result");
        };
        assert_eq!(
            set_position,
            Some(("귀주성".to_string(), "179".to_string()))
        );
        assert!(get_user_event(&body, "오지산초동끝").is_empty());

        let warrior_room = format!("무성호법전이회귀-{}", std::process::id());
        let (warrior_key, warrior_instance) = place_event_mob("하북성", "25", &warrior_room);
        super::set_user_event(&mut body, "전투", "1");
        super::set_user_event(&mut body, "문지기", "1");
        mark_event_mob_corpse("하북성", &warrior_room, warrior_instance);
        super::try_mob_event(&mut body, "하북성", &warrior_room, "무성호법 영대혈 눌러")
            .expect("warrior acupoint event");
        {
            let world = crate::world::get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room("하북성", &warrior_room)
                .into_iter()
                .find(|mob| mob.instance_id == warrior_instance)
                .unwrap();
            assert!(!mob.alive && mob.act == 3);
        }
        assert_eq!(get_user_event(&body, "무성호법"), "1");
        assert!(get_user_event(&body, "전투").is_empty());
        assert!(get_user_event(&body, "문지기").is_empty());

        let child_room = format!("꼬마전이회귀-{}", std::process::id());
        let (child_key, _) = place_event_mob("안휘성", "40", &child_room);
        body.set("이벤트설정리스트", "");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "안휘성", &child_room, "꼬마 대화")
                .expect("child initial dialogue")
        else {
            panic!("child initial dialogue returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("개구리가죽")));
        assert_eq!(get_user_event(&body, "황소개구리"), "1");

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in [
            guard_key,
            elder_key,
            gate_key,
            cave_key,
            warrior_key,
            child_key,
        ] {
            world.mob_cache.remove_mob(&key);
        }
        clear_test_oneitems(&["황룡마조"]);
    }

    #[test]
    fn cheong_ubi_keeps_the_python_maetaegyeol_successor_chain() {
        let room = format!("진매타결회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("귀주성", "청우비", &room);
        let mut body = Body::new();
        body.set("이름", "진매타결회귀");
        body.skill_list.push("매타결".to_string());

        super::try_mob_event(&mut body, "귀주성", &room, "청우비 대화")
            .expect("maetaegyeol initial event");
        assert_eq!(get_user_event(&body, "진매타결"), "1");
        for (before, after) in [
            ("진매타결", "진매타결1"),
            ("진매타결1", "진매타결2"),
            ("진매타결2", "진매타결3"),
        ] {
            super::try_mob_event(&mut body, "귀주성", &room, "청우비 대화")
                .expect("maetaegyeol followup event");
            assert!(get_user_event(&body, before).is_empty());
            assert_eq!(get_user_event(&body, after), "1");
        }
        super::try_mob_event(&mut body, "귀주성", &room, "청우비 대화")
            .expect("maetaegyeol teaching event");
        assert!(get_user_event(&body, "진매타결3").is_empty());
        assert!(!body.skill_list.iter().any(|skill| skill == "매타결"));
        assert!(body.skill_list.iter().any(|skill| skill == "진매타결"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn legacy_stone_skill_rewards_teach_only_to_players_missing_the_skill() {
        let mut body = Body::new();
        body.set("이름", "비석전수회귀");

        let tower_room = format!("토룡십구장회귀-{}", std::process::id());
        let (tower_key, _) = place_event_mob("백층탑", "비석90", &tower_room);
        super::try_mob_event(&mut body, "백층탑", &tower_room, "비석 눌러")
            .expect("tower stone teaching event");
        assert!(body.skill_list.iter().any(|skill| skill == "토룡십구장"));
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "백층탑", &tower_room, "비석 눌러")
                .expect("tower stone followup event")
        else {
            panic!("tower stone followup returned a non-event result");
        };
        assert_eq!(
            set_position,
            Some(("백층탑".to_string(), "290".to_string()))
        );

        let hero_room = format!("사폭풍흑핵열회귀-{}", std::process::id());
        let (hero_key, _) = place_event_mob("영웅문", "돌비석", &hero_room);
        super::try_mob_event(&mut body, "영웅문", &hero_room, "돌비석 무공 파해")
            .expect("hero stone teaching event");
        assert!(body.skill_list.iter().any(|skill| skill == "사폭풍흑핵열"));
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "영웅문", &hero_room, "돌비석 무공 파해")
                .expect("hero stone followup event")
        else {
            panic!("hero stone followup returned a non-event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("다시 읽어봐도")));

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in [tower_key, hero_key] {
            world.mob_cache.remove_mob(&key);
        }
    }

    #[test]
    fn mirror_and_turtle_keep_python_inner_power_and_skill_count_messages() {
        let mut body = Body::new();
        body.set("이름", "거울거북회귀");

        let mirror_room = format!("거울회귀-{}", std::process::id());
        let (mirror_key, _) = place_event_mob("영웅문", "거울", &mirror_room);
        body.set("최고내공", 33_999_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "영웅문", &mirror_room, "거울 들")
                .expect("mirror power gate")
        else {
            panic!("mirror power gate returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("내공 34000")));
        body.set("최고내공", 34_000_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "영웅문", &mirror_room, "거울 들")
                .expect("mirror skill gate")
        else {
            panic!("mirror skill gate returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("무공 142개")));
        body.skill_list = (0..142).map(|n| format!("회귀무공{n}")).collect();
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut body, "영웅문", &mirror_room, "거울 들")
                .expect("mirror success")
        else {
            panic!("mirror success returned a non-event result");
        };
        assert_eq!(
            set_position,
            Some(("영웅문".to_string(), "0101".to_string()))
        );

        let turtle_room = format!("거북회귀-{}", std::process::id());
        let (turtle_key, _) = place_event_mob("용궁", "4700a", &turtle_room);
        body.set("최고내공", 33_999_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "용궁", &turtle_room, "대왕거북 타")
                .expect("turtle power gate")
        else {
            panic!("turtle power gate returned a non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("내공 34000")));

        let mut world = crate::world::get_world_state().write().unwrap();
        for key in [mirror_key, turtle_key] {
            world.mob_cache.remove_mob(&key);
        }
    }

    #[test]
    fn milestone_dialogues_keep_python_threshold_failure_messages() {
        let cases = [
            ("지아_대화_대_힘백.rhai", "힘", 99, "힘 100 되면"),
            ("지아_대화_대_힘오백.rhai", "힘", 499, "힘 500 되면"),
            ("지아_대화_대_힘천.rhai", "힘", 999, "힘 1000 되면"),
            ("요월_대화_대_백세.rhai", "나이", 99, "100살 안된"),
            ("요월_대화_대_백오십세.rhai", "나이", 149, "150살 안된"),
            ("요월_대화_대_이백세.rhai", "나이", 199, "200살 안된"),
            ("요월_대화_대_힘백.rhai", "힘", 99, "힘 100 되면"),
            ("요월_대화_대_힘오백.rhai", "힘", 499, "힘 500 되면"),
            ("요월_대화_대_힘천.rhai", "힘", 999, "힘 1000 되면"),
            ("밍밍_대화_대_백세.rhai", "나이", 99, "100살 안된"),
            ("밍밍_대화_대_백오십세.rhai", "나이", 149, "150살 안된"),
            ("밍밍_대화_대_이백세.rhai", "나이", 199, "200살 안된"),
        ];
        for (script, stat, value, expected) in cases {
            let mut body = Body::new();
            body.set("이름", format!("기념일회귀{script}"));
            body.set(stat, value);
            let (output, _) = run_zone_event(&mut body, "낙양성", script, None);
            assert!(
                output.iter().any(|line| line.contains(expected)),
                "{script}: {output:?}"
            );
        }

        let mut strength_success = Body::new();
        strength_success.set("이름", "힘기념일성공회귀");
        strength_success.set("힘", 100_i64);
        strength_success.set("최고체력", 10_i64);
        run_zone_event(
            &mut strength_success,
            "낙양성",
            "지아_대화_대_힘백.rhai",
            None,
        );
        assert_eq!(strength_success.get_int("최고체력"), 1_010);
        assert_eq!(get_user_event(&strength_success, "힘백이벤트"), "1");

        let mut age_success = Body::new();
        age_success.set("이름", "나이기념일성공회귀");
        age_success.set("나이", 100_i64);
        run_zone_event(&mut age_success, "낙양성", "밍밍_대화_대_백세.rhai", None);
        assert_eq!(age_success.get_int("레벨"), 1_000);
        assert_eq!(get_user_event(&age_success, "백세축하이벤트"), "1");
    }

    #[test]
    fn level_and_inner_power_gates_keep_their_python_failure_output() {
        for zone in ["감숙성", "운남성", "섬서성", "귀주성", "산서성", "사천성"] {
            let mut body = Body::new();
            body.set("이름", format!("방파미달회귀{zone}"));
            body.set("레벨", 549_i64);
            let (output, destination) =
                run_zone_event(&mut body, zone, "방파관리인_입장.rhai", None);
            assert_eq!(destination, None);
            assert!(output.iter().any(|line| line.contains("능력도 안되면서")));
        }

        let mut cave = Body::new();
        cave.set("이름", "오지산미달회귀");
        cave.set("레벨", 799_i64);
        let (output, destination) =
            run_zone_event(&mut cave, "귀주성", "7_대화_대_오지산_오지.rhai", None);
        assert_eq!(destination, None);
        assert!(output.iter().any(|line| line.contains("능력은 아직 부족")));

        let mut tower = Body::new();
        tower.set("이름", "구층탑미달회귀");
        tower.set("레벨", 199_i64);
        let (output, destination) =
            run_zone_event(&mut tower, "낙양성", "구층탑_입장_입.rhai", None);
        assert_eq!(destination, None);
        assert!(output.iter().any(|line| line.contains("무형의 기운")));

        let mut celestial = Body::new();
        celestial.set("이름", "옥황상제미달회귀");
        celestial.set("최고내공", 29_999_i64);
        add_test_items(&mut celestial, "금강멸류관", 1);
        let (output, destination) = run_zone_event(
            &mut celestial,
            "낙양성",
            "곤륜선인_대화_대_옥황상제.rhai",
            None,
        );
        assert_eq!(destination, None);
        assert!(output.iter().any(|line| line.contains("내공 3만")));
    }

    #[test]
    fn skill_gated_events_keep_their_python_failure_output() {
        let cases = [
            (
                "곤륜산",
                "고대유적_조사.rhai",
                "아무 일도 일어나지 않습니다",
            ),
            (
                "대설산",
                "고대유적_조사.rhai",
                "아무 일도 일어나지 않습니다",
            ),
            ("백두산", "3500_대화_대.rhai", "규화보전을 탐하여"),
            ("백두산", "3400_대_대화.rhai", "무공을 겨루고 싶다면"),
        ];
        for (zone, script, expected) in cases {
            let mut body = Body::new();
            body.set("이름", format!("무공관문회귀{zone}{script}"));
            let (output, destination) = run_zone_event(&mut body, zone, script, None);
            assert_eq!(destination, None);
            assert!(
                output.iter().any(|line| line.contains(expected)),
                "{zone}/{script}: {output:?}"
            );
        }
    }

    #[test]
    fn corpse_gate_keeps_live_combat_and_corpse_reward_paths_separate() {
        let room = format!("시체분기회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("낙양성", "23", &room);
        let mut body = Body::new();
        body.set("이름", "시체분기회귀");

        super::try_mob_event(&mut body, "낙양성", &room, "황소 꼬리 잘라")
            .expect("live tail event");
        assert_eq!(body.act, crate::player::ActState::Fight);
        assert!(!body_has_item_spec(&body, "소털"));

        crate::script::combat_commands::remove_combat_target_instance_id(&mut body, instance_id);
        body.act = crate::player::ActState::Stand;
        mark_event_mob_corpse("낙양성", &room, instance_id);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "황소 꼬리 잘라")
                .expect("corpse tail event")
        else {
            panic!("corpse tail was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("꼬리를 자릅니다")));
        assert!(body_has_item_spec(&body, "소털"));
        let world = crate::world::get_world_state().read().unwrap();
        let mob = world
            .mob_cache
            .get_all_mobs_in_room("낙양성", &room)
            .into_iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert_eq!(mob.act, 3);
        drop(world);

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn legacy_level_gate_blocks_only_below_the_python_threshold() {
        let mut low = Body::new();
        low.set("이름", "레벨관문미달회귀");
        low.set("레벨", 199_i64);
        let (output, destination) = run_zone_event(&mut low, "낙양성", "구층탑_입장_입.rhai", None);
        assert!(output.iter().any(|line| line.contains("무형의 기운")));
        assert_eq!(destination, None);

        let mut accepted = Body::new();
        accepted.set("이름", "레벨관문통과회귀");
        accepted.set("레벨", 200_i64);
        let (_, destination) = run_zone_event(&mut accepted, "낙양성", "구층탑_입장_입.rhai", None);
        assert_eq!(destination, Some(("구층탑".into(), "1".into())));
    }

    #[test]
    fn legacy_skill_directives_keep_all_skill_gates_teaching_and_removal() {
        let mut qualified = Body::new();
        qualified.set("이름", "무공동작회귀");
        qualified.skill_list = vec!["육맥신법".into(), "육맥신공".into()];
        run_zone_event(&mut qualified, "무림맹", "대학사_육맥신검.rhai", None);
        assert!(qualified.skill_list.iter().any(|skill| skill == "육맥신검"));

        let mut missing = Body::new();
        missing.set("이름", "무공동작미달회귀");
        missing.skill_list = vec!["육맥신법".into()];
        run_zone_event(&mut missing, "무림맹", "대학사_육맥신검.rhai", None);
        assert!(!missing.skill_list.iter().any(|skill| skill == "육맥신검"));

        let mut removable = Body::new();
        removable.set("이름", "무공회수회귀");
        removable.skill_list.push("가상무공".into());
        let mut data = RawMobData::new();
        data.zone = "낙양성".into();
        let words = vec!["광무".into(), "가상무공".into(), "무공삭제".into()];
        let _ = do_event_rhai(
            &mut removable,
            &data,
            "test",
            &words,
            "test",
            "기타맨_무공삭제_무공제거_무공지움.rhai",
            None,
        );
        assert!(!get_user_event(&removable, "무공삭제").is_empty());
        let _ = do_event_rhai(
            &mut removable,
            &data,
            "test",
            &words,
            "test",
            "기타맨_무공삭제_무공제거_무공지움.rhai",
            None,
        );
        assert!(!removable.skill_list.iter().any(|skill| skill == "가상무공"));
    }

    #[test]
    fn event_skill_removal_keeps_python_first_occurrence_and_training_record() {
        let mut body = Body::new();
        body.skill_list = vec!["시험무공".into(), "시험무공".into(), "다른무공".into()];
        body.skill_map
            .insert("시험무공".into(), crate::player::SkillTraining::new(7, 321));
        run_zone_event_source(
            &mut body,
            "낙양성",
            r#"
fn event() {
    remove_skill("시험무공");
    end_event();
}
"#,
            None,
        );
        assert_eq!(body.skill_list, vec!["시험무공", "다른무공"]);
        assert_eq!(
            body.skill_map.get("시험무공"),
            Some(&crate::player::SkillTraining::new(7, 321))
        );
        run_zone_event_source(
            &mut body,
            "낙양성",
            r#"
fn event() {
    remove_skill("시험무공");
    teach_skill("시험무공");
    end_event();
}
"#,
            None,
        );
        assert_eq!(body.skill_list, vec!["다른무공", "시험무공"]);
        assert_eq!(
            body.skill_map.get("시험무공"),
            Some(&crate::player::SkillTraining::new(7, 321))
        );
    }

    #[test]
    fn vision_training_event_restores_python_allowlist_prerequisites_and_skill_consumption() {
        let room = format!("비전수련회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "무공맨", &room);

        let mut unsupported = Body::new();
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut unsupported, "낙양성", &room, "비전노인 없는비전 수련")
                .expect("vision trainer selection")
        else {
            panic!("unsupported vision must remain in event output");
        };
        assert!(output_lines.iter().any(|line| line.contains("그러한 비전")));

        let mut missing = Body::new();
        let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event(
            &mut missing,
            "낙양성",
            &room,
            "비전노인 강룡십팔장비전 수련",
        )
        .expect("missing prerequisite event") else {
            panic!("missing prerequisites must remain in event output");
        };
        assert!(output_lines.iter().any(|line| line.contains("장안기공")));
        assert!(missing.get_string("비전수련").is_empty());

        // `$비전종류확인` is evaluated after the active-training guard and
        // rejects an already learned exact entry without consuming its
        // prerequisite ordinary skills.
        let mut learned = Body::new();
        learned.add_secret_skill("강룡십팔장비전");
        learned.skill_list = vec![
            "분근착골수".into(),
            "장안기공".into(),
            "사량발천근".into(),
            "금나수".into(),
        ];
        let CommandResult::MobEvent { output_lines, .. } = super::try_mob_event(
            &mut learned,
            "낙양성",
            &room,
            "비전노인 강룡십팔장비전 수련",
        )
        .expect("learned vision event") else {
            panic!("learned vision must remain in event output");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("벌써 그 비전을 수련")));
        assert!(learned.get_string("비전수련").is_empty());
        assert_eq!(learned.skill_list.len(), 4);

        let mut qualified = Body::new();
        qualified.skill_list = vec![
            "분근착골수".into(),
            "장안기공".into(),
            "사량발천근".into(),
            "금나수".into(),
        ];
        super::try_mob_event(
            &mut qualified,
            "낙양성",
            &room,
            "비전노인 강룡십팔장비전 수련",
        )
        .expect("qualified vision event");
        assert_eq!(qualified.get_string("비전수련"), "강룡십팔장비전");
        assert!(qualified.skill_list.is_empty());

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut qualified, "낙양성", &room, "비전노인 무극검비전 수련")
                .expect("existing training event")
        else {
            panic!("existing vision training must remain in event output");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("이미 수련중인 비전")));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn every_vision_trainer_prerequisite_list_is_consumed_on_its_success_path() {
        // Execute all eight authored `$무공리스트확인!`/`$무공리스트삭제`
        // pairs, not only the 강룡십팔장 example.  Python consumes each
        // prerequisite exactly when its corresponding vision is accepted.
        let room = format!("비전전수전체회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "무공맨", &room);
        let cases: [(&str, &[&str]); 8] = [
            (
                "강룡십팔장비전",
                &["분근착골수", "장안기공", "사량발천근", "금나수"],
            ),
            ("무극검비전", &["대력금나수"]),
            ("천마검비전", &["투골타혈법", "전암전회"]),
            ("뇌음자흑강비전", &["이화접목", "차기미기"]),
            ("대비단혼강비전", &["전이대법", "격체전공"]),
            ("천마무격신장비전", &["건곤대나이", "흡성대법"]),
            ("혈세천하비전", &["공수탈백인"]),
            ("멸천혈폭비전", &["음양귀혼"]),
        ];
        for (vision, prerequisites) in cases {
            let mut body = Body::new();
            body.set("이름", format!("비전전수{vision}회귀"));
            body.skill_list = prerequisites
                .iter()
                .map(|skill| (*skill).to_string())
                .collect();
            let command = format!("비전노인 {vision} 수련");
            let result = super::try_mob_event(&mut body, "낙양성", &room, &command)
                .expect("vision trainer event");
            assert!(
                matches!(result, CommandResult::MobEvent { .. }),
                "{vision}: expected immediate authored completion, got {result:?}"
            );
            assert_eq!(body.get_string("비전수련"), vision, "{vision}");
            assert!(
                body.skill_list.is_empty(),
                "{vision}: {:#?}",
                body.skill_list
            );
        }
        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn jade_emperor_vision_prerequisites_skip_only_already_learned_blocks() {
        // Python `$비전종류확인!` skips the following block when its vision
        // is already learned.  The Jade Emperor's eight checks therefore
        // report the first *missing* vision, not the first learned one.
        let scripts = [
            "옥황상제_대화_대_고영신공.rhai",
            "옥황상제_대화_대_역근경.rhai",
            "옥황상제_대화_대_태극강기.rhai",
            "옥황상제_대화_대_북명신공.rhai",
            "옥황상제_대화_대_천외비선.rhai",
            "옥황상제_대화_대_가의신공.rhai",
            "옥황상제_대화_대_명옥공.rhai",
        ];
        for script in scripts {
            let source = std::fs::read_to_string(format!("data/script/선인/{script}"))
                .expect("Jade Emperor vision script");
            assert!(source.contains("if !has_vision(vision)"), "{script}");
        }

        let mut no_visions = Body::new();
        let (first_missing, _) = run_zone_event(
            &mut no_visions,
            "선인",
            "옥황상제_대화_대_역근경.rhai",
            None,
        );
        assert!(first_missing
            .iter()
            .any(|line| line.contains("멸천혈폭비전 먼저 배우시게")));

        let mut first_vision_learned = Body::new();
        first_vision_learned.add_secret_skill("멸천혈폭비전");
        let (next_missing, _) = run_zone_event(
            &mut first_vision_learned,
            "선인",
            "옥황상제_대화_대_역근경.rhai",
            None,
        );
        assert!(!next_missing
            .iter()
            .any(|line| line.contains("멸천혈폭비전 먼저 배우시게")));
        assert!(next_missing
            .iter()
            .any(|line| line.contains("혈세천하비전 먼저 배우시게")));
    }

    #[test]
    fn police_congratulations_keep_python_level_gate_buff_and_one_time_rewards() {
        let room = format!("포졸축하회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "포졸", &room);

        let mut high_level = Body::new();
        high_level.set("이름", "포졸고레벨회귀");
        high_level.set("레벨", 300_i64);
        let CommandResult::MobEvent {
            output_lines: high_output,
            ..
        } = super::try_mob_event(&mut high_level, "낙양성", &room, "포졸 축하")
            .expect("police congratulations event")
        else {
            panic!("police congratulations returned a non-event result");
        };
        assert!(high_output.iter().any(|line| line.contains("300레벨 이상")));
        assert!(high_level.active_skills.is_empty());
        assert!(get_user_event(&high_level, "리뉴얼").is_empty());
        for item in ["진열장", "합성11", "무림지도1", "무림지도2"] {
            assert!(
                !body_has_item_spec(&high_level, item),
                "high level received {item}"
            );
        }

        let mut low_level = Body::new();
        low_level.set("이름", "포졸저레벨회귀");
        low_level.set("레벨", 299_i64);
        let CommandResult::MobEvent {
            output_lines: low_output,
            ..
        } = super::try_mob_event(&mut low_level, "낙양성", &room, "포졸 축하")
            .expect("police congratulations event")
        else {
            panic!("police congratulations returned a non-event result");
        };
        assert!(low_output
            .iter()
            .any(|line| line.contains("포졸") && line.contains("당신에게 힘을")));
        assert!(low_output.iter().any(|line| line.contains("여러가지 선물")));
        assert!(!get_user_event(&low_level, "리뉴얼").is_empty());
        assert_eq!(low_level._str, 100);
        assert_eq!(low_level._dex, 100);
        assert_eq!(low_level._arm, 100);
        assert_eq!(low_level.active_skills.len(), 1);
        assert_eq!(low_level.active_skills[0].name, "이벤트");
        assert_eq!(low_level.active_skills[0].start_time, 999);
        for item in ["진열장", "합성11", "무림지도1", "무림지도2"] {
            assert!(
                body_has_item_spec(&low_level, item),
                "low level missed {item}"
            );
        }

        let CommandResult::MobEvent {
            output_lines: retry_output,
            ..
        } = super::try_mob_event(&mut low_level, "낙양성", &room, "포졸 축하")
            .expect("repeat police congratulations event")
        else {
            panic!("repeat police congratulations returned a non-event result");
        };
        assert!(!retry_output
            .iter()
            .any(|line| line.contains("여러가지 선물")));
        assert_eq!(low_level.active_skills.len(), 1);

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        drop(world);
        for name in ["포졸고레벨회귀", "포졸저레벨회귀"] {
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }
    }

    #[test]
    fn blacksmith_legacy_event_arrays_are_rhai_with_python_olsuk_and_script_handoff() {
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string("data/mob/낙양성/합체맨.json").unwrap())
                .unwrap();
        let info = &json["몹정보"];
        for key in [
            "이벤트: $대화 $대 무기강화",
            "이벤트: $대화 $대 올숙",
            "이벤트: $대화 $대 올숙이천",
        ] {
            assert!(
                info[key].is_string(),
                "{key} must no longer use legacy array events"
            );
        }

        let room = format!("합체맨올숙회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "합체맨", &room);
        let mut body = Body::new();
        body.set("이름", "합체맨올숙회귀");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "대장장이 무기강화 대화")
                .expect("blacksmith weapon upgrade event")
        else {
            panic!("incomplete olsuk weapon must stay in event output");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("올숙무기가 없지")));

        body.set("올숙완료", 1_i64);
        let mut weapon = crate::object::Object::new();
        weapon.set("인덱스", "합체맨올숙회귀_올숙무기");
        weapon.set("공격력", 1_999_i64);
        let weapon = std::sync::Arc::new(std::sync::Mutex::new(weapon));
        body.object.objs.push(weapon.clone());
        let CommandResult::StartScript {
            script_name,
            use_rhai,
            ..
        } = super::try_mob_event(&mut body, "낙양성", &room, "대장장이 무기강화 대화")
            .expect("sub-2000 weapon must skip the material guard")
        else {
            panic!("sub-2000 weapon must hand off without 강철판");
        };
        assert_eq!(script_name, "무기강화");
        assert!(use_rhai);

        weapon.lock().unwrap().set("공격력", 2_000_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "대장장이 무기강화 대화")
                .expect("high attack weapon must enter the material branch")
        else {
            panic!("high attack weapon without plates must stay in event output");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("강화재료가 부족하지")));

        add_test_items(&mut body, "강철판", 5);
        let CommandResult::StartScript {
            script_name,
            use_rhai,
            ..
        } = super::try_mob_event(&mut body, "낙양성", &room, "대장장이 무기강화 대화")
            .expect("qualified blacksmith upgrade event")
        else {
            panic!("qualified olsuk weapon must hand off to the upgrade script");
        };
        assert_eq!(script_name, "무기강화");
        assert!(use_rhai);

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "대장장이 올숙 대화")
                .expect("blacksmith olsuk event")
        else {
            panic!("missing pills must stay in event output");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("고농축 주과 10개")));

        let mut candidate = Body::new();
        candidate.set("이름", "합체맨올숙자격회귀");
        add_test_items(&mut candidate, "합성6-2", 10);
        for weapon_type in 1..=5 {
            candidate.set(&format!("{weapon_type} 숙련도"), 1_000_i64);
        }
        let CommandResult::StartScript { script_name, .. } =
            super::try_mob_event(&mut candidate, "낙양성", &room, "대장장이 올숙 대화")
                .expect("qualified olsuk event")
        else {
            panic!("qualified candidate must enter the original question script");
        };
        assert_eq!(script_name, "올숙천");
        candidate.set("올숙완료", 1_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut candidate, "낙양성", &room, "대장장이 올숙 대화")
                .expect("completed olsuk event")
        else {
            panic!("completed candidate must remain in dialogue output");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("이미 올숙무기를 지급")));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/합체맨올숙회귀.json");
    }

    #[test]
    fn blood_tower_cremation_requires_a_corpse_and_immediately_respawns_it() {
        let room = format!("혈탑삼매진화회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("혈탑", "무림인07", &room);
        let mut body = Body::new();
        body.set("이름", "혈탑삼매진화회귀");

        // Python `$몹상태확인 시체` suppresses the event for a living mob.
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "혈탑", &room, "혈탑살수 태워")
                .expect("blood tower cremation event")
        else {
            panic!("living blood tower killer returned a non-event result");
        };
        assert!(output_lines.is_empty());

        mark_event_mob_corpse("혈탑", &room, instance_id);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "혈탑", &room, "시체 태워")
                .expect("corpse cremation event")
        else {
            panic!("corpse cremation returned a non-event result");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("삼매진화의 진기로 태웁니다")));
        {
            let world = crate::world::get_world_state().read().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room("혈탑", &room)
                .into_iter()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            assert!(mob.alive);
            assert_eq!(mob.act, 0);
            assert_eq!(mob.hp, mob.max_hp);
        }

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn iron_bell_rank_view_resolves_python_rank_placeholders_in_rhai() {
        let _rank_guard = RANK_TEST_LOCK.lock().unwrap();
        crate::world::rank::rank_clear("힘");
        let room = format!("쇠종순위회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "1-1", &room);
        let mut body = Body::new();
        body.set("이름", "쇠종순위회귀");
        assert_eq!(
            crate::world::rank::rank_write("힘", "쇠종순위회귀", 999_999, 1),
            1
        );

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "쇠종 봐")
                .expect("iron bell rank view event")
        else {
            panic!("iron bell rank view returned a non-event result");
        };
        let output = output_lines.join("\n");
        assert!(output.contains("쇠종순위회귀"));
        assert!(output.contains("순위ː\x1b[1m1"));
        assert!(!output.contains("[순위자]"));
        assert!(!output.contains("[순위]"));

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "쇠종 1 봐")
                .expect("numbered iron bell rank view event")
        else {
            panic!("numbered iron bell view returned a non-event result");
        };
        assert!(output_lines.join("\n").contains("쇠종순위회귀"));

        // Python getInt("1번") returns 1.  `$순위확인` must therefore use
        // the numeric rank branch rather than look for a player named 1번.
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "쇠종 1번 봐")
                .expect("prefixed-number iron bell rank view event")
        else {
            panic!("prefixed-number iron bell view returned a non-event result");
        };
        assert!(output_lines.join("\n").contains("쇠종순위회귀"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        crate::world::rank::rank_clear("힘");
    }

    #[test]
    fn rank_event_integer_parser_keeps_python_prefix_digits_for_handwritten_rhai() {
        let _rank_guard = RANK_TEST_LOCK.lock().unwrap();
        crate::world::rank::rank_clear("소오강호");
        assert_eq!(
            crate::world::rank::rank_write("소오강호", "접두순위회귀", 999_999, 300),
            1
        );
        let mut body = Body::new();
        body.set("이름", "순위정수회귀");
        let source = std::fs::read_to_string("data/script/구층탑/비석_보_봐_보아_보다_본다.rhai")
            .expect("source rank-stone script");
        let (output, _) = run_zone_event_source_with_words(
            &mut body,
            "구층탑",
            &source,
            &["비석".into(), "1번".into(), "봐".into()],
            None,
        );
        assert!(output.join("\n").contains("접두순위회귀"));

        // Python getInt("0번") is zero, so it remains a literal target name.
        crate::world::rank::rank_write("소오강호", "0번", 1, 300);
        let (output, _) = run_zone_event_source_with_words(
            &mut body,
            "구층탑",
            &source,
            &["비석".into(), "0번".into(), "봐".into()],
            None,
        );
        assert!(output.join("\n").contains("0번"));
        crate::world::rank::rank_clear("소오강호");
    }

    #[test]
    fn wedding_bell_rank_view_resolves_python_rank_placeholders_in_rhai() {
        crate::world::rank::rank_clear("결혼");
        let room = format!("결혼쇠종순위회귀-{}", std::process::id());
        // The marriage-ranking bell shown to players is named/referred to as
        // `쇠종`; `폭폭` is a separate malformed fixture named `폭축`.
        let (mob_key, _) = place_event_mob("낙양성", "가짜쇠종", &room);
        let mut body = Body::new();
        body.set("이름", "결혼쇠종순위회귀");
        body.set("결혼", 999_999_i64);
        assert_eq!(
            crate::world::rank::rank_write("결혼", "결혼쇠종순위회귀", 999_999, 1),
            1
        );

        super::try_mob_event(&mut body, "낙양성", &room, "쇠종 쳐")
            .expect("wedding bell rank record event");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "쇠종 봐")
                .expect("wedding bell rank view event")
        else {
            panic!("wedding bell rank view returned a non-event result");
        };
        let output = output_lines.join("\n");
        assert!(output.contains("결혼쇠종순위회귀"));
        assert!(output.contains("순위ː\x1b[1m1"), "{output:?}");
        assert!(!output.contains("[순위자]"));
        assert!(!output.contains("[순위]"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        crate::world::rank::rank_clear("결혼");
    }

    #[test]
    fn legacy_rank_record_broadcasts_only_when_the_player_becomes_first() {
        let _rank_guard = RANK_TEST_LOCK.lock().unwrap();
        crate::world::rank::rank_clear("힘");
        let room = format!("쇠종기록회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "1-1", &room);
        let mut body = Body::new();
        body.set("이름", "쇠종기록회귀");
        body.set("힘", 1_234_i64);

        let CommandResult::MobEvent {
            output_lines,
            broadcast_lines,
            ..
        } = super::try_mob_event(&mut body, "낙양성", &room, "쇠종 쳐")
            .expect("iron bell rank record event")
        else {
            panic!("iron bell rank record was not an event");
        };
        assert!(output_lines
            .iter()
            .any(|line| line.contains("울려퍼집니다")));
        assert!(broadcast_lines
            .iter()
            .any(|line| line.contains("쇠종기록회귀") && line.contains("울려퍼집니다")));

        let CommandResult::MobEvent {
            broadcast_lines, ..
        } = super::try_mob_event(&mut body, "낙양성", &room, "쇠종 쳐")
            .expect("repeat iron bell rank record event")
        else {
            panic!("repeat iron bell rank record was not an event");
        };
        assert!(broadcast_lines.is_empty());

        // `$순위기록` admits every placement inside its limit.  Only a
        // placement outside that limit skips the braced success text and
        // reaches the source's fallback line.
        crate::world::rank::rank_clear("힘");
        for rank in 0..200 {
            crate::world::rank::rank_write("힘", &format!("쇠종상위{rank}"), 10_000 - rank, 200);
        }
        let mut outside = Body::new();
        outside.set("이름", "쇠종순위밖회귀");
        outside.set("힘", 1_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut outside, "낙양성", &room, "쇠종 쳐")
                .expect("outside-limit iron bell event")
        else {
            panic!("outside-limit iron bell was not an event");
        };
        assert!(output_lines.iter().any(|line| line.contains("듣기싫은")));
        assert!(!output_lines.iter().any(|line| line.contains("웅장한")));
        assert_eq!(crate::world::rank::rank_read("힘", "쇠종순위밖회귀"), 0);

        crate::world::rank::rank_clear("힘");
        crate::world::rank::rank_write("힘", "쇠종상위한명", 10_000, 200);
        let mut admitted = Body::new();
        admitted.set("이름", "쇠종순위권회귀");
        admitted.set("힘", 1_i64);
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut admitted, "낙양성", &room, "쇠종 쳐")
                .expect("admitted non-first iron bell event")
        else {
            panic!("admitted non-first iron bell was not an event");
        };
        assert!(output_lines.iter().any(|line| line.contains("웅장한")));
        assert!(!output_lines.iter().any(|line| line.contains("듣기싫은")));
        assert_eq!(crate::world::rank::rank_read("힘", "쇠종순위권회귀"), 2);

        crate::world::rank::rank_clear("최고내공");
        for rank in 0..200 {
            crate::world::rank::rank_write(
                "최고내공",
                &format!("창호지상위{rank}"),
                10_000 - rank,
                200,
            );
        }
        let mut paper = Body::new();
        paper.set("이름", "창호지순위밖회귀");
        paper.set("최고내공", 1_i64);
        paper.set("체력", 200_i64);
        let (output, _) = run_luoyang_event(&mut paper, "1-2_건너_건_걸어_걸.rhai");
        assert!(output.iter().any(|line| line.contains("창호지가 찢어지며")));
        assert!(!output.iter().any(|line| line.contains("가벼운 몸놀림")));
        assert_eq!(paper.get_int("체력"), 100);

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        crate::world::rank::rank_clear("힘");
        crate::world::rank::rank_clear("최고내공");
    }

    #[test]
    fn every_remaining_rank_board_uses_rhai_templates_for_success_missing_and_all() {
        let rank_scripts = [
            "data/script/낙양성/1-2_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/가쇠종_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/가짜쇠종_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/금강동인_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/무황성전_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/민첩_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/볏짚_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/석판_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/전직비석_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/청강석_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/통나무_보_봐_보아_보다_본다.rhai",
            "data/script/낙양성/폭폭_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석50-1_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석50_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석70-1_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석70_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석90-1_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석90_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석백-1_보_봐_보아_보다_본다.rhai",
            "data/script/백층탑/비석백_보_봐_보아_보다_본다.rhai",
            "data/script/전직/비석_보_봐_보아_보다_본다.rhai",
        ];
        for path in rank_scripts {
            let source = std::fs::read_to_string(path).unwrap();
            assert!(source.contains("rank_render("), "{path}");
            assert!(!source.contains("legacy: $순위확인"), "{path}");
        }

        crate::world::rank::rank_clear("2 숙련도");
        let room = format!("통나무순위회귀-{}", std::process::id());
        let (mob_key, _) = place_event_mob("낙양성", "통나무", &room);
        let mut body = Body::new();
        body.set("이름", "통나무순위회귀");
        crate::world::rank::rank_write("2 숙련도", "통나무순위회귀", 999_999, 1);

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "통나무 봐")
                .expect("wooden target rank view")
        else {
            panic!("wooden target rank view returned a non-event result");
        };
        assert!(output_lines.join("\n").contains("통나무순위회귀"));

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "통나무 없는사람 봐")
                .expect("missing wooden target rank view")
        else {
            panic!("missing wooden target returned a non-event result");
        };
        assert!(output_lines.join("\n").contains("흔적이 보이지 않네요"));

        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut body, "낙양성", &room, "통나무 모두 봐")
                .expect("all wooden target rank view")
        else {
            panic!("all wooden target returned a non-event result");
        };
        assert!(output_lines.join("\n").contains("통나무순위회귀"));

        let mut world = crate::world::get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        crate::world::rank::rank_clear("2 숙련도");
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

        // `$몹상태확인 시체` fails while 석융빈 is alive, so Python falls
        // through to `$전투시작`.  Keep that non-corpse branch distinct from
        // the later corpse/reward path below.
        let mut alive_insert = Body::new();
        alive_insert.set("이름", "백우선생존삽입회귀");
        super::set_user_event(&mut alive_insert, "가백우", "1");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut alive_insert, "하북성", &room, "석융빈 백우선 꽂아")
                .expect("living zombie insert event must be selected")
        else {
            panic!("living zombie insert returned non-event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("멍하니")));
        assert_eq!(alive_insert.act, crate::player::ActState::Fight);
        assert_eq!(
            crate::script::combat_commands::combat_target_instance_ids(&alive_insert),
            vec![zombie_id]
        );

        // The prior live-target branch is a separate player.  End that
        // simulated fight before asking the main fixture to challenge the
        // same mob, just as an actual combat death/escape would.
        crate::script::combat_commands::remove_combat_target_instance_id(
            &mut alive_insert,
            zombie_id,
        );
        alive_insert.act = crate::player::ActState::Stand;
        crate::script::combat_commands::remove_combat_target_instance_id(&mut body, museong_id);
        body.act = crate::player::ActState::Stand;
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            let mob = world
                .mob_cache
                .get_all_mobs_in_room_mut("하북성", &room)
                .unwrap()
                .iter_mut()
                .find(|mob| mob.instance_id == zombie_id)
                .unwrap();
            mob.act = 0;
            mob.targets.clear();
        }

        super::set_user_event(&mut body, "혈유비", "1");
        add_test_items(&mut body, "340", 1);
        super::try_mob_event(&mut body, "하북성", &room, "석융빈 쳐")
            .expect("zombie challenge must be selected");
        assert_eq!(body.act, crate::player::ActState::Fight);
        assert_eq!(
            crate::script::combat_commands::combat_target_instance_ids(&body),
            vec![zombie_id]
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
        crate::script::combat_commands::remove_combat_target_instance_id(&mut body, zombie_id);
        body.act = crate::player::ActState::Stand;

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
            drop(world);
            crate::script::combat_commands::remove_combat_target_instance_id(
                &mut body,
                instance_id,
            );
            body.act = crate::player::ActState::Stand;
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
        crate::script::combat_commands::remove_combat_target_instance_id(&mut body, abbess_id);
        body.act = crate::player::ActState::Stand;
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
            crate::script::combat_commands::remove_combat_target_instance_id(
                &mut body,
                instance_id,
            );
            body.act = crate::player::ActState::Stand;

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
        body.set("맷집", 91_i64);
        body.set("전직", 1_i64);
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
        assert!(!get_user_event(&body, "소오강호끝").is_empty());
        assert!(!get_user_event(&body, "소오강호진짜끝").is_empty());
        for retired in [
            "동정호진짜끝",
            "제일신룡단",
            "과거",
            "과거1",
            "과거2",
            "과거끝",
        ] {
            assert!(
                get_user_event(&body, retired).is_empty(),
                "$소오강호설정 must replace stale event {retired}"
            );
        }
        assert_eq!(body.get_int("힘"), 100);
        assert_eq!(body.get_int("맷집"), 60);
        assert!(
            matches!(body.get("맷집"), Value::Float(value) if (value - (182.0 / 3.0)).abs() < f64::EPSILON)
        );
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
    fn yuk_jahong_unique_gate_uses_the_python_item_index_not_its_display_name() {
        // `objs/event.py:$기연확인!` calls checkOneItemIndex().  황룡마조
        // currently happens to have the same display name and index, but the
        // Rhai conversion must retain the index contract rather than relying
        // on that data coincidence.
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();

        let source = std::fs::read_to_string("data/script/강소성/육자홍_절.rhai")
            .expect("Yuk Jahong event script");
        assert!(source.contains("!one_item_exists(\"황룡마조\")"));
        assert!(!source.contains("one_item_exists_name(\"황룡마조\")"));

        let prepare = |name: &str| {
            let mut body = Body::new();
            body.set("이름", name);
            super::set_user_event(&mut body, "진마혁끝", "1");
            super::set_user_event(&mut body, "황룡마조1", "1");
            body
        };

        let mut unclaimed = prepare("육자홍원본인덱스미소유회귀");
        run_zone_event(&mut unclaimed, "강소성", "육자홍_절.rhai", None);
        assert!(body_has_item_spec(&unclaimed, "황룡마조"));
        assert!(!body_has_item_spec(&unclaimed, "황룡마조-5"));

        crate::oneitem::oneitem_clear();
        assert!(crate::oneitem::oneitem_have("황룡마조", "이미소유한사람"));
        let mut claimed = prepare("육자홍원본인덱스소유회귀");
        run_zone_event(&mut claimed, "강소성", "육자홍_절.rhai", None);
        assert!(!body_has_item_spec(&claimed, "황룡마조"));
        assert!(body_has_item_spec(&claimed, "황룡마조-5"));

        crate::oneitem::oneitem_clear();
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).expect("restore unique-item registry");
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
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
    fn blood_mist_valley_guard_starts_unmasked_fight_or_escorts_masked_to_final_room() {
        let mut rooms = RoomCache::new();
        let final_gate = rooms.get_room("산서성", "1189").unwrap();
        let final_gate = final_gate.read().unwrap();
        assert_eq!(final_gate.display_name, "오대산 혈무별원");
        assert!(final_gate.exits.is_empty());
        assert_eq!(final_gate.mob_ids, vec!["66"]);
        drop(final_gate);

        let room = format!("혈무곡회귀-{}", std::process::id());
        let (mob_key, instance_id) = place_event_mob("산서성", "66", &room);
        let mut unmasked = Body::new();
        unmasked.set("이름", "혈무곡무가면회귀");
        super::try_mob_event(&mut unmasked, "산서성", &room, "혈광호위 대화")
            .expect("unmasked guard dialogue must be selected");
        assert!(
            crate::script::combat_commands::combat_target_instance_ids(&unmasked)
                .contains(&instance_id)
        );

        let mut masked = Body::new();
        masked.set("이름", "혈무곡가면회귀");
        add_test_items(&mut masked, "인피면구", 1);
        let CommandResult::MobEvent { set_position, .. } =
            super::try_mob_event(&mut masked, "산서성", &room, "혈광호위 대화")
                .expect("masked guard dialogue must be selected")
        else {
            panic!("guard dialogue must complete immediately");
        };
        assert_eq!(set_position, Some(("산서성".into(), "1190".into())));

        crate::world::get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .remove_mob(&mob_key);
        let _ = std::fs::remove_file("data/user/혈무곡무가면회귀.json");
        let _ = std::fs::remove_file("data/user/혈무곡가면회귀.json");
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
    fn unique_owner_placeholder_uses_python_bare_owner_token() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();
        assert!(crate::oneitem::oneitem_have("163", "먼저온사람"));

        let mut body = Body::new();
        body.set("이름", "기연소지자회귀");
        let (output, _) = run_zone_event(&mut body, "동정호", "요마_절_구배_삼배.rhai", None);
        assert!(
            output.iter().any(|line| line.contains("먼저온사람 보다")),
            "$기연확인! must substitute Python's bare [기연소지자] token: {output:?}"
        );
        assert!(
            output
                .iter()
                .all(|line| !line.contains("[기연소지자]") && !line.contains("먼저온사람이 보다")),
            "$기연확인! must not add a particle to the owner token: {output:?}"
        );

        for (path, index) in [
            ("data/script/동정호/요마_절_구배_삼배.rhai", "163"),
            ("data/script/동정호/무황_절_구배_삼배.rhai", "348"),
            ("data/script/호북성/36_절_구배_삼배.rhai", "71"),
            ("data/script/낙양성/88_부셔_부.rhai", "66"),
        ] {
            let source = std::fs::read_to_string(path).unwrap();
            assert!(
                !source.contains("[기연소지자]"),
                "unconverted owner token: {path}"
            );
            assert!(
                source.contains(&format!("one_item_owner_raw(\"{index}\")")),
                "Python bare-owner helper missing from {path}"
            );
        }

        let _ = std::fs::remove_file("data/user/기연소지자회귀.json");
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn bulhong_valley_unique_owner_before_cheoreom_stays_bare() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();
        assert!(crate::oneitem::oneitem_have("해왕조", "먼저온사람"));

        let mut body = Body::new();
        body.set("이름", "불혼곡소유자회귀");
        let source = std::fs::read_to_string("data/script/산서성/46_대화_대.rhai").unwrap();
        let (output, _) = run_zone_event_source(&mut body, "산서성", &source, None);
        assert!(
            output.iter().any(|line| line.contains("먼저온사람처럼")),
            "Python's `[기연소지자]처럼` must keep the owner bare: {output:?}"
        );
        assert!(
            output.iter().all(|line| !line.contains("먼저온사람이처럼")),
            "the owner must not gain an extra subject particle: {output:?}"
        );

        let _ = std::fs::remove_file("data/user/불혼곡소유자회귀.json");
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn bulhong_valley_answer_applies_the_python_owner_subject_particle() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();
        assert!(crate::oneitem::oneitem_have("해왕조", "먼저온사람"));

        let mut body = Body::new();
        body.set("이름", "불혼곡정답회귀");
        super::set_user_event(&mut body, "불혼곡", "1");
        let source = std::fs::read_to_string("data/script/산서성/46_답.rhai").unwrap();
        let (output, _) = run_zone_event_source_with_words(
            &mut body,
            "산서성",
            &source,
            &["불혼곡주".into(), "242".into(), "답".into()],
            None,
        );
        assert!(
            output
                .iter()
                .any(|line| line.contains("먼저온사람이 먼저 왔었다네")),
            "Python postPosition1 must resolve `[기연소지자](이/가)`: {output:?}"
        );
        assert!(
            output.iter().all(|line| !line.contains("(이/가)")),
            "Rhai output must not leave Python's particle marker literal: {output:?}"
        );
        assert!(body_has_item_spec(&body, "해왕조-5"));
        assert!(get_user_event(&body, "불혼곡").is_empty());
        assert!(!get_user_event(&body, "불혼곡끝").is_empty());

        let _ = std::fs::remove_file("data/user/불혼곡정답회귀.json");
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn unique_owner_name_lookup_keeps_python_bare_owner_before_ege() {
        let oneitem_path = std::path::Path::new("data/config/oneitem.json");
        let saved_oneitem_file = std::fs::read(oneitem_path).ok();
        crate::oneitem::oneitem_clear();
        assert!(crate::oneitem::oneitem_have("77", "먼저온사람"));

        let mut body = Body::new();
        body.set("이름", "기연이름소지자회귀");
        body.set("은전", 30_000_000_i64);
        let source =
            std::fs::read_to_string("data/script/낙양성/기연맨_위치확인_위치_확인.rhai").unwrap();
        let (output, _) = run_zone_event_source_with_words(
            &mut body,
            "낙양성",
            &source,
            &["기연맨".into(), "간장검".into(), "위치확인".into()],
            None,
        );
        assert!(
            output.iter().any(|line| line.contains("먼저온사람에게")),
            "$기연존재확인 must substitute the bare owner before `에게`: {output:?}"
        );
        assert!(
            output.iter().all(|line| !line.contains("먼저온사람이에게")),
            "$기연존재확인 must not add an extra subject particle: {output:?}"
        );
        assert_eq!(body.get_int("은전"), 0);

        crate::oneitem::oneitem_clear();
        let mut missing = Body::new();
        missing.set("이름", "기연이름미소지회귀");
        missing.set("은전", 30_000_000_i64);
        let (output, _) = run_zone_event_source_with_words(
            &mut missing,
            "낙양성",
            &source,
            &["기연맨".into(), "간장검".into(), "위치확인".into()],
            None,
        );
        assert!(
            output.iter().any(|line| line.contains("강호에 없다네")),
            "missing unique must keep the Python fallback message: {output:?}"
        );
        assert_eq!(
            missing.get_int("은전"),
            30_000_000,
            "Python only charges when `$기연존재확인` finds an owner"
        );

        let _ = std::fs::remove_file("data/user/기연이름소지자회귀.json");
        let _ = std::fs::remove_file("data/user/기연이름미소지회귀.json");
        if let Some(contents) = saved_oneitem_file {
            std::fs::write(oneitem_path, contents).unwrap();
        } else {
            let _ = std::fs::remove_file(oneitem_path);
        }
        assert!(crate::oneitem::oneitem_reload());
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
        assert!(
            !get_tendency(&body, "알수없는성향"),
            "Python getTendency() returns None/false for an unknown condition"
        );
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

        // `$몹상태확인! 시체` skips the reward block for a corpse.  Its
        // remaining source path must therefore attack a living badger rather
        // than granting hide or silently terminating the event.
        let living_room = format!("오소리생존회귀-{}", std::process::id());
        let (living_key, living_id) = place_event_mob("낙양성", "25", &living_room);
        let mut living_body = Body::new();
        living_body.set("이름", "오소리생존회귀");
        super::try_mob_event(&mut living_body, "낙양성", &living_room, "오소리 가죽 벗겨")
            .expect("living badger event must select the source mob");
        assert_eq!(living_body.act, crate::player::ActState::Fight);
        assert!(
            crate::script::combat_commands::combat_target_instance_ids(&living_body)
                .contains(&living_id)
        );
        assert!(!body_has_item_spec(&living_body, "오소리가죽"));
        crate::script::combat_commands::remove_combat_target_instance_id(
            &mut living_body,
            living_id,
        );
        crate::world::get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .remove_mob(&living_key);

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
        let _ = std::fs::remove_file("data/user/오소리생존회귀.json");
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
        let eaten = storage
            .execute("먹어", &mut body, "초혼단", None, None, None)
            .unwrap();
        assert_eq!(
            body.get_int("최고내공"),
            110,
            "eat output={:?}, stacks={:?}, objects={}",
            eaten.0,
            body.object.inv_stack,
            body.object.objs.len()
        );
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
        clear_test_oneitems(&["158"]);
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
        clear_test_oneitems(&["158"]);
        let _ = std::fs::remove_file("data/user/화산검객회귀.json");
    }

    #[test]
    fn dharma_cave_corpse_search_gives_unique_sword_and_feather_pants() {
        clear_test_oneitems(&["161"]);
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
        clear_test_oneitems(&["161"]);
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
        clear_test_oneitems(&["918"]);
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
        clear_test_oneitems(&["918"]);
        let _ = std::fs::remove_file("data/user/제석천실회귀.json");
        let _ = std::fs::remove_file("data/user/제석천실시체회귀.json");
    }

    #[test]
    fn blood_spirit_cave_altar_awards_its_unique_sword_once_per_player_event() {
        clear_test_oneitems(&["151"]);
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
        clear_test_oneitems(&["151"]);
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
    fn peach_blossom_forest_tree_live_python_and_bird_chain_match_source_transitions() {
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

        // Python `$몹상태확인! 시체` skips the reward block for a corpse;
        // the live python is eviscerated and then changed to a corpse.
        let opened = super::try_mob_event(&mut body, "호북성", &room, "대망 배째")
            .expect("live great python event must be selected");
        let CommandResult::MobEvent { output_lines, .. } = opened else {
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
            // Python `Mob.setAct("시체")` sets ACT_DEATH (2); ACT_REGEN (3)
            // is entered later by the corpse-expiry update.
            assert_eq!(python.act, 2);
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

        // 원본은 여기서 일반 진행 키가 아니라 ANSI가 포함된 별도 legacy
        // 키를 삭제한다. 둘을 합치면 윤대인3 재방문 경로가 사라진다.
        super::del_user_event(&mut body, "혼원1");
        super::set_user_event(&mut body, "토령관", "1");
        super::set_user_event(&mut body, "\x1b[33m윤대인\x1b[37;40m3", "1");
        super::set_user_event(&mut body, "윤대인3", "1");
        add_test_items(&mut body, "수령시", 1);
        let (_, destination) = run_zone_event(&mut body, "산동성", "58_대_대화.rhai", None);
        assert_eq!(destination, Some(("산동성".to_string(), "401".to_string())));
        assert!(get_user_event(&body, "\x1b[33m윤대인\x1b[37;40m3").is_empty());
        assert!(
            !get_user_event(&body, "윤대인3").is_empty(),
            "source must not delete the ordinary 윤대인3 dialogue state"
        );

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
    fn hundred_floor_tower_cremation_restores_source_immediate_respawn_after_corpse_reward() {
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
            assert!(
                source.contains("respawn_selected_mob();"),
                "{} must retain the source immediate `리젠후생성` transition",
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
        assert!(
            mob.alive,
            "Python `리젠후생성` must immediately regenerate the corpse"
        );
        assert_eq!(mob.act, 0, "source cremation must restore stand state");
        assert_eq!(mob.hp, mob.max_hp, "source cremation must restore full hp");
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
            body.object.inv_stack.clear();
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
        let CommandResult::MobEventEnter {
            resume_func,
            prompt,
            ..
        } = started
        else {
            panic!("source gatekeeper dialogue must wait for enter");
        };
        assert_eq!(resume_func.as_deref(), Some("step1"));
        assert_eq!(prompt, "[엔터키를 누르세요]");
        let resumed = do_event_rhai(
            &mut dialogue,
            &data,
            "test",
            &[],
            "test",
            "검후_대_대화.rhai",
            Some("step1".to_string()),
        );
        let CommandResult::MobEventEnter {
            resume_func,
            prompt,
            ..
        } = resumed
        else {
            panic!("source gatekeeper dialogue must continue waiting at step1");
        };
        assert_eq!(resume_func.as_deref(), Some("step2"));
        assert_eq!(prompt, "[엔터키를 누르세요]");

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
    fn sword_empress_terminal_states_keep_the_source_dialogue_before_other_branches() {
        for state in ["혈살루끝", "검후가짜눈물"] {
            let mut body = Body::new();
            body.set("이름", format!("검후종료상태{state}"));
            super::set_user_event(&mut body, state, "1");

            let (output, _) = run_zone_event(&mut body, "절강성", "검후_대_대화.rhai", None);
            assert!(
                output
                    .iter()
                    .any(|line| line.contains("더 이상 볼일이 없습니다")),
                "{state}: {output:?}"
            );
            assert!(
                !get_user_event(&body, state).is_empty(),
                "{state} must only gate dialogue, not be consumed"
            );
            let _ = std::fs::remove_file(format!("data/user/검후종료상태{state}.json"));
        }
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
        let add_required_visions = |body: &mut Body| {
            for vision in [
                "멸천혈폭비전",
                "혈세천하비전",
                "천마검비전",
                "천마무격신장비전",
                "대비단혼강비전",
                "무극검비전",
                "뇌음자흑강비전",
                "강룡십팔장비전",
            ] {
                body.add_secret_skill(vision);
            }
        };

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
            .any(|line| line.contains("혈세천하비전 먼저")));
        assert!(!blocked_by_vision
            .skill_list
            .iter()
            .any(|skill| skill == "역근경"));

        for (skill, threshold, inner_power) in [
            ("태극강기", 60_usize, 3_200_i64),
            ("고영신공", 70, 3_200),
            ("가의신공", 80, 3_200),
            ("명옥공", 85, 3_200),
            ("북명신공", 90, 4_200),
            ("천외비선", 108, 5_200),
        ] {
            let mut body = Body::new();
            body.set("이름", format!("옥황상제{skill}회귀"));
            body.set("최고내공", inner_power);
            add_required_visions(&mut body);
            for index in 0..threshold {
                body.skill_list.push(format!("기초무공{index}"));
            }
            super::try_mob_event(&mut body, "선인", &room, &format!("옥황상제 {skill} 대화"))
                .expect("source skill teaching must select the emperor");
            assert!(body.skill_list.iter().any(|name| name == skill), "{skill}");
            let _ = std::fs::remove_file(format!("data/user/옥황상제{skill}회귀.json"));
        }

        for (skill, threshold, required_power) in
            [("북명신공", 90_usize, 4_200_i64), ("천외비선", 108, 5_200)]
        {
            let mut body = Body::new();
            body.set("이름", format!("옥황상제{skill}내공부족회귀"));
            body.set("최고내공", required_power - 1);
            add_required_visions(&mut body);
            for index in 0..threshold {
                body.skill_list.push(format!("기초무공{index}"));
            }
            let CommandResult::MobEvent { output_lines, .. } =
                super::try_mob_event(&mut body, "선인", &room, &format!("옥황상제 {skill} 대화"))
                    .expect("inner-power-gated teaching must select the emperor")
            else {
                panic!("inner-power-gated teaching returned a non-event result");
            };
            assert!(output_lines
                .iter()
                .any(|line| line.contains(&format!("내공 {required_power}"))));
            assert!(!body.skill_list.iter().any(|name| name == skill));
            let _ = std::fs::remove_file(format!("data/user/옥황상제{skill}내공부족회귀.json"));
        }

        let mut insufficient_count = Body::new();
        insufficient_count.set("이름", "옥황상제수련부족회귀");
        insufficient_count.set("최고내공", 3200_i64);
        add_required_visions(&mut insufficient_count);
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
        add_required_visions(&mut root);
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
        let _rank_guard = RANK_TEST_LOCK.lock().unwrap();
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

        // Python Rank.write_rank keeps 200 entries regardless of the
        // directive's display limit.  Once full, `$순위기록` skips the
        // bracket that consumes the tower-completion events.
        crate::world::rank::rank_clear("무적선인");
        for rank in 0..200 {
            crate::world::rank::rank_write(
                "무적선인",
                &format!("반고상위순위자{rank}"),
                10_000 - rank,
                100,
            );
        }
        let mut outside = Body::new();
        outside.set("이름", "반고순위밖회귀");
        outside.set("무적선인", 1_i64);
        super::set_user_event(&mut outside, "반고선택", "1");
        let CommandResult::MobEvent { output_lines, .. } =
            super::try_mob_event(&mut outside, "선인", &room, "반고 대화")
                .expect("outside-rank Pangu dialogue")
        else {
            panic!("outside-rank Pangu dialogue was not an event");
        };
        assert!(output_lines.iter().any(|line| line.contains("모든 시련")));
        assert!(!get_user_event(&outside, "반고선택").is_empty());
        assert!(get_user_event(&outside, "선인탑끝").is_empty());
        assert_eq!(
            crate::world::rank::rank_read("무적선인", "반고순위밖회귀"),
            0
        );

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
        let _ = std::fs::remove_file("data/user/반고순위밖회귀.json");
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
