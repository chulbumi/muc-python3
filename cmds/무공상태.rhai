// 무공상태.rhai - 무공 상태 확인
// Python: cmds/무공상태.py

fn cmd(ob, line) {
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    send_line(ob, " \x1b[1m\x1b[32mЖ \x1b[37m" + get_string(ob, "이름") + "의 무공집결 상태 \x1b[1m\x1b[32mЖ\x1b[0m\x1b[37m");
    send_line(ob, "──────────────────────────────");
    send_line(ob, "☞ 무공 상태 표시 기능은 현재 구현 중입니다.");
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
