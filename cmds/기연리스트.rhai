// 기연리스트.rhai - Python: cmds/기연리스트.py
// oneitem_list_index_entries() → #[이름]\r\n:[index]\r\n\r\n. (Item.isOneItem 필터는 추후 적용) 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
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
