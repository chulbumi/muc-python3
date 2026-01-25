// 범용 인터랙티브 스크립트용: get_word_input, get_line_input, get_key_input, confirm_input
// _script_resume_op, _script_resume_input, _script_persist는 Rust 러너가 스코프에 push

fn get_word_input(prompt) {
    if _script_resume_op == "get_word" {
        return _script_resume_input;
    }
    throw #{ type: "script_suspend", op: "get_word", prompt: prompt + " " };
}

fn get_line_input(prompt) {
    if _script_resume_op == "get_line" {
        return _script_resume_input;
    }
    throw #{ type: "script_suspend", op: "get_line", prompt: prompt + " " };
}

fn get_key_input(prompt) {
    if _script_resume_op == "get_key" {
        return _script_resume_input;
    }
    throw #{ type: "script_suspend", op: "get_key", prompt: prompt + " " };
}

fn confirm_input(persist_val) {
    if _script_resume_op == "confirm" {
        return;
    }
    throw #{ type: "script_suspend", op: "confirm", prompt: "입력하신 내용이 맞습니까? (네/취소) : ", persist: persist_val };
}

fn end_script() {
    throw #{ type: "script_complete" };
}
