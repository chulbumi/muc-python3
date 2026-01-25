// 꼬리말.rhai - 꼬리말 명령어. Python: cmds/꼬리말.py
// set_value(ob, "꼬리말", line) → Body 저장. 표시 시 get_body_string(ob,"꼬리말") 참조.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 꼬리말");
        return;
    }
    if length(line) > 20 {
        send_line(ob, "☞ 너무 깁니다.");
        return;
    }
    set_value(ob, "꼬리말", line);
    send_line(ob, "☞ 꼬리말을 설정 하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
