// 설치.rhai - 방 설치
// Python: cmds/설치.py

fn install_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string()
        + "/" + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 설치");
        return;
    }
    let item = item_equip_presentation(ob, line, 1);
    let result = install_item(ob, line);
    if result == "" {
        let text = item["post"] + " 설치합니다.";
        send_line(ob, "당신이 " + text);
        let own_name = get_string(ob, "이름");
        let room_text = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name)
            + " " + text;
        let context = get_party_context();
        for player_name in get_room_players(ob) {
            if player_name != own_name {
                let prompt = "";
                for person in context["room_players"] {
                    if person["name"] == player_name { prompt = install_prompt(person); break; }
                }
                send_raw_to_user(player_name, "\r\n" + room_text + "\r\n" + prompt);
            }
        }
    } else if result == "missing" {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
    } else if result == "invalid" {
        send_line(ob, "☞ 설치할 수 있는 것이 아닙니다. ^^");
    } else if result == "duplicate" {
        send_line(ob, "☞ 이미 설치가 되어 있습니다. ^^");
    } else {
        send_line(ob, "☞ 이곳에 설치할 허가권이 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
