// Python cmds/버려.py의 출력과 같은 방 알림을 그대로 구성한다.

fn dropped_item_post(item) {
    let name_a = if item["ansi"] != "" {
        item["ansi"] + item["name"] + "\x1b[0;37m"
    } else {
        "\x1b[0;36m" + item["name"] + "\x1b[37m"
    };
    name_a + han_eul(item["particle_source"])
}

fn cmd(ob, line) {
    if length(line) == 0 {
        send_line(ob, "☞ 사용법: [아이템 이름] 버려");
        return;
    }

    if !has_drop_environment(ob) {
        send_line(ob, "☞ 아무것도 버릴수 없습니다.");
        return;
    }

    if starts_with(line, "은전") {
        send_line(ob, "☞ 은전은 버릴 수 없어요. ^^");
        return;
    }

    let all = line == "모두" || line == "전부";
    let result = drop_inventory_python(ob, line, all);
    if result["status"] == "blocked" {
        send_line(ob, "☞ 그 물건은 버릴 수 없어요. ^^");
        return;
    }
    if result["status"] == "not_found" {
        if all {
            send_line(ob, "☞ 더이상 버릴 물건이 없다네");
        } else {
            send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        }
        return;
    }

    let name = get_body_string(ob, "이름");
    let actor = "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name);
    let room_lines = [];

    for item in result["dropped"] {
        let count = item["count"];
        if count == 1 {
            let post = dropped_item_post(item);
            send_line(ob, "당신이 \x1b[36m" + post + "\x1b[37m 버립니다.");
            room_lines.push(actor + " \x1b[36m" + post + "\x1b[37m 버립니다.");
        } else {
            let count_text = count.to_string();
            send_line(ob, "당신이 \x1b[36m" + item["name"] + "\x1b[37m "
                + count_text + "개를 버립니다.");
            room_lines.push(actor + " \x1b[36m" + item["name"] + "\x1b[37m "
                + count_text + "개를 버립니다.");
        }
    }

    for item in result["failed"] {
        let count = item["count"];
        if count == 1 {
            let post = dropped_item_post(item);
            let actor_space = if all { "  " } else { " " };
            send_line(ob, "당신이 \x1b[36m" + post + "\x1b[37m" + actor_space
                + "버리자 바로 부서집니다.");
            room_lines.push(actor + " \x1b[36m" + post
                + "\x1b[37m 버리자 바로 부서집니다.");
        } else {
            let count_text = count.to_string();
            send_line(ob, "당신이 \x1b[36m" + item["name"] + "\x1b[37m "
                + count_text + "개를 버리자 바로 부서집니다.");
            room_lines.push(actor + " \x1b[36m" + item["name"] + "\x1b[37m "
                + count_text + "개를 버리자 바로 부서집니다.");
        }
    }

    let room_message = join(room_lines, "\r\n");
    for player_name in get_room_players(ob) {
        if player_name != name {
            send_to_user(player_name, room_message);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
