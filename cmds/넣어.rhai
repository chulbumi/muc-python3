// Python cmds/넣어.py의 출력만 구성한다.

fn box_item_post(item) {
    let name_a = if item["ansi"] != "" {
        item["ansi"] + item["name"] + "\x1b[0;37m"
    } else {
        "\x1b[0;36m" + item["name"] + "\x1b[37m"
    };
    name_a + han_eul(item["particle_source"])
}

fn send_box_room(ob, message) {
    let context = get_box_room_context();
    for player in context["players"] {
        if player["id"] == context["self_id"] {
            continue;
        }
        let raw = "\r\n" + message + "\r\n";
        if player["show_prompt"] {
            raw += "\r\n\x1b[0;37;40m[ " + player["hp"].to_string() + "/"
                + player["max_hp"].to_string() + ", " + player["mp"].to_string()
                + "/" + player["max_mp"].to_string() + " ] ";
        }
        box_send_raw(ob, player["id"], raw);
    }
}

fn cmd(ob, line) {
    let words = line_args(line);
    if length(line) == 0 || len(words) < 2 {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 넣어");
        return;
    }

    let result = box_put_python(ob, line);
    let status = result["status"];
    if status == "unsupported" {
        internal_not_handled();
        return;
    }
    if status == "no_box" {
        send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네");
        return;
    }
    if status == "not_expandable" {
        send_line(ob, "☞ 더 이상 수량의 증가가 안되요. ^^");
        return;
    }
    if status == "not_enough_money" {
        send_line(ob, "☞ 돈이 모자라네요. ^^");
        return;
    }
    if status == "box_full" {
        send_line(ob, "☞ 보관함에 더 이상 넣을 수 없어요. ^^");
        return;
    }
    if status == "item_not_found" {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }
    if status == "cannot_store" {
        send_line(ob, "☞ 보관 할 수 없는 물품입니다. ^^");
        return;
    }
    if status == "nothing" {
        send_line(ob, "☞ 더이상 보관할 물건이 없어요. ^^");
        return;
    }
    if status != "ok" {
        internal_not_handled();
        return;
    }

    let box_a = "\x1b[36m" + result["box_name"] + "\x1b[37m";
    let own_name = get_body_string(ob, "이름");
    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    let room_lines = [];

    if words[1] == "은전" {
        let count = result["money"].to_string();
        send_line(ob, "당신이 " + box_a + "에 은전 " + count + "개를 보관합니다.");
        room_lines.push(actor + " " + box_a + "에 은전 " + count + "개를 보관합니다.");
    } else {
        for item in result["groups"] {
            let count = item["count"];
            if count == 1 {
                let post = box_item_post(item);
                send_line(ob, "당신이 " + box_a + "에 \x1b[36m" + post
                    + "\x1b[37m 보관합니다.");
                room_lines.push(actor + " " + box_a + "에 \x1b[36m" + post
                    + "\x1b[37m 보관합니다.");
            } else {
                let count_text = count.to_string();
                send_line(ob, "당신이 " + box_a + "에 \x1b[36m" + item["name"]
                    + "\x1b[37m " + count_text + "개를 보관합니다.");
                room_lines.push(actor + " " + box_a + "에 \x1b[36m" + item["name"]
                    + "\x1b[37m " + count_text + "개를 보관합니다.");
            }
        }
    }

    send_box_room(ob, join(room_lines, "\r\n"));
}

fn main(ob, line) {
    cmd(ob, line);
}
