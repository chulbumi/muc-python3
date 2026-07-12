// 기연리스트.rhai - Python: cmds/기연리스트.py
// oneitem_list_index_entries() → #[이름]\r\n:[index]\r\n\r\n. 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let arr = oneitem_list_index_entries();
    let lines = [];
    for e in arr {
        let n = e["name"].to_string();
        let i = e["index"].to_string();
        lines.push("#" + n + "\r\n:" + i + "\r\n\r\n");
    }
    let msg = "[단일아이템인덱스]\r\n" + join(lines, "");
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
