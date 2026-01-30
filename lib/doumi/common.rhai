// lib/doumi/common.rhai
// get_name, get_password, get_sex, get_enter
// 각각: _doumi_resume_op == op 이면 _doumi_resume_input 반환(재개), 아니면 throw doumi_suspend
// _doumi_position: 현재까지 실행된 suspend 지점의 수 (resume 시 건너뛰기용)
// _suspend_counter_get(), _suspend_counter_inc()는 Rust 함수로 counter 관리

fn get_name() {
    let counter = _suspend_counter_get();
    // 이미 처리된 suspend 지점이면 건너뜀
    if counter < _doumi_position {
        _suspend_counter_inc();
        return;
    }
    // 현재 지점에서 resume 중이면 입력 반환
    if counter == _doumi_position && _doumi_resume_op == "get_name" {
        _suspend_counter_inc();
        return _doumi_resume_input;
    }
    throw #{ type: "doumi_suspend", op: "get_name", prompt: "무림존함ː" };
}

fn get_password() {
    let counter = _suspend_counter_get();
    // 이미 처리된 suspend 지점이면 건너뜀
    if counter < _doumi_position {
        _suspend_counter_inc();
        return;
    }
    // 현재 지점에서 resume 중이면 입력 반환
    if counter == _doumi_position && _doumi_resume_op == "get_password" {
        _suspend_counter_inc();
        return _doumi_resume_input;
    }
    throw #{ type: "doumi_suspend", op: "get_password", prompt: "존함암호ː" };
}

fn get_sex() {
    let counter = _suspend_counter_get();
    // 이미 처리된 suspend 지점이면 건너뜀
    if counter < _doumi_position {
        _suspend_counter_inc();
        return;
    }
    // 현재 지점에서 resume 중이면 입력 반환
    if counter == _doumi_position && _doumi_resume_op == "get_sex" {
        _suspend_counter_inc();
        return _doumi_resume_input;
    }
    throw #{ type: "doumi_suspend", op: "get_sex", prompt: "성별(남/여)ː" };
}

fn get_enter() {
    let counter = _suspend_counter_get();
    // 이미 처리된 suspend 지점이면 건너뜀
    if counter < _doumi_position {
        _suspend_counter_inc();
        return;
    }
    // 현재 지점에서 resume 중이면 입력 반환 (엔터는 빈 값)
    if counter == _doumi_position && _doumi_resume_op == "get_enter" {
        _suspend_counter_inc();
        return;
    }
    throw #{ type: "doumi_suspend", op: "get_enter", prompt: "【엔터키를 누르세요】" };
}

// 특정 문자열 입력 대기. expected와 일치할 때까지 재입력. throw에 expected 넣어 클라이언트가 검증.
fn get_key_input(expected) {
    let counter = _suspend_counter_get();
    // 이미 처리된 suspend 지점이면 건너뜀
    if counter < _doumi_position {
        _suspend_counter_inc();
        return;
    }
    // 현재 지점에서 resume 중이면 입력 반환
    if counter == _doumi_position && _doumi_resume_op == "get_key_input" {
        _suspend_counter_inc();
        return;
    }
    throw #{ type: "doumi_suspend", op: "get_key_input", prompt: "", expected: expected };
}
