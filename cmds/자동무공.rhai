// 자동무공.rhai - 자동 무공 사용 설정
// Python: cmds/자동무공.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        let v = get_string(ob, "자동무공");
        if v == "" {
            send_line(ob, "☞ 자동무공 : 없음");
        } else {
            send_line(ob, "☞ 자동무공 : [\x1b[1;37m" + v + "\x1b[0;37m]");
        }
        return;
    }
    send_line(ob, "☞ 자동 무공 설정 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
