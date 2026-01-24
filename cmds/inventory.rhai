// inventory.rhai - 소지품 (소지품.rhai와 동일, list_inventory 사용)

fn cmd(ob, line) {
    send_line(ob, "━━━━━━━━━━━━━━━━━");
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m  ◁     소     지     품     ▷  \x1b[0m\x1b[37m\x1b[40m");
    send_line(ob, "─────────────────");

    let items = list_inventory(ob);
    if len(items) == 0 {
        send_line(ob, "\x1b[36m☞ 아무것도 없습니다.\x1b[37m");
    } else {
        for pair in items {
            let name = pair[0];
            let c = pair[1];
            if c == 1 {
                send_line(ob, "\x1b[36m" + name + "\x1b[37m");
            } else {
                send_line(ob, "\x1b[36m" + name + " \x1b[36m" + c + "개\x1b[37m");
            }
        }
    }

    send_line(ob, "─────────────────");
    let e = get_int(ob, "은전");
    send_line(ob, "\x1b[0m\x1b[47m\x1b[30m▶ 은전 : " + e + " 개 \x1b[0m\x1b[37m\x1b[40m");
    let g = get_int(ob, "금전");
    if g > 0 {
        send_line(ob, "\x1b[0m\x1b[43m\x1b[30m▶ 금전 : " + g + " 개 \x1b[0m\x1b[37m\x1b[40m");
    }
    send_line(ob, "─────────────────\x1b[0;37m");
}

fn main(ob, line) {
    cmd(ob, line);
}
