// 머리말.rhai - 머리말 명령어. Python: cmds/머리말.py
// set_value(ob, "머리말", line) → Body 저장. 표시 시 get_body_string(ob,"머리말") 참조.

fn cmd(ob, line) {
    let value = trim(line);
    if value == "" {
        send_line(ob, "☞ 사용법: [내용] 머리말");
        return;
    }
    if length(value) > 20 {
        send_line(ob, "☞ 너무 깁니다.");
        return;
    }
    set_text_affix(ob, "머리말", value);
    send_line(ob, "☞ 머리말을 설정 하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
