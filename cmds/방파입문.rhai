// 방파입문.rhai - 방파 입문 (방주가 대상을 방파에 입문시킴)
// Python: cmds/방파입문.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법 : [대상] 방파입문");
        return;
    }

    let my_position = get_attr(ob, "직위");
    if my_position != "방주" && my_position != "부방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }

    let my_name = get_attr(ob, "이름");
    if my_name == line {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }

    // 현재 방에 있는 플레이어 목록에서 대상 찾기
    let players = get_room_players(ob);
    let target_found = false;
    let i = 0;
    while i < len(players) {
        if players[i] == line {
            target_found = true;
            break;
        }
        i = i + 1;
    }

    if !target_found {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }

    let my_guild = get_attr(ob, "소속");
    if my_guild == "" {
        send_line(ob, "☞ 당신은 방파에 소속되어 있지 않습니다.");
        return;
    }

    let requested = get_room_player_attr(ob, line, "입문신청자");
    if requested.is_unit() || requested.to_string() != my_name {
        send_line(ob, "☞ " + line + " 님은 방파 입문을 신청하지 않았습니다.");
        return;
    }
    if !guild_add_member(ob, line) {
        send_line(ob, "☞ 이미 방파원이거나 입문 처리에 실패했습니다.");
        return;
    }
    set_room_player_attr(ob, line, "소속", my_guild);
    set_room_player_attr(ob, line, "직위", "제자");
    set_room_player_attr(ob, line, "입문신청자", "");
    send_line(ob, "☞ " + line + " 님을 " + my_guild + "에 입문시켰습니다.");
    send_to_user(line, "☞ " + my_guild + " 방파에 입문했습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
