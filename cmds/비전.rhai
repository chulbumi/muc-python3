// 비전.rhai - 비전 설정
// Python: cmds/비전.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        let v = get_string(ob, "비전설정");
        if v == "" {
            send_line(ob, "☞ 비전 : 없음");
        } else {
            send_line(ob, "☞ 비전 : [\x1b[1;37m" + v + "\x1b[0;37m]");
        }
        return;
    }
    send_line(ob, "☞ 비전 설정 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
