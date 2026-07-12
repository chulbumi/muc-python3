// 지난잡담.rhai - 지난 대화 기록 확인
// Python: cmds/지난잡담.py

fn cmd(ob, line) {
    let history = get_chat_history(ob);
    for h in history {
        send_line(ob, h);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
