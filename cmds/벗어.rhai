// 벗어.rhai - 장비 해제. Python: cmds/벗어.py. item_unequip, item_unequip_all, parse_order_name

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [아이템 이름] 해제");
        return;
    }
    let t = trim(line);
    if t == "모두" || t == "전부" {
        let items = item_unequip_all(ob);
        if items.len() == 0 {
            send_line(ob, "☞ 착용중인 장비가 없어요.");
            return;
        }
        let own_name = get_string(ob, "이름");
        let actor = own_name + han_iga(own_name);
        let room_lines = [];
        for name in items {
            let item_text = "\x1b[36m" + name + han_eul(name) + "\x1b[37m 착용해제 합니다.";
            send_line(ob, "당신이 " + item_text);
            room_lines.push(actor + " " + item_text);
        }
        let room_text = join(room_lines, "\r\n");
        for player_name in get_room_players(ob) {
            if player_name != own_name {
                send_to_user(player_name, room_text);
            }
        }
        return;
    }
    let arr = parse_order_name(line);
    let ord = arr[0];
    let nm = arr[1];
    let err = item_unequip(ob, nm, ord);
    if err == "usage" { send_line(ob, "☞ 사용법: [아이템 이름] 해제"); return; }
    if err == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if err != "" { return; }
    let eul = han_eul(nm);
    send_line(ob, "당신이 \x1b[36m" + nm + eul + "\x1b[37m 착용해제 합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
