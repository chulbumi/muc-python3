// 입어.rhai - 장비 착용. Python: cmds/입어.py. item_equip(ob,name,order), parse_order_name

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [아이템 이름] 착용");
        return;
    }
    let target = trim(line);
    if target == "모두" || target == "전부" {
        let equipped = item_equip_all(ob);
        if equipped.len() == 0 {
            send_line(ob, "☞ 더이상 착용할 장비가 없어요.");
            return;
        }
        let room_lines = [];
        let actor_name = get_string(ob, "이름");
        let actor = actor_name + han_iga(actor_name);
        for item in equipped {
            let text = if item["script"] == "" {
                "\x1b[36m" + item["name"] + han_eul(item["name"]) + "\x1b[37m 착용합니다."
            } else {
                item["script"]
            };
            send_line(ob, "당신이 " + text);
            room_lines.push(actor + " " + text);
        }
        let room_text = join(room_lines, "\r\n");
        for player_name in get_room_players(ob) {
            if player_name != actor_name {
                send_to_user(player_name, room_text);
            }
        }
        return;
    }
    let arr = parse_order_name(line);
    let ord = arr[0];
    let nm = arr[1];
    let err = item_equip(ob, nm, ord);
    if err == "usage" { send_line(ob, "☞ 사용법: [아이템 이름] 착용"); return; }
    if err == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if err == "not_equippable" { send_line(ob, "☞ 착용할 수 있는것이 아니에요."); return; }
    if err == "slot_used" { send_line(ob, "☞ 더 이상 착용이 불가능해요."); return; }
    if err != "" { return; }
    let eul = han_eul(nm);
    send_line(ob, "당신이 \x1b[36m" + nm + eul + "\x1b[37m 착용합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
