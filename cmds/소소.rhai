// 소소.rhai - 소소. Python: cmds/소소.py
// [대상] findObjName or ob. target.getInvenItemCount, objs. list_inventory(ob) 사용.

fn cmd(ob, line) {
    send_line(ob, "━━━━━━━━━━━━━━━━━");
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m  ◁     소     지     품     ▷  \x1b[0m\x1b[37m\x1b[40m");
    send_line(ob, "─────────────────");
    let inv = list_inventory(ob);
    if len(inv) == 0 {
        send_line(ob, "\x1b[36m☞ 아무것도 없습니다.\x1b[37m");
    } else {
        for it in inv {
            let name = it[0];
            let cnt = it[1];
            if cnt == 1 {
                send_line(ob, "\x1b[36m" + name + "\x1b[37m");
            } else {
                send_line(ob, "\x1b[36m" + name + " \x1b[36m" + int_to_str(cnt) + "개\x1b[37m");
            }
        }
    }
    send_line(ob, "─────────────────");
    send_line(ob, "\x1b[0m\x1b[47m\x1b[30m▶ 은전 : " + pad_start(int_to_str(get_int(ob, "은전")), 20, " ") + " 개 \x1b[0m\x1b[37m\x1b[40m");
    send_line(ob, "─────────────────");
}

fn main(ob, line) {
    cmd(ob, line);
}
