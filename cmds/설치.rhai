// 설치.rhai - 방 설치
// Python: cmds/설치.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 설치");
        return;
    }
    let item = item_equip_presentation(ob, line, 1);
    let result = install_item(ob, line);
    if result == "" {
        let name = item["name"];
        let text = "\x1b[36m" + name + han_eul(name) + "\x1b[37m 설치합니다.";
        send_line(ob, "당신이 " + text);
        let own_name = get_string(ob, "이름");
        let room_text = "\x1b[33m" + own_name + "\x1b[37m" + han_iga(own_name)
            + " " + text;
        for player_name in get_room_players(ob) {
            if player_name != own_name {
                send_to_user(player_name, room_text);
            }
        }
    } else {
        send_line(ob, result);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
