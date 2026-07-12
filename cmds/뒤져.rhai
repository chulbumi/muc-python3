// Python `cmds/뒤져.py`. State and item movement are supplied by efuns;
// every visible string remains hot-reloadable here.

fn empty_search(ob, name, dead) {
    let actor = "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m" + han_iga(get_string(ob, "이름"));
    let mob = "\x1b[33m" + name + "\x1b[37m";
    if dead {
        send_line(ob, "당신이 " + mob + "의 시체를 뒤집니다. '뒤적~ 뒤적~'");
        send_room(ob, actor + " " + mob + "의 시체를 뒤집니다. '뒤적~ 뒤적~'");
    } else {
        send_line(ob, "당신이 " + mob + han_eul(name) + " 뒤집니다. '뒤적~ 뒤적~'");
        send_room(ob, actor + " " + mob + han_eul(name) + " 뒤집니다. '뒤적~ 뒤적~'");
    }
}

fn searched_item_a(item) {
    if item["안시"] != "" {
        item["안시"] + item["이름"] + "\x1b[0;37m"
    } else {
        "\x1b[0;36m" + item["이름"] + "\x1b[37m"
    }
}

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 뒤져");
        return;
    }

    // Python Room.findObjName can select a player. Players are only touched;
    // their inventory is never transferred.
    let player = find_target(ob, line);
    if player["found"] == true {
        let name = player["to_target"]["name"];
        let target_a = "\x1b[1m" + name + "\x1b[0;37m";
        let actor = "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m" + han_iga(get_string(ob, "이름"));
        send_line(ob, "당신이 " + target_a + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        send_room(ob, actor + " " + target_a + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        return;
    }

    let result = search_room_mob(ob, line);
    if result["status"] != "ok" {
        send_line(ob, "☞ 뒤질대상이 없어요. ^^");
        return;
    }
    let name = result["name"];
    let mob = "\x1b[33m" + name + "\x1b[37m";
    let actor = "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m" + han_iga(get_string(ob, "이름"));
    if !result["searchable"] {
        send_line(ob, "당신이 " + mob + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        send_room(ob, actor + " " + mob + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        return;
    }
    let items = result["items"];
    if len(items) == 0 {
        empty_search(ob, name, result["dead"]);
        return;
    }
    let room_lines = [];
    for item in items {
        let item_a = searched_item_a(item);
        let item_name = item["이름"];
        if result["dead"] {
            send_line(ob, "당신이 " + mob + "의 시체속에서 " + item_a + han_eul(item_name) + " 뒤져서 가집니다.");
            room_lines.push(actor + " " + mob + "의 시체속에서 " + item_a + han_eul(item_name) + " 뒤져서 가집니다.");
        } else {
            send_line(ob, "당신이 " + mob + "에게서 " + item_a + han_eul(item_name) + " 뒤져서 가집니다.");
            room_lines.push(actor + " " + mob + "에게서 " + item_a + han_eul(item_name) + " 뒤져서 가집니다.");
        }
    }
    send_room(ob, join(room_lines, "\r\n"));
}

fn main(ob, line) { cmd(ob, line); }
