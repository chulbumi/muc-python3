// 벗어.rhai - 장비 해제. Python: cmds/벗어.py. item_unequip, item_unequip_all, parse_order_name

fn unequip_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string()
        + "/" + person["max_mp"].to_string() + " ] "
}

fn send_unequip_room(ob, room_text) {
    let own_name = get_string(ob, "이름");
    let context = get_party_context();
    for player_name in get_room_players(ob) {
        if player_name != own_name {
            let output = "\r\n" + room_text + "\r\n";
            for person in context["room_players"] {
                if person["name"] == player_name {
                    output += unequip_prompt(person);
                    break;
                }
            }
            send_raw_to_user(player_name, output);
        }
    }
}

fn cmd(ob, line) {
    let t = trim(line);
    if t == "" {
        send_line(ob, "☞ 사용법: [아이템 이름] 해제");
        return;
    }
    if t == "모두" || t == "전부" {
        let items = item_unequip_all(ob);
        if items.len() == 0 {
            send_line(ob, "☞ 착용중인 장비가 없어요.");
            return;
        }
        let own_name = get_string(ob, "이름");
        let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
        let room_lines = [];
        for item in items {
            let item_text = "\x1b[36m" + item["post"] + "\x1b[37m 착용해제 합니다.";
            send_line(ob, "당신이 " + item_text);
            room_lines.push(actor + " " + item_text);
        }
        let room_text = join(room_lines, "\r\n");
        send_unequip_room(ob, room_text);
        return;
    }
    let arr = parse_order_name(t);
    let ord = arr[0];
    let nm = arr[1];
    let result = item_unequip(ob, nm, ord);
    if result["err"] == "usage" { send_line(ob, "☞ 사용법: [아이템 이름] 해제"); return; }
    if result["err"] == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if result["err"] != "" { return; }
    let item_text = "\x1b[36m" + result["post"] + "\x1b[37m 착용해제 합니다.";
    send_line(ob, "당신이 " + item_text);
    let own_name = get_string(ob, "이름");
    let room_text = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name) + " " + item_text;
    send_unequip_room(ob, room_text);
}

fn main(ob, line) {
    cmd(ob, line);
}
