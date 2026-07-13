// 올숙리스트.rhai - 모든 아이템 목록 조회
// Python: cmds/올숙리스트.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let msg = "[올숙아이템인덱스]\r\n";
    for item in get_item_catalog() {
        if item["user_enabled"] {
            msg += "#" + item["name"] + "\r\n:" + item["index"] + "\r\n\r\n";
        }
    }
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
