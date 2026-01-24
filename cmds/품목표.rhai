// 품목표.rhai - 품목표 명령어
// Python: cmds/품목표.py

fn cmd(ob, line) {
    let txt = get_merchant_script(ob);
    if length(txt) > 0 {
        send_line(ob, txt);
    } else {
        send_line(ob, "☞ 품목을 보여줄 상인이 없어요. ^^");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
