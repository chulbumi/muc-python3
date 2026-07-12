// 소켓.rhai - 아이템 소켓 시스템
// Python: cmds/소켓.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let msg = "";
    let old = "";
    for entry in get_online_socket_entries() {
        if entry["name"] == "" { continue; }
        if old != entry["host"] {
            msg += "\r\n" + pad_start(entry["host"], 16, " ") + " : " + entry["name"];
        } else {
            msg += ", " + entry["name"];
        }
        old = entry["host"];
    }
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
