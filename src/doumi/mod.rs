//! 도우미(초기/빠른) Rhai 스크립트 러너
//!
//! doumi.json 없이 lib/doumi/*.rhai 스크립트를 Rhai 언어로 실행합니다.
//! - set_tick(n), send_line(ob, msg), get_name(), get_password(), get_sex(), get_enter(), start_script(ob), finish_script(ob)

use rhai::{Engine, Scope, Dynamic, Map, EvalAltResult};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::hangul::{han_ira, han_iga};

/// [공], [공](이라/라), [공](아/야), [공](이/가) 치환
fn substitute_doumi_variables(text: &str, name: &str, _gender: &str) -> String {
    fn has_batchilm(s: &str) -> bool {
        s.chars().last().map_or(false, |c| {
            let code = c as u32;
            (0xAC00..=0xD7A3).contains(&code) && ((code - 0xAC00) % 28) > 0
        })
    }
    let mut r = text.to_string();
    if r.contains("[공](이라/라)") {
        r = r.replace("[공](이라/라)", &format!("{}{}", name, han_ira(name)));
    }
    if r.contains("[공](아/야)") {
        let p = if has_batchilm(name) { "아" } else { "야" };
        r = r.replace("[공](아/야)", &format!("{}{}", name, p));
    }
    if r.contains("[공](이/가)") {
        r = r.replace("[공](이/가)", &format!("{}{}", name, han_iga(name)));
    }
    r = r.replace("[공]", name);
    r
}

/// suspend 시 반환되는 정보. Op에 따라 클라이언트가 입력 검증 후 resume.
#[derive(Debug, Clone)]
pub struct DoumiSuspend {
    pub op: String,
    pub prompt: String,
    pub expected: Option<String>, // get_key_input용
}

/// 한 번의 run 결과: 출력 + 지연 + (끝남 | suspend)
#[derive(Debug)]
pub enum DoumiRunResult {
    Suspend {
        lines: Vec<String>,
        delay_ms: u64,
        suspend: DoumiSuspend,
    },
    Finished {
        name: String,
        password: String,
        gender: String,
    },
}

