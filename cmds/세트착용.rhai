// 세트착용.rhai - 장비 세트 착용
// Python: cmds/세트착용.py

fn set_wear_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    let set_name = get_string(ob, "세트기억");
    if set_name == "" {
        send_line(ob, "☞ 사용법: [세트기억] 후 [세트착용]");
        return;
    }
    let count = 0;
    let room_lines = [];
    let own_name = get_string(ob, "이름");
    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    for saved in get_saved_set_items(ob, set_name) {
        let item = item_equip_presentation(ob, saved["name"], saved["order"]);
        if item_equip(ob, saved["name"], saved["order"]) == "" {
            let text = if item["script"] == "" {
                "\x1b[36m" + item["post"] + "\x1b[37m 착용합니다."
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
    let context = get_party_context();
    for player_name in get_room_players(ob) {
        if player_name != own_name {
            let output = "\r\n" + room_text + "\r\n";
            for person in context["room_players"] {
                if person["name"] == player_name { output += set_wear_prompt(person); break; }
            }
            send_raw_to_user(player_name, output);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
