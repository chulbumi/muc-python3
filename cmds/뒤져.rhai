// Python `cmds/뒤져.py`. State and item movement are supplied by efuns;
// every visible string remains hot-reloadable here.

fn empty_search(ob, name, dead) {
    if dead {
        send_line(ob, "당신이 " + name + "의 시체를 뒤집니다. '뒤적~ 뒤적~'");
        send_room(ob, get_string(ob, "이름") + han_iga(get_string(ob, "이름")) + " " + name + "의 시체를 뒤집니다. '뒤적~ 뒤적~'");
    } else {
        send_line(ob, "당신이 " + name + han_eul(name) + " 뒤집니다. '뒤적~ 뒤적~'");
        send_room(ob, get_string(ob, "이름") + han_iga(get_string(ob, "이름")) + " " + name + han_eul(name) + " 뒤집니다. '뒤적~ 뒤적~'");
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
        send_line(ob, "당신이 " + name + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        send_room(ob, get_string(ob, "이름") + han_iga(get_string(ob, "이름")) + " " + name + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        return;
    }

    let result = search_room_mob(ob, line);
    if result["status"] != "ok" {
        send_line(ob, "☞ 뒤질대상이 없어요. ^^");
        return;
    }
    let name = result["name"];
    if !result["searchable"] {
        send_line(ob, "당신이 " + name + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        send_room(ob, get_string(ob, "이름") + han_iga(get_string(ob, "이름")) + " " + name + "의 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
        return;
    }
    let items = result["items"];
    if len(items) == 0 {
        empty_search(ob, name, result["dead"]);
        return;
    }
    let room_lines = [];
    for item in items {
        if result["dead"] {
            send_line(ob, "당신이 " + name + "의 시체속에서 " + item + han_eul(item) + " 뒤져서 가집니다.");
            room_lines.push(get_string(ob, "이름") + han_iga(get_string(ob, "이름")) + " " + name + "의 시체속에서 " + item + han_eul(item) + " 뒤져서 가집니다.");
        } else {
            send_line(ob, "당신이 " + name + "에게서 " + item + han_eul(item) + " 뒤져서 가집니다.");
            room_lines.push(get_string(ob, "이름") + han_iga(get_string(ob, "이름")) + " " + name + "에게서 " + item + han_eul(item) + " 뒤져서 가집니다.");
        }
    }
    send_room(ob, join(room_lines, "\r\n"));
}

fn main(ob, line) { cmd(ob, line); }
