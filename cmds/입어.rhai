// 입어.rhai - 장비 착용. Python: cmds/입어.py. item_equip(ob,name,order), parse_order_name

fn equip_item_text(item) {
    if item["script"] == "" {
        return item["post"] + " 착용합니다.";
    }
    item["script"]
}

fn equip_room_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn send_equip_room(ob, room_lines) {
    if room_lines.len() == 0 { return; }
    let own_name = get_string(ob, "이름");
    let room_text = join(room_lines, "\r\n");
    let context = get_party_context();
    for player_name in get_room_players(ob) {
        if player_name != own_name {
            let output = "\r\n" + room_text + "\r\n";
            for person in context["room_players"] {
                if person["name"] == player_name { output += equip_room_prompt(person); break; }
            }
            send_raw_to_user(player_name, output);
        }
    }
}

fn cmd(ob, line) {
    let target = trim(line);
    if target == "" {
        send_line(ob, "☞ 사용법: [아이템 이름] 착용");
        return;
    }
    if target == "모두" || target == "전부" {
        let equipped = item_equip_all(ob);
        if equipped.len() == 0 {
            send_line(ob, "☞ 더이상 착용할 장비가 없어요.");
            return;
        }
        let room_lines = [];
        let actor_name = get_string(ob, "이름");
        let actor = "\x1b[1m" + actor_name + "\x1b[0;37m" + han_iga(actor_name);
        for item in equipped {
            let text = equip_item_text(item);
            send_line(ob, "당신이 " + text);
            room_lines.push(actor + " " + text);
        }
        send_equip_room(ob, room_lines);
        return;
    }
    let arr = parse_order_name(target);
    let ord = arr[0];
    let nm = arr[1];
    let item = item_equip_presentation(ob, nm, ord);
    let err = item_equip(ob, nm, ord);
    if err == "usage" { send_line(ob, "☞ 사용법: [아이템 이름] 착용"); return; }
    if err == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if err == "not_equippable" { send_line(ob, "☞ 착용할 수 있는것이 아니에요."); return; }
    if err == "mastery_required" { send_line(ob, "☞ 당신의 능력으로는 착용이 불가능해요."); return; }
    if err == "slot_used" { send_line(ob, "☞ 더 이상 착용이 불가능해요."); return; }
    if err != "" { return; }
    let text = equip_item_text(item);
    send_line(ob, "당신이 " + text);
    let actor_name = get_string(ob, "이름");
    send_equip_room(ob, ["\x1b[1m" + actor_name + "\x1b[0;37m"
        + han_iga(actor_name) + " " + text]);
}

fn main(ob, line) {
    cmd(ob, line);
}
