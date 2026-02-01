//! 도우미(초기/빠른) Rhai 스크립트 러너
//!
//! 단계 기반(step-based) 캐릭터 생성 시스템.
//! - lib/doumi/common.rhai: wait_enter, wait_input, wait_key_input 함수
//! - 개별 스크립트: step1_welcome(), step2_name(), ... 함수 정의
//! - 각 단계 함수는 wait_* 호출로 다음 단계 지정하며 suspend

use rhai::{Engine, Scope, Dynamic, Map, EvalAltResult, Module, Shared};
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
    pub next_step: Option<String>, // 다음에 호출할 단계 함수명 (예: "step2_name")
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
///
/// ## Step-based execution (새로운 방식)
/// - `current_step`: Some("step2_name")이면 해당 단계 함수만 호출
/// - None이거나 비어있으면 "step1_welcome"부터 시작
/// - 각 단계 함수는 wait_enter() 또는 wait_input() 호출로 다음 단계 지정
///
/// - `ob`: 도우미용 Map. get_name/get_password/get_sex 결과를 ob["이름"], ob["암호"], ob["성별"]에 적을 수 있음.
/// - `resume`: Some((op, input))이면 해당 op에서 재개
/// - `output`: send_line으로 쌓을 버퍼 (호출자가 초기화 후 전달)
/// - `delay_ms`: set_tick으로 설정할 값 (호출자가 0으로 초기화 후 전달, run 후 읽음)
/// - 반환: Ok(Some((name,pass,gender))) = finish_script 호출됨, Ok(None) = 스크립트 종료만, Err(s) = suspend
pub fn run_doumi(
    script_path: &str,
    ob: &mut Map,
    current_step: Option<&str>,
    resume: Option<(&str, &str)>,
    output: &mut Vec<String>,
    delay_ms: &mut u64,
) -> Result<Option<(String, String, String)>, DoumiSuspend> {
    output.clear();
    let mut finished: Option<(String, String, String)> = None;

    let mut engine = Engine::new();

    // Get resume parameters
    let resume_op = resume.map(|(op, _)| op.to_string()).unwrap_or_else(|| String::new());
    let resume_input = resume.map(|(_, input)| input.to_string()).unwrap_or_else(|| String::new());

    // Determine which step to call
    let step_to_call = current_step.unwrap_or("step1_welcome");

    // Debug logging
    eprintln!("DOUMI run: script_path={}, step_to_call={}, resume_op={:?}, resume_input={:?}, output.len={}, ob.len={}",
        script_path, step_to_call, resume_op, resume_input, output.len(), ob.len());
    if ob.len() > 0 {
        eprintln!("  ob contents: {:?}", ob.iter().map(|(k,v)| (k, v.clone().into_string().unwrap_or_default())).collect::<Vec<_>>());
    }

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
        unsafe { (*out_ptr).push(line); };
    });

    // finish_script(ob) — ob에서 이름/암호/성별 읽어 finished에 넣음.
    let fin = std::cell::Cell::new(&mut finished as *mut Option<(String, String, String)>);
    engine.register_fn("finish_script", move |ob_val: Dynamic| {
        let m = ob_val.clone().try_cast::<Map>();
        let n: String = m.as_ref().and_then(|x| x.get("이름").and_then(|v: &Dynamic| v.clone().into_string().ok())).unwrap_or_default();
        let p: String = m.as_ref().and_then(|x| x.get("암호").and_then(|v: &Dynamic| v.clone().into_string().ok())).unwrap_or_default();
        let g: String = m.as_ref().and_then(|x| x.get("성별").and_then(|v: &Dynamic| v.clone().into_string().ok())).unwrap_or_default();
        eprintln!("[DOUMI] finish_script called: name={}, password={}, gender={}", n, if p.is_empty() { "(empty)" } else { "(hidden)" }, g);
        unsafe { *fin.get() = Some((n, p, g)) };
    });

    // _doumi_resume_op와 _doumi_resume_input을 전역 모듈에 등록
    let mut doumi_module = Module::new();
    doumi_module.set_var("_doumi_resume_op", resume_op.clone());
    doumi_module.set_var("_doumi_resume_input", resume_input.clone());

    // ob에서 이미 저장된 값들을 읽어옴
    let name_input: String = ob.get("이름").and_then(|v| v.clone().into_string().ok()).unwrap_or_default();
    let password_input: String = ob.get("암호").and_then(|v| v.clone().into_string().ok()).unwrap_or_default();
    let sex_input: String = ob.get("성별").and_then(|v| v.clone().into_string().ok()).unwrap_or_default();

    // 현재 resume에 대한 새 값 추가 (ob에 아직 반영되지 않은 값)
    let (final_name, final_password, final_sex) = if resume_op == "get_name" {
        (resume_input.clone(), password_input, sex_input)
    } else if resume_op == "get_password" {
        (name_input, resume_input.clone(), sex_input)
    } else if resume_op == "get_sex" {
        (name_input, password_input, resume_input.clone())
    } else {
        (name_input, password_input, sex_input)
    };

    doumi_module.set_var("_saved_name", final_name);
    doumi_module.set_var("_saved_password", final_password);
    doumi_module.set_var("_saved_sex", final_sex);

    engine.register_global_module(Shared::new(doumi_module));

    // scope 생성
    let mut scope = Scope::new();
    scope.push("ob", ob.clone());

    // common.rhai 로드
    let common_path = Path::new("lib/doumi/common.rhai");
    let common_src = if common_path.exists() {
        std::fs::read_to_string(common_path).unwrap_or_default()
    } else {
        String::new()
    };

    let path = Path::new(script_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let path_str = if ext.is_empty() {
        format!("{}.rhai", script_path)
    } else {
        script_path.to_string()
    };
    let full = Path::new(&path_str);
    let main_src = if full.exists() {
        std::fs::read_to_string(full).map_err(|_| DoumiSuspend {
            op: "load".to_string(),
            prompt: format!("스크립트를 찾을 수 없습니다: {}", path_str),
            expected: None,
            next_step: None,
        })?
    } else {
        return Err(DoumiSuspend {
            op: "load".to_string(),
            prompt: format!("스크립트를 찾을 수 없습니다: {}", path_str),
            expected: None,
            next_step: None,
        });
    };

    // common.rhai와 메인 스크립트를 합쳐서 실행
    let combined_src = format!("{}\n{}", common_src, main_src);

    // scope 생성 - ob를 scope에 등록
    let mut scope = Scope::new();
    scope.push("ob", ob.clone());

    // 현재 단계 함수만 호출, ob를 인자로 전달
    let call_src = format!("{}\n{}(ob);", combined_src, step_to_call);

    let result = engine.eval_with_scope::<Dynamic>(&mut scope, &call_src);

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
        Ok(_) => {
            eprintln!("DOUMI finished: script_path={}, step={}, finished={:?}", script_path, step_to_call, finished);
            return Ok(finished);
        },
        Err(e) => {
            // Extract the inner error value that contains our Map
            fn extract_error_value(mut err: &EvalAltResult) -> Option<Dynamic> {
                loop {
                    match err {
                        EvalAltResult::ErrorRuntime(v, _) => return Some(v.clone()),
                        EvalAltResult::ErrorInFunctionCall(_, _, inner, _) => {
                            err = inner;
                        }
                        _ => return None,
                    }
                }
            }

            if let Some(err_value) = extract_error_value(&e) {
                if let Some(m) = err_value.clone().try_cast::<Map>() {
                    let t: String = m.get("type").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                    if t == "doumi_suspend" {
                        let op: String = m.get("op").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                        let prompt: String = m.get("prompt").and_then(|v: &Dynamic| v.clone().into_string().ok()).unwrap_or_default();
                        let expected: Option<String> = m.get("expected").and_then(|v: &Dynamic| v.clone().into_string().ok());
                        // next_step를 에러 Map에서 추출
                        let next_step: Option<String> = m.get("next_step").and_then(|v: &Dynamic| v.clone().into_string().ok());
                        // Sync ob before returning suspend
                        if let Some(d) = scope.get_value::<Dynamic>("ob") {
                            if let Some(ob_map) = d.try_cast::<Map>() {
                                ob.clear();
                                for (k, v) in ob_map {
                                    ob.insert(k, v);
                                }
                            }
                        }
                        eprintln!("DOUMI suspend: step={}, op={}, prompt={}, next_step={:?}, output_lines={}",
                            step_to_call, op, prompt, next_step, output.len());
                        return Err(DoumiSuspend { op, prompt, expected, next_step });
                    }
                }
            }

            // If not doumi_suspend or couldn't extract, return error
            eprintln!("DOUMI error: script_path={}, step={}, error={:?}", script_path, step_to_call, e);
            return Err(DoumiSuspend {
                op: "error".to_string(),
                prompt: (*e).to_string(),
                expected: None,
                next_step: None,
            });
        }
    }
}

/// run_doumi를 호출하고, Suspend/Finished를 DoumiRunResult로 변환.
pub fn run_doumi_to_result(
    script_path: &str,
    ob: &mut Map,
    current_step: Option<&str>,
    resume: Option<(&str, &str)>,
) -> DoumiRunResult {
    let mut output = Vec::new();
    let mut delay_ms = 0u64;

    match run_doumi(script_path, ob, current_step, resume, &mut output, &mut delay_ms) {
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
