// 지난대화.rhai - 지난대화. Python: cmds/지난대화.py
// ob.talkHistory 순회.

fn cmd(ob, line) {
    let history = get_talk_history(ob);
    for msg in history {
        send_line(ob, msg);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
