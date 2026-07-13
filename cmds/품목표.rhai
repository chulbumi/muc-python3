// 품목표.rhai - 품목표 명령어
// Python: cmds/품목표.py

fn cmd(ob, line) {
    let lines = get_merchant_script(ob);
    if len(lines) > 0 {
        for line in lines { send_line(ob, line); }
    } else {
        send_line(ob, "☞ 품목을 보여줄 상인이 없어요. ^^");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
