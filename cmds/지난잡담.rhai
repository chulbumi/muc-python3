// 지난잡담.rhai - 지난 대화 기록 확인
// Python: cmds/지난잡담.py

fn cmd(ob, line) {
    let history = get_talk_history(ob);
    if len(history) == 0 {
        send_line(ob, "☞ 기록된 대화가 없습니다.");
        return;
    }
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    send_line(ob, " ◁ 지난 대화 기록 ▷");
    send_line(ob, "────────────────────────────");
    for h in history {
        send_line(ob, "　 " + h);
    }
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
