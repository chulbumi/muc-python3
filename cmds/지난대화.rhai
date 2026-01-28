// 지난대화.rhai - 지난대화. Python: cmds/지난대화.py
// ob.talkHistory 순회.

fn cmd(ob, line) {
    let history = get_talk_history(ob);
    if len(history) == 0 {
        send_line(ob, "☞ 아직 대화 기록이 없습니다.");
    } else {
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━");
        send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m  ◁  지난 대화 기록  ▷  \x1b[0m\x1b[37m\x1b[40m");
        send_line(ob, "────────────────────────────");

        for msg in history {
            send_line(ob, msg);
        }

        send_line(ob, "────────────────────────────\x1b[0;37m");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
