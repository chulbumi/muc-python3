// Python: cmds/소지품.py

fn cmd(ob, line) {
    let view = get_inventory_view(ob, line);
    if !view["ok"] {
        send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네");
        return;
    }

    send_line(ob, "━━━━━━━━━━━━━━━━━");
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m  ◁     소     지     품     ▷  \x1b[0m\x1b[37m\x1b[40m");
    send_line(ob, "─────────────────");

    if view["empty"] {
        send_line(ob, "\x1b[36m☞ 아무것도 없습니다.\x1b[37m");
    } else {
        for pair in view["items"] {
            let name = pair[0];
            let c = pair[1];
            if c == 1 {
                send_line(ob, "\x1b[36m" + name + "\x1b[37m");
            } else {
                send_line(ob, "\x1b[36m" + name + " \x1b[36m" + int_to_str(c) + "개\x1b[37m");
            }
        }
    }

    send_line(ob, "─────────────────");
    let e = view["silver"];
    send_line(ob, "\x1b[0m\x1b[47m\x1b[30m▶ 은전 : " + pad_start(int_to_str(e), 20, " ") + " 개 \x1b[0m\x1b[37m\x1b[40m");
    let g = view["gold"];
    if g > 0 {
        send_line(ob, "\x1b[0m\x1b[43m\x1b[30m▶ 금전 : " + pad_start(int_to_str(g), 20, " ") + " 개 \x1b[0m\x1b[37m\x1b[40m");
    }
    send_line(ob, "─────────────────\x1b[0;37m");
}

fn main(ob, line) {
    cmd(ob, line);
}
