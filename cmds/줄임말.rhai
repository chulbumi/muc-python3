// 줄임말.rhai - 줄임말 명령어
// Python: cmds/줄임말.py

fn cmd(ob, line) {
    let v = if is_empty(line) { 1 } else { max(0, min(1, to_int(trim(line)))) };
    set_value(ob, "줄임말", v);
    body_save(ob);
    if v == 1 {
        send_line(ob, "☞ 줄임말이 켜졌습니다.");
    } else {
        send_line(ob, "☞ 줄임말이 꺼졌습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
