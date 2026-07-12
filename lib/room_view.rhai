// Shared Python Player.viewMapData presentation used by movement and death entry.
// Rust exposes only the room state through movement_room_view(); formatting stays here.

fn has_exit(exits, name) {
    for exit_name in exits {
        if exit_name == name {
            return true;
        }
    }
    false
}

fn long_exits(exits) {
    if exits.len() == 0 {
        return "\r\n  ○  어느 쪽으로도 이동할 수 없습니다.\r\n";
    }

    let names = "";
    for exit_name in exits {
        if names != "" {
            names += "ː";
        }
        names += "\x1b[32m" + exit_name + "\x1b[37m";
    }

    let out = "";
    if has_exit(exits, "북서") { out += "\x1b[32m↖\x1b[37m"; } else { out += "  "; }
    if has_exit(exits, "북") { out += "\x1b[32m△\x1b[37m"; } else { out += "  "; }
    if has_exit(exits, "북동") { out += "\x1b[32m↗\x1b[37m\r\n"; } else { out += "\r\n"; }
    if has_exit(exits, "서") { out += "\x1b[32m◁\x1b[37m"; } else { out += "  "; }
    out += "○";
    if has_exit(exits, "동") { out += "\x1b[32m▷\x1b[37m"; } else { out += "  "; }
    out += " 〔" + names + "〕쪽으로 이동할 수 있습니다.\r\n";
    if has_exit(exits, "남서") { out += "\x1b[32m↙\x1b[37m"; } else { out += "  "; }
    if has_exit(exits, "남") { out += "\x1b[32m▽\x1b[37m"; } else { out += "  "; }
    if has_exit(exits, "남동") { out += "\x1b[32m↘\x1b[37m"; } else { out += "  "; }
    out
}

fn short_exits(exits) {
    let names = "";
    for exit_name in exits {
        names += exit_name + " ";
    }
    if names == "" {
        names = "없음";
    }
    "\r\n[출구] : " + names
}

fn mob_actor(name) {
    "\x1b[33m" + name + "\x1b[37m" + han_iga(name)
}

fn emit_room_view(ob) {
    let view = movement_room_view(ob);
    if !view["ok"] {
        return;
    }

    let header = "\r\n\x1b[1;30m[\x1b[0;37m[[\x1b[1;37m[]\x1b[1m "
        + view["name"]
        + " \x1b[1;37m[]\x1b[0;37m]]\x1b[1;30m]\x1b[0;37m";
    if get_int(ob, "관리자등급") >= 1000 {
        header += " (" + view["zone"] + ":" + view["room"] + ")";
    }
    send_line(ob, header);

    if get_user_config(ob, "간략설명") != "1" {
        send_line(ob, "");
        let description = "";
        for line in view["description"] {
            if description != "" {
                description += "\r\n";
            }
            description += line;
        }
        send_line(ob, description);
    }

    if get_user_config(ob, "나침반제거") != "1" {
        send_line(ob, long_exits(view["exits"]));
    } else {
        send_line(ob, short_exits(view["exits"]));
        send_line(ob, "");
    }

    let box_line = "";
    for box_view in view["boxes"] {
        box_line += box_view + "    ";
    }
    if box_line != "" {
        send_line(ob, "☞ " + box_line);
    }

    for mob in view["mobs"] {
        if mob["mob_type"] == 7 || mob["act"] == 3 {
            continue;
        }
        let name = mob["name"];
        if mob["act"] == 4 {
            send_line(ob, mob_actor(name) + " 흐트러진 진기를 추스리고 있습니다.");
        } else if mob["act"] == 0 {
            send_line(ob, mob["desc1"]);
        } else if mob["act"] == 1 {
            let line = "";
            for head in mob["defense_heads"] {
                line += head + " ";
            }
            line += mob_actor(name) + " 목숨을 건 사투를 벌이고 있습니다.";
            send_line(ob, line);
        } else if mob["act"] == 2 {
            send_line(ob, "\x1b[33m" + name + "\x1b[37m의 싸늘한 시체가 있습니다.");
        }
    }

    let own_name = get_string(ob, "이름");
    for player in view["players"] {
        if player["transparent"] || player["name"] == own_name {
            continue;
        }
        let line = "";
        if player["guild_title"] != "" {
            line += "\x1b[1m【" + player["guild_title"] + "】\x1b[0m";
        }
        for head in player["defense_heads"] {
            line += head + " ";
        }
        if player["head"] != "" {
            line += player["head"] + " ";
        }
        let name = player["name"];
        line += "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name) + " ";
        if player["tail"] != "" {
            line += player["tail"] + " ";
        }
        if player["act"] == 0 {
            line += "서 있습니다.";
        } else if player["act"] == 4 {
            line += "운기조식을 하고 있습니다.";
        } else if player["act"] == 1 {
            line += "목숨을 건 사투를 벌이고 있습니다.";
        } else if player["act"] == 2 {
            line += "쓰러져 있습니다.";
        }
        send_line(ob, line);
    }
}
