// Python cmds/방파입문.py. State transitions live in guild_accept_member;
// every visible byte remains here.

fn guild_accept_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    let position = get_string(ob, "직위");
    if position != "방주" && position != "부방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법 : [대상] 방파입문");
        return;
    }
    let target_name = resolve_room_player_name(ob, line);
    if target_name == "" {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    let own_name = get_string(ob, "이름");
    if target_name == own_name {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }
    let status = guild_accept_member(ob, target_name);
    if status == "not_requested" {
        send_line(ob, "☞ 방파를 신청한 그런 무림인이 없습니다.");
        return;
    }
    if status != "ok" { return; }

    let target_a = "\x1b[1m" + target_name + "\x1b[0;37m";
    let actor_a = "\x1b[1m" + own_name + "\x1b[0;37m";
    send_line(ob, "당신이 " + target_a + han_eul(target_name) + " 방파에 입문시켰음을 선포합니다.");
    let recipient = "\r\n" + actor_a + han_iga(own_name)
        + " 당신을 방파에 입문시켰음을 선포합니다.\r\n";
    let context = get_party_context();
    for player in context["room_players"] {
        if player["name"] == target_name {
            recipient += guild_accept_prompt(player);
            break;
        }
    }
    send_raw_to_user(target_name, recipient);
}

fn main(ob, line) { cmd(ob, line); }
