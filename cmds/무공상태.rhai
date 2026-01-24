// 무공상태.rhai - 무공상태. Python: cmds/무공상태.py
// [대상] findObjName, target.skills, skillMap, strBar, 방어시간. 관리자면 대상 지정 가능.

fn cmd(ob, line) {
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    send_line(ob, " \x1b[1m\x1b[32mЖ \x1b[37m" + get_string(ob, "이름") + "의 무공집결 상태 \x1b[1m\x1b[32mЖ\x1b[0m\x1b[37m");
    send_line(ob, "──────────────────────────────");
    send_line(ob, "☞ 습득한 무공이 없습니다. (target.skills 연동 예정)");
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
