// 가져.rhai - 바닥에서 아이템 줍기 (Python: cmds/가져.py)
fn pickup_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string()
        + "/" + person["max_mp"].to_string() + " ] "
}

fn send_pickup_room(ob, own_name, room_text) {
    let context = get_party_context();
    for player_name in get_room_players(ob) {
        if player_name != own_name {
            let prompt = "";
            for person in context["room_players"] {
                if person["name"] == player_name { prompt = pickup_prompt(person); break; }
            }
            send_raw_to_user(player_name, "\r\n" + room_text + "\r\n" + prompt);
        }
    }
}

fn cmd(ob, line) {
    let target = trim(line);
    if target == "" { send_line(ob, "☞ 사용법: [아이템 이름] 주워"); return; }
    if target == "모두" || target == "전부" {
        let picked = item_get_all(ob);
        if picked.len() == 0 {
            send_line(ob, "☞ 더이상 가질 물건이 없다네");
            return;
        }
        let actor_name = get_string(ob, "이름");
        let actor = "\x1b[1m" + actor_name + "\x1b[0;37m" + han_iga(actor_name);
        let room_lines = [];
        for item in picked {
            let name = item["name"];
            let count = item["count"];
            let text = if count == 1 {
                "\x1b[36m" + item["post"]
                    + "\x1b[37m 집어서 품속에 갈무리 합니다."
            } else {
                "\x1b[36m" + name + "\x1b[37m " + count.to_string()
                    + "개를 집어서 품속에 갈무리 합니다."
            };
            send_line(ob, "당신이 " + text);
            room_lines.push(actor + " " + text);
        }
        let room_text = join(room_lines, "\r\n");
        send_pickup_room(ob, actor_name, room_text);
        return;
    }
    let args = line_args(target);
    let count = if len(args) >= 2 { parse_int_prefix(args[1]) } else { 1 };
    let count = if count < 1 { 0 } else { if count > 100 { 50 } else { count } };
    let name = args[0];
    let result = item_get_detail(ob, name, count);
    if result["status"] == "too_heavy" {
        send_line(ob, "☞ 자네의 힘으로는 더이상 가질 수 없다네");
        return;
    }
    if result["status"] == "inv_full" {
        send_line(ob, "☞ 자네가 가질 물품의 한계라네");
        return;
    }
    if result["status"] != "ok" {
        send_line(ob, "☞ 강호에 그런 물건은 존재하지 않는다네");
        return;
    }
    let own_name = get_string(ob, "이름");
    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    let room_lines = [];
    for item in result["groups"] {
        let text = if item["count"] == 1 {
            "\x1b[36m" + item["post"] + "\x1b[37m 집어서 품속에 갈무리 합니다."
        } else {
            "\x1b[36m" + item["name"] + "\x1b[37m " + item["count"].to_string()
                + "개를 집어서 품속에 갈무리 합니다."
        };
        send_line(ob, "당신이 " + text);
        room_lines.push(actor + " " + text);
    }
    let room_text = join(room_lines, "\r\n");
    send_pickup_room(ob, own_name, room_text);
}

fn main(ob, line) {
    cmd(ob, line);
}
