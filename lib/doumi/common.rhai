// lib/doumi/common.rhai
// 단계 기반(step-based) 스크립트를 위한 공통 함수들
// wait_enter, wait_input, wait_key_input은 다음 단계를 지정하며 suspend

// 엔터키 입력 대기 - 다음 단계로 넘어감
fn wait_enter(next_step) {
    throw #{
        type: "doumi_suspend",
        op: "wait_enter",
        prompt: "【엔터키를 누르세요】",
        next_step: next_step
    };
}

// 사용자 입력 대기 - op에 따라 다른 프롬프트 표시
fn wait_input(next_step, op) {
    let prompt = if op == "get_name" {
        "무림존함ː"
    } else if op == "get_password" {
        "비밀번호ː"
    } else if op == "get_sex" {
        "성별(남/여)ː"
    } else {
        "> "
    };
    throw #{
        type: "doumi_suspend",
        op: op,
        prompt: prompt,
        next_step: next_step
    };
}

// 특정 키 입력 대기 - expected와 일치하는 입력만 허용
fn wait_key_input(next_step, expected) {
    throw #{
        type: "doumi_suspend",
        op: "get_key_input",
        prompt: "『" + expected + "』를 입력 하세요",
        expected: expected,
        next_step: next_step
    };
}

// 레거시 호환을 위한 함수들 (기존 스크립트와 호환)
fn get_name() {
    throw #{ type: "doumi_suspend", op: "get_name", prompt: "무림존함ː" };
}

fn get_password() {
    throw #{ type: "doumi_suspend", op: "get_password", prompt: "존함암호ː" };
}

fn get_sex() {
    throw #{ type: "doumi_suspend", op: "get_sex", prompt: "성별(남/여)ː" };
}

fn get_enter() {
    throw #{ type: "doumi_suspend", op: "get_enter", prompt: "【엔터키를 누르세요】" };
}

fn get_key_input(expected) {
    throw #{ type: "doumi_suspend", op: "get_key_input", prompt: "", expected: expected };
}

// 초기화 함수 (단계 기반에서는 no-op, 하위 호환용)
fn start_script(ob) {
    // 스크립트 시작 시 초기화 (step-based에서는 no-op)
    // 이전 버전과의 호환성을 위해 제공
}

