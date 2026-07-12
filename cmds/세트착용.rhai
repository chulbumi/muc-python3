// 세트착용.rhai - 장비 세트 착용
// Python: cmds/세트착용.py

fn cmd(ob, line) {
    let set_name = get_string(ob, "세트기억");
    if set_name == "" {
        send_line(ob, "☞ 사용법: [세트기억] 후 [세트착용]");
        return;
    }
    let count = 0;
    for item_name in get_saved_set_items(ob, set_name) {
        if item_equip(ob, item_name, 0) == "" { count += 1; }
    }
    if count == 0 { send_line(ob, "☞ 더이상 착용할 장비가 없어요."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
