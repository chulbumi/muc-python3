// 자동무공삭제.rhai - 자동 무공 지정 해제
// Python: cmds/자동무공삭제.py

fn cmd(ob, line) {
    if get_body_string(ob, "자동무공") == "" {
        send_line(ob, "☞ 자동무공 : 없음");
        return;
    }
    set_value(ob, "자동무공", "");
    send_line(ob, "☞ 자동무공을 삭제하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
