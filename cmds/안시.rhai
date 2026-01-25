// 안시.rhai - 안시(ANSI) on/off 설정
// Python: cmds/안시.py

fn cmd(ob, line) {
    let v = if is_empty(line) { 1 } else { max(0, min(1, to_int(trim(line)))) };
    set_value(ob, "안시", v);
    body_save(ob);
    if v == 1 {
        send_line(ob, "☞ 안시가 켜졌습니다.");
    } else {
        send_line(ob, "☞ 안시가 꺼졌습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
