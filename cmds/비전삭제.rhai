// 비전삭제.rhai - 비전 삭제
// Python: cmds/비전삭제.py

fn cmd(ob, line) {
    if get_string(ob, "비전설정") == "" {
        send_line(ob, "☞ 지정된 비전이 없습니다.");
        return;
    }

    set_vision(ob, "");
    send_line(ob, "☞ 지정된 비전을 삭제합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
