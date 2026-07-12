// 분해.rhai - 아이템 분해
// Python: cmds/분해.py

fn cmd(ob, line) {
    if line != "모두" { send_line(ob, "☞ 사용법: [모두] 분해"); return; }
    let result = decompose_all_items(ob);
    if result["status"] == "no_merchant" { send_line(ob, "☞ 상인이 없어요. ^_^"); return; }
    if result["status"] == "empty" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    let own_name = get_string(ob, "이름");
    let actor = own_name + han_iga(own_name);
    let room_lines = [];
    for name in result["items"] {
        let text = "\x1b[0;36m" + name + "\x1b[0;37m 1개를 분해합니다.";
        send_line(ob, "당신이 " + text);
        room_lines.push(actor + " " + text);
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
