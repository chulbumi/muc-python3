// 무리.rhai - 무리. Python: cmds/무리.py
// line=='': viewParty(Party, follower). else: addParty. ob.Party, ob.follower 없음.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        send_line(ob, "◁ \x1b[1m" + get_string(ob, "이름") + "\x1b[0m\x1b[40m\x1b[37m의 동행 ▷");
        send_line(ob, "────────────────────────────");
        send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^ (Party/follower 연동 예정)");
        send_line(ob, "동행중");
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        return;
    }
    send_line(ob, "☞ 당신을 따르는 대상이 아닙니다. (follower 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
