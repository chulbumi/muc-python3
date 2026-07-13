// 쉬어.rhai - 쉬기. Python: cmds/쉬어.py. has_room_property(쉼금지), set_act(ob,"휴식")

fn rest_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    let pos = get_my_position(ob);
    let z = pos["zone"];
    let r = pos["room"];
    if has_room_property(z, r, "쉼금지") {
        send_line(ob, "☞ 이곳에서 운기조식하기엔 적당하지 않군요.");
        return;
    }
    let a = get_int(ob, "act");
    if a == 4 {
        send_line(ob, "☞ 벌써 쉬고 있어요. ^^");
    } else if a == 0 {
        send_line(ob, "당신이 자세를 편안히 하며 운기조식에 들어갑니다.");
        let name = get_body_string(ob, "이름");
        let actor = "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name);
        let context = get_party_context();
        for player_name in get_room_players(ob) {
            if player_name != name {
                let output = "\r\n" + actor + " 자세를 편안히 하며 운기조식에 들어갑니다.\r\n";
                for person in context["room_players"] {
                    if person["name"] == player_name { output += rest_prompt(person); break; }
                }
                send_raw_to_user(player_name, output);
            }
        }
        set_act(ob, "휴식");
    } else {
        send_line(ob, "☞ 지금 쉬기에는 좋지가 않네요. ^^");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
