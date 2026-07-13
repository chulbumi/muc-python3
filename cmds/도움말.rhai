// 도움말.rhai - 도움말 명령어
// Python: cmds/도움말.py

fn cmd(ob, line) {
    // Python indexes HELP with the non-empty argument exactly as received.
    let normalized = trim(line);
    let topic = if normalized == "" { "도움말" } else { normalized };
    let txt = get_help(topic);
    if length(txt) > 0 {
        send_line(ob, txt);
    } else {
        send_line(ob, "☞ 해당 도움말이 없어요. ^^");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
