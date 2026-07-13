// 일어나.rhai - 일어나기. Python: cmds/일어나.py. set_act(ob,"서")

fn stand_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    let a = get_int(ob, "act");
    if a == 4 {
        send_line(ob, "당신이 운기조식을 마치며 벌떡 일어섭니다.");
        let name = get_body_string(ob, "이름");
        let actor = "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name);
        let context = get_party_context();
        for player_name in get_room_players(ob) {
            if player_name != name {
                let output = "\r\n" + actor + " 운기조식을 마치며 벌떡 일어섭니다.\r\n";
                for person in context["room_players"] {
                    if person["name"] == player_name { output += stand_prompt(person); break; }
                }
                send_raw_to_user(player_name, output);
            }
        }
        set_act(ob, "서");
    } else if a == 0 {
        send_line(ob, "☞ 이미 서있는 상태입니다. ^^");
    } else {
        send_line(ob, "☞ 운기조식중인 상태가 아닙니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
