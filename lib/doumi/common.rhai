// lib/doumi/common.rhai
// get_name, get_password, get_sex, get_enter
// resume 시: _suspend_counter로 현재 위치 파악, 해당 위치의 저장된 값 반환
// _doumi_position: 현재까지 실행된 suspend 지점의 수 (resume 시 건너뛰기용)
// _saved_name, _saved_password, _saved_sex: 각 필드의 저장된 값

fn get_name() {
    let counter = _suspend_counter_get();
    // 이미 값이 저장되어 있으면 반환 (이미 완료된 필드)
    if _saved_name != "" {
        _suspend_counter_inc();
        return _saved_name;
    }
    // 값이 없고, 아직 입력받을 차례가 아니면 skip
    if counter < _doumi_position {
        _suspend_counter_inc();
        return "";
    }
    // 값이 없고, 바로 입력받아야 하는 경우
    if counter == _doumi_position && _doumi_resume_op == "get_name" {
        _suspend_counter_inc();
        return _saved_name;  // Rust에서 설정한 값
    }
    // 새로운 입력이 필요
    throw #{ type: "doumi_suspend", op: "get_name", prompt: "무림존함ː" };
}

fn get_password() {
    let counter = _suspend_counter_get();
    if _saved_password != "" {
        _suspend_counter_inc();
        return _saved_password;
    }
    if counter < _doumi_position {
        _suspend_counter_inc();
        return "";
    }
    if counter == _doumi_position && _doumi_resume_op == "get_password" {
        _suspend_counter_inc();
        return _saved_password;
    }
    throw #{ type: "doumi_suspend", op: "get_password", prompt: "존함암호ː" };
}

fn get_sex() {
    let counter = _suspend_counter_get();
    if _saved_sex != "" {
        _suspend_counter_inc();
        return _saved_sex;
    }
    if counter < _doumi_position {
        _suspend_counter_inc();
        return "";
    }
    if counter == _doumi_position && _doumi_resume_op == "get_sex" {
        _suspend_counter_inc();
        return _saved_sex;
    }
    throw #{ type: "doumi_suspend", op: "get_sex", prompt: "성별(남/여)ː" };
}

fn get_enter() {
    let counter = _suspend_counter_get();
    // 현재 지점보다 앞서 처리된 suspend 지점이면 건너뜀
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
    // 현재 지점보다 앞서 처리된 suspend 지점이면 건너뜀
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
