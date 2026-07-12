// 세트착용.rhai - 장비 세트 착용
// Python: cmds/세트착용.py

fn cmd(ob, line) {
    let set_name = get_string(ob, "세트기억");
    if set_name == "" {
        send_line(ob, "☞ 사용법: [세트기억] 후 [세트착용]");
        return;
    }
    let count = 0;
    let room_lines = [];
    let own_name = get_string(ob, "이름");
    let actor = "\x1b[33m" + own_name + "\x1b[37m" + han_iga(own_name);
    for item_name in get_saved_set_items(ob, set_name) {
        let item = item_equip_presentation(ob, item_name, 1);
        if item_equip(ob, item_name, 1) == "" {
            let text = if item["script"] == "" {
                "\x1b[36m" + item["name"] + han_eul(item["name"])
                    + "\x1b[37m 착용합니다."
            } else {
                item["script"]
            };
            send_line(ob, "당신이 " + text);
            room_lines.push(actor + " " + text);
            count += 1;
        }
    }
    if count == 0 {
        send_line(ob, "☞ 더이상 착용할 장비가 없어요.");
        return;
    }
    let room_text = join(room_lines, "\r\n");
    for player_name in get_room_players(ob) {
        if player_name != own_name {
            send_to_user(player_name, room_text);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
