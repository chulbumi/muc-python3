// 무리.rhai - 무리. Python: cmds/무리.py
// line=='': viewParty(Party, follower). else: addParty. ob.Party, ob.follower 없음.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        send_line(ob, "◁ \x1b[1m" + get_string(ob, "이름") + "\x1b[0m\x1b[40m\x1b[37m의 동행 ▷");
        send_line(ob, "────────────────────────────");
        send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
        send_line(ob, "동행중");
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        return;
    }
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
