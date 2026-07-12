// 분해.rhai - 아이템 분해
// Python: cmds/분해.py

fn decompose_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    if line != "모두" { send_line(ob, "☞ 사용법: [모두] 분해"); return; }
    let result = decompose_all_items(ob);
    if result["status"] == "no_merchant" { send_line(ob, "☞ 상인이 없어요. ^_^"); return; }
    if result["status"] == "empty" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    let own_name = get_string(ob, "이름");
    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    let context = get_party_context();
    for item in result["items"] {
        let name_a = if item["안시"] != "" {
            item["안시"] + item["이름"] + "\x1b[0;37m"
        } else {
            "\x1b[0;36m" + item["이름"] + "\x1b[37m"
        };
        let text = name_a + " 1개를 분해합니다.";
        send_line(ob, "당신이 " + text);
        let room_text = actor + " " + text;
        for player_name in get_room_players(ob) {
            if player_name != own_name {
                let output = "\r\n" + room_text + "\r\n";
                for person in context["room_players"] {
                    if person["name"] == player_name { output += decompose_prompt(person); break; }
                }
                send_to_user(player_name, output);
            }
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
