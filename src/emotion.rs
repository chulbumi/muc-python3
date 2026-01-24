//! 감정표현(emotion) 모듈
//!
//! data/config/emotion.json의 "감정표현" 데이터를 로드하고,
//! 파이썬 objs/emotion.py (Emotion.load, replace, makeScript), objs/player.doEmotion 로직을 제공.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use crate::command::CommandResult;
use crate::hangul::post_position_all;
use crate::player::Body;

/// emotion.json 로딩. "감정표현" → 키별 [kd0, kd1] 또는 [kd0, kd1, kd2].
/// 키 "감사 감"처럼 공백으로 여러 alias가 있으면 각 alias마다 같은 Vec를 등록.
/// RwLock로 감싸 관리자 명령 '업데이트 표현'에서 재로딩 가능.
static EMOTION_MAP: Lazy<RwLock<HashMap<String, Vec<String>>>> = Lazy::new(|| {
    RwLock::new(load_emotion_map().unwrap_or_default())
});

#[derive(serde::Deserialize)]
struct EmotionJson {
    #[serde(rename = "감정표현")]
    emotions: HashMap<String, Vec<String>>,
}

fn load_emotion_map() -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    let path = Path::new("data/config/emotion.json");
    let content = std::fs::read_to_string(path)?;
    let j: EmotionJson = serde_json::from_str(&content)?;
    let mut by_alias = HashMap::new();
    for (key, val) in j.emotions {
        for alias in key.split_whitespace() {
            by_alias.insert(alias.to_string(), val.clone());
        }
    }
    Ok(by_alias)
}

/// alias로 템플릿 배열 조회. 없으면 None. ( cloning; Vec는 보통 2~3개 원소 )
pub fn get_templates(cmd: &str) -> Option<Vec<String>> {
    EMOTION_MAP.read().unwrap().get(cmd).cloned()
}

/// alias인지 여부.
pub fn is_emotion_command(cmd: &str) -> bool {
    EMOTION_MAP.read().unwrap().contains_key(cmd)
}

/// emotion.json을 다시 로드. 관리자 명령 '업데이트 표현'에서 호출.
pub fn reload_emotion_map() -> Result<(), String> {
    let map = load_emotion_map().map_err(|e| e.to_string())?;
    *EMOTION_MAP.write().unwrap() = map;
    Ok(())
}

/// line에서 첫 번째 "..." 구간 내용을 sub로 치환. 파이썬 Emotion.replace.
pub fn replace_quoted(line: &str, sub: &str) -> String {
    if sub.is_empty() {
        return line.to_string();
    }
    let s = match line.find('"') {
        Some(i) => i,
        None => return line.to_string(),
    };
    let e = match line[s + 1..].find('"') {
        Some(j) => s + 1 + j,
        None => return line.to_string(),
    };
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..=s]);
    out.push_str(sub);
    out.push_str(&line[e..]);
    out
}

/// makeScript: replace 적용 후 [공]->당신/I, [방]/[방성]->당신/U, post_position_all.
/// U가 None: buf2='', buf1·buf3만 사용.
/// 반환: (buf1: 본인, buf2: 대상 플레이어 전용, buf3: 방 전체·대상 제외)
pub fn make_script(line: &str, i: &str, u: Option<&str>, sub: &str) -> (String, String, String) {
    let line = replace_quoted(line, sub);
    if u.is_none() {
        let b1 = line.replace("[공]", "당신");
        let b1 = post_position_all(&b1);
        let b3 = line.replace("[공]", i);
        let b3 = post_position_all(&b3);
        return (b1, String::new(), b3);
    }
    let u = u.unwrap();
    let b1 = line.replace("[공]", "당신").replace("[방]", u).replace("[방성]", u);
    let b1 = post_position_all(&b1);

    let b2 = line.replace("[공]", i).replace("[방]", "당신").replace("[방성]", "당신");
    let b2 = post_position_all(&b2);

    let b3 = line.replace("[공]", i).replace("[방]", u).replace("[방성]", u);
    let b3 = post_position_all(&b3);
    (b1, b2, b3)
}

/// find_in_room에서 반환. 플레이어면 접촉거부 여부 포함.
#[derive(Debug, Clone)]
pub enum EmotionTarget {
    Player {
        name: String,
        contact_refuse: bool,
    },
    Mob { name: String },
}

/// do_emotion: 파이썬 objs/player.doEmotion.
/// kd=emotion.json 템플릿 [kd0,kd1] 또는 [kd0,kd1,kd2]. 대상 없음/self→kd[0]; 몹→kd[1]; 플레이어→kd[1], 접촉거부+3번째 있음→kd[2].
/// I=getNameA(노랑), U=대상 getNameA(노랑). replace(인용구)→sub, [공]/[방]/[방성] 치환·post_position.
pub fn do_emotion(
    body: &Body,
    cmd: &str,
    param: &str,
    target: Option<EmotionTarget>,
) -> CommandResult {
    let kd = match get_templates(cmd) {
        Some(v) => v,
        None => return CommandResult::Error("해당 감정표현을 찾을 수 없습니다.".to_string()),
    };
    // 파이썬 makeScript(I=getNameA, U=getNameA). [공]/[방] 치환 시 노란색 이름 사용.
    let i = body.get_name_a();
    let u_owned: Option<String> = match &target {
        Some(EmotionTarget::Player { name, .. }) | Some(EmotionTarget::Mob { name }) => {
            Some(format!("\x1b[33m{}\x1b[37m", name))
        }
        None => None,
    };
    let u_ref = u_owned.as_ref().map(|s| s.as_str());

    if param.is_empty() {
        let (buf1, _buf2, buf3) = make_script(&kd[0], &i, None, param);
        return CommandResult::EmotionToRoom(buf1, buf3, None);
    }

    let first = param.split_whitespace().next().unwrap_or("");
    let sub_rest = param
        .trim_start()
        .strip_prefix(first)
        .map(|s| s.trim())
        .unwrap_or("");

    let (buf1, buf2, buf3) = match &target {
        None => make_script(&kd[0], &i, None, param),
        Some(EmotionTarget::Mob { .. }) => make_script(&kd[1], &i, u_ref, sub_rest),
        Some(EmotionTarget::Player { contact_refuse, .. }) => {
            let e = if *contact_refuse && kd.len() >= 3 {
                kd[2].as_str()
            } else {
                kd[1].as_str()
            };
            make_script(e, &i, u_ref, sub_rest)
        }
    };

    let to_target = match &target {
        Some(EmotionTarget::Player { name, .. }) => Some((name.clone(), buf2)),
        _ => None,
    };

    CommandResult::EmotionToRoom(buf1, buf3, to_target)
}
