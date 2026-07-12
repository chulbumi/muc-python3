// 가져.rhai - 바닥에서 아이템 줍기 (Python: cmds/가져.py)
fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [아이템 이름] 주워"); return; }
    let target = trim(line);
    if target == "모두" || target == "전부" {
        let picked = item_get_all(ob);
        if picked.len() == 0 {
            send_line(ob, "☞ 더이상 가질 물건이 없다네");
            return;
        }
        let actor_name = get_string(ob, "이름");
        let actor = actor_name + han_iga(actor_name);
        let room_lines = [];
        for item in picked {
            let name = item["name"];
            let count = item["count"];
            let text = if count == 1 {
                "\x1b[36m" + name + han_eul(name)
                    + "\x1b[37m 집어서 품속에 갈무리 합니다."
            } else {
                "\x1b[36m" + name + "\x1b[37m " + count.to_string()
                    + "개를 집어서 품속에 갈무리 합니다."
            };
            send_line(ob, "당신이 " + text);
            room_lines.push(actor + " " + text);
        }
        let room_text = join(room_lines, "\r\n");
        for player_name in get_room_players(ob) {
            if player_name != actor_name {
                send_to_user(player_name, room_text);
            }
        }
        return;
    }
    let args = line_args(line);
    let count = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    let count = if count < 1 { 1 } else { if count > 100 { 50 } else { count } };
    let name = args[0];
    let picked = item_get(ob, name, count);
    if picked == 0 {
        send_line(ob, "☞ 강호에 그런 물건은 존재하지 않는다네");
        return;
    }
    send_line(ob, format_item_action_self(name, "집어서 품속에 갈무리 합니다.", picked));
}

fn main(ob, line) {
    cmd(ob, line);
}
