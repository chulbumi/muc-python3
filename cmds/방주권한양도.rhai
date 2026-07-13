// 방주권한양도.rhai - 방주 권한 양도
// Python: cmds/방주권한양도.py

fn cmd(ob, line) {
    if get_attr(ob, "직위") != "방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법 : [대상] 방주권한양도");
        return;
    }
    let target_name = resolve_room_player_name(ob, line);
    if target_name == "" {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    let result = transfer_guild_leader(ob, target_name);
    if result == "self" { send_line(ob, "☞ 이미 당신은 방주 입니다."); }
    else if result == "wrong_guild" { send_line(ob, "☞ 당신의 소속이 아닙니다."); }
    else if result == "not_deputy" { send_line(ob, "☞ 방주권한은 부방주에게만 양도할 수 있습니다."); }
    else if result == "low_level" { send_line(ob, "☞ 방주가 되기에는 역량이 부족합니다."); }
    else if result == "ok" {
        let own_name = get_string(ob, "이름");
        let msg = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name)
            + " \x1b[1m" + target_name + "\x1b[0;37m" + han_eul(target_name)
            + " 방주로 권한이양을 선포합니다.";
        for player in get_all_online_players() {
            if player["이름"].to_string() != target_name { continue; }
            if !config_text_is_enabled(player["설정상태"].to_string(), "엘피출력") {
                send_raw_to_user(target_name, "\r\n\x1b[0;37;40m[ "
                    + player["현재체력"].to_string() + "/"
                    + player["최고체력"].to_string() + ", "
                    + player["현재내공"].to_string() + "/"
                    + player["최고내공"].to_string() + " ] ");
            }
            break;
        }
        send_python_guild_group(ob, msg);
    }
    else { send_line(ob, "☞ 이곳에 그런 무림인이 없습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
