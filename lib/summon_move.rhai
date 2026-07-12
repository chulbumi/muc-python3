// Shared presentation/state orchestration for Python enterRoom(..., "소환", "소환").
fn perform_summon_move(ob, zone, room) {
    let old_players = get_room_players(ob);
    let error = set_my_position(ob, zone, room);
    if error != "" { return error; }
    clear_summon_combat(ob);
    let name = get_body_string(ob, "이름");
    let actor = "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name);
    let visible = get_int(ob, "투명상태") != 1;
    if visible {
        send_line(ob, "당신이 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'");
        for player in old_players {
            if player != name { send_to_user(player, actor + " 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'"); }
        }
    }
    let view = view_map_data(ob);
    if view["ok"] { for row in split(view["text"], "\r\n") { send_line(ob, row); } }
    if visible {
        for player in get_room_players(ob) {
            if player != name { send_to_user(player, actor + " 알수 없는 기운에 감싸여 나타납니다. '고오오오~~~'"); }
        }
    }
    finish_entry_aggression(ob);
    ""
}
