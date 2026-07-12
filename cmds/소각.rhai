// Python `cmds/소각.py`: 선택된 소지품을 제거하고 같은 방에 알린다.

fn destroy_room_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [물품이름] 소각"); return; }
    let args = line_args(line);
    let count = if len(args) >= 2 { python_get_int(args[1]) } else { 1 };
    let count = if count < 1 { 1 } else { if count > 100 { 100 } else { count } };
    let parsed = parse_order_name(args[0]);
    let order = parsed[0];
    let requested = if order != 1 { 1 } else { count };
    let result = item_destroy_detail(ob, parsed[1], order, requested);
    if result["status"] != "ok" {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }
    let name = result["name"].to_string();
    let removed = result["count"].to_int();
    let item_text = if removed == 1 {
        "\x1b[36m" + result["post"] + "\x1b[37m"
    } else {
        "\x1b[36m" + name + "\x1b[37m " + removed.to_string() + "개를"
    };
    send_line(ob, "당신이 " + item_text + " 소각해버립니다.");
    let own_name = get_string(ob, "이름");
    let room_text = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name)
        + " " + item_text + " 소각해버립니다.";
    let context = get_party_context();
    for player in get_room_players(ob) {
        if player != own_name {
            let output = "\r\n" + room_text + "\r\n";
            for person in context["room_players"] {
                if person["name"] == player { output += destroy_room_prompt(person); break; }
            }
            send_to_user(player, output);
        }
    }
}

fn main(ob, line) { cmd(ob, line); }
