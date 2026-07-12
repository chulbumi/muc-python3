// 일어나.rhai - 일어나기. Python: cmds/일어나.py. set_act(ob,"서")

fn cmd(ob, line) {
    let a = get_int(ob, "act");
    if a == 4 {
        send_line(ob, "당신이 운기조식을 마치며 벌떡 일어섭니다.");
        let name = get_body_string(ob, "이름");
        let actor = "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name);
        for player_name in get_room_players(ob) {
            if player_name != name {
                send_to_user(player_name, actor + " 운기조식을 마치며 벌떡 일어섭니다.");
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
