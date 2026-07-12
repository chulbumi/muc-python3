// 방어구찾기.rhai - 방어구 찾기 (관리자)
// Python: cmds/방어구찾기.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let found = 0;
    for item in get_item_catalog() {
        if item["type"] == "방어구" {
            let name_a = if item["ansi"] != "" {
                item["ansi"] + item["name"] + "\x1b[0;37m"
            } else {
                "\x1b[0;36m" + item["name"] + "\x1b[37m"
            };
            send_line(ob, name_a + " : " + item["index"]);
            found += 1;
        }
    }
    if found == 0 { send_line(ob, "☞ 찾으시는 아이템이 없습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