/// lib/doumi/common.rhai + 메인 스크립트를 실행.
/// - `ob`: 도우미용 Map. get_name/get_password/get_sex 결과를 ob["이름"], ob["암호"], ob["성별"]에 적을 수 있음.
/// - `resume`: Some((op, input))이면 해당 op에서 재개. get_name 등이 input을 반환함.
/// - `output`: send_line으로 쌓을 버퍼 (호출자가 초기화 후 전달)
/// - `delay_ms`: set_tick으로 설정할 값 (호출자가 0으로 초기화 후 전달, run 후 읽음)
/// - 반환: Ok(Some((name,pass,gender))) = finish_script 호출됨, Ok(None) = 스크립트 종료만, Err(s) = suspend
pub fn run_doumi(
    script_path: &str,
    ob: &mut Map,
    resume: Option<(&str, &str)>,
    output: &mut Vec<String>,
    delay_ms: &mut u64,
) -> Result<Option<(String, String, String)>, DoumiSuspend> {
    output.clear();
    let mut finished: Option<(String, String, String)> = None;

    let mut engine = Engine::new();

    // set_tick(n) — n*100 ms. delay_ms에 기록.
    let dms = Arc::new(AtomicU64::new(*delay_ms));
    let dms_set = dms.clone();
    engine.register_fn("set_tick", move |n: i64| {
        dms_set.store(((n).max(0) as u64) * 100, Ordering::SeqCst);
    });

    // send_line(ob, msg) — [공] 치환 후 output에 push. \r\n 붙임.
    let out_ptr = output as *mut Vec<String>;
    engine.register_fn("send_line", move |ob_val: Dynamic, msg: &str| {
        let name: String = ob_val.clone().try_cast::<Map>()
            .and_then(|m| m.get("이름").and_then(|v: &Dynamic| v.clone().into_string().ok()))
            .unwrap_or_default();
        let gender: String = ob_val.clone().try_cast::<Map>()
            .and_then(|m| m.get("성별").and_then(|v: &Dynamic| v.clone().into_string().ok()))
            .unwrap_or_default();
        let s = substitute_doumi_variables(msg, &name, &gender);
        let line = if s.is_empty() { "\r\n".to_string() } else { format!("{}\r\n", s) };
        unsafe { (*out_ptr).push(line) };
    });

    // start_script(ob) — 스크립트 시작 훅. 현재 no-op, 필요 시 초기화 등.
    engine.register_fn("start_script", |_ob: Dynamic| {});

    // finish_script(ob) — ob에서 이름/암호/성별 읽어 finished에 넣음.
    let fin = std::cell::Cell::new(&mut finished as *mut Option<(String, String, String)>);
    engine.register_fn("finish_script", move |ob_val: Dynamic| {
        let m = ob_val.clone().try_cast::<Map>();
        let n: String = m.as_ref().and_then(|x| x.get("이름").and_then(|v: &Dynamic| v.clone().into_string().ok())).unwrap_or_default();
        let p: String = m.as_ref().and_then(|x| x.get("암호").and_then(|v: &Dynamic| v.clone().into_string().ok())).unwrap_or_default();
        let g: String = m.as_ref().and_then(|x| x.get("성별").and_then(|v: &Dynamic| v.clone().into_string().ok())).unwrap_or_default();
        unsafe { *fin.get() = Some((n, p, g)) };
    });

    // common.rhai 로드 (get_name, get_password, get_sex, get_enter, doumi_suspend)
    let common_path = Path::new("lib/doumi/common.rhai");
    if common_path.exists() {
        let common_src = std::fs::read_to_string(common_path).unwrap_or_default();
        if let Err(e) = engine.run(&common_src) {
            // common.rhai 자체가 get_* 호출해 suspend할 수는 없으므로, 진짜 오류면 런타임에 나옴.
            tracing::warn!("doumi common.rhai load/run: {:?}", e);
        }
    }

    let mut scope = Scope::new();
    scope.push("ob", ob.clone());
    scope.push("_doumi_resume_op", resume.as_ref().map(|r| r.0.to_string()).unwrap_or_else(|| String::new()));
    scope.push("_doumi_resume_input", resume.as_ref().map(|r| r.1.to_string()).unwrap_or_else(|| String::new()));

    let path = Path::new(script_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let path_str = if ext.is_empty() {
        format!("{}.rhai", script_path)
    } else {
        script_path.to_string()
    };
    let full = Path::new(&path_str);
    let src = if full.exists() {
        std::fs::read_to_string(full).map_err(|_| DoumiSuspend {
            op: "load".to_string(),
            prompt: format!("스크립트를 찾을 수 없습니다: {}", path_str),
            expected: None,
        })?
    } else {
        return Err(DoumiSuspend {
            op: "load".to_string(),
            prompt: format!("스크립트를 찾을 수 없습니다: {}", path_str),
            expected: None,
        });
    };

    let result = engine.eval_with_scope::<Dynamic>(&mut scope, &src);

    *delay_ms = dms.load(Ordering::SeqCst);

    // scope의 ob 갱신을 caller ob에 반영 (get_name 등으로 ob["이름"] 등이 설정됨)
    if let Some(d) = scope.get_value::<Dynamic>("ob") {
        if let Some(m) = d.try_cast::<Map>() {
            ob.clear();
            for (k, v) in m {
                ob.insert(k, v);
            }
        }
    }

    match result {
        Ok(_) => return Ok(finished),
        Err(e) => {
            if let EvalAltResult::ErrorRuntime(err, _) = *e {
                if let Some(m) = err.clone().try_cast::<Map>() {
                    let t: String = m.get("type").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                    if t == "doumi_suspend" {
                        let op: String = m.get("op").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                        let prompt: String = m.get("prompt").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                        let expected: Option<String> = m.get("expected").and_then(|v: &Dynamic| v.clone().into_string().ok());
                        return Err(DoumiSuspend { op, prompt, expected });
                    }
                }
                return Err(DoumiSuspend {
                    op: "error".to_string(),
                    prompt: err.to_string(),
                    expected: None,
                });
            }
            return Err(DoumiSuspend {
                op: "error".to_string(),
                prompt: (*e).to_string(),
                expected: None,
            });
        }
    }
}

/// run_doumi를 호출하고, Suspend/Finished를 DoumiRunResult로 변환.
pub fn run_doumi_to_result(
    script_path: &str,
    ob: &mut Map,
    resume: Option<(&str, &str)>,
) -> DoumiRunResult {
    let mut output = Vec::new();
    let mut delay_ms = 0u64;

    match run_doumi(script_path, ob, resume, &mut output, &mut delay_ms) {
        Ok(Some((n, p, g))) => DoumiRunResult::Finished { name: n, password: p, gender: g },
        Ok(None) => {
            let (n, p, g) = (
                ob.get("이름").and_then(|v| v.clone().into_string().ok()).unwrap_or_default(),
                ob.get("암호").and_then(|v| v.clone().into_string().ok()).unwrap_or_default(),
                ob.get("성별").and_then(|v| v.clone().into_string().ok()).unwrap_or_default(),
            );
            DoumiRunResult::Finished { name: n, password: p, gender: g }
        }
        Err(s) => DoumiRunResult::Suspend {
            lines: output,
            delay_ms,
            suspend: s,
        },
    }
}
