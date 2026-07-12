// 아이템찾기.rhai - 아이템 찾기 (관리자)
// Python: cmds/아이템찾기.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 운영자 명령: [아이템이름] 아이템찾기");
        return;
    }
    let found = 0;
    for item in get_item_catalog() {
        if item["name"].contains(line) {
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
