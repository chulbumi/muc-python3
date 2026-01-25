// lib/doumi/common.rhai
// get_name, get_password, get_sex, get_enter
// 각각: _doumi_resume_op == op 이면 _doumi_resume_input 반환(재개), 아니면 throw doumi_suspend

fn get_name() {
    if _doumi_resume_op == "get_name" {
        return _doumi_resume_input;
    }
    throw #{ type: "doumi_suspend", op: "get_name", prompt: "무림존함ː" };
}

fn get_password() {
    if _doumi_resume_op == "get_password" {
        return _doumi_resume_input;
    }
    throw #{ type: "doumi_suspend", op: "get_password", prompt: "존함암호ː" };
}

fn get_sex() {
    if _doumi_resume_op == "get_sex" {
        return _doumi_resume_input;
    }
    throw #{ type: "doumi_suspend", op: "get_sex", prompt: "성별(남/여)ː" };
}

fn get_enter() {
    if _doumi_resume_op == "get_enter" {
        return;
    }
    throw #{ type: "doumi_suspend", op: "get_enter", prompt: "【엔터키를 누르세요】" };
}
