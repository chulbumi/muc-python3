// Python `cmds/소소.py`: 본인 또는 관리자 같은 방 대상의 간단 소지품 표시.

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
            let count = pair[1];
            if count == 1 {
                send_line(ob, "\x1b[36m" + name + "\x1b[37m");
            } else {
                send_line(ob, "\x1b[36m" + name + " \x1b[36m" + count.to_string() + "개\x1b[37m");
            }
        }
    }
    send_line(ob, "─────────────────");
    send_line(ob, "\x1b[0m\x1b[47m\x1b[30m▶ 은전 : "
        + pad_start(view["silver"].to_string(), 20, " ") + " 개 \x1b[0m\x1b[37m\x1b[40m");
    // Python 소소.py는 관리자 대상 조회에서도 금전만큼은 조회자가 가진 값을 출력한다.
    let gold = get_int(ob, "금전");
    if gold > 0 {
        send_line(ob, "\x1b[0m\x1b[47m\x1b[30m▶ 금전 : "
            + pad_start(gold.to_string(), 20, " ") + " 개 \x1b[0m\x1b[37m\x1b[40m");
    }
    send_line(ob, "─────────────────");
}

fn main(ob, line) { cmd(ob, line); }
