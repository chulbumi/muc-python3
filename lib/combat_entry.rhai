// Presentation for Python Player.enterRoom aggressive-mob setFight(..., True).

fn entry_mob_start(name, script) {
    "\x1b[33m" + name + "\x1b[37m" + han_iga(name) + " " + script
}

fn entry_skill_text(line, attacker, target, weapon) {
    let text = return_replace(line, "[공]", attacker);
    text = return_replace(text, "[방]", target);
    text = return_replace(text, "[무]", weapon);
    text = post_position_once(text);
    text = post_position_once(text);
    post_position_once(text)
}

fn corpse_update_text(corpse) {
    let mob = "\x1b[33m" + corpse["mob"] + "\x1b[37m";
    let output = "\r\n" + mob + "의 시체가 무림지존의 손에 이끌려 망자의 강을 건너갑니다.";
    if corpse["items"].len > 0 {
        output += "\r\n";
        for item in corpse["items"] {
            let item_name = if item["ansi"] != "" {
                item["ansi"] + item["name"] + "\x1b[0;37m"
            } else {
                "\x1b[0;36m" + item["name"] + "\x1b[37m"
            };
            output += "\r\n" + mob + "의 시체속에서 " + item_name
                + han_iga(item["name"]) + " 모습을 드러냅니다.";
        }
    }
    output
}

fn send_entry_room_updates(result) {
    let expired = result["room_expired_items"];
    while expired.len > 0 {
        let name = expired[0];
        let count = 0;
        for candidate in expired {
            if candidate == name { count += 1; }
        }
        let remaining = [];
        for candidate in expired {
            if candidate != name { remaining.push(candidate); }
        }
        expired = remaining;
        let line = if count == 1 {
            name + han_iga(name) + " 먼지가 되어 사라집니다."
        } else {
            name + " " + count + "개가 먼지가 되어 사라집니다."
        };
        for player_name in result["room_update_players"] {
            if player_name != "" { send_to_user(player_name, "\r\n" + line); }
        }
    }
    for message in result["room_update_messages"] {
        for player_name in result["room_update_players"] {
            if player_name != "" { send_to_user(player_name, "\r\n" + message); }
        }
    }
    for corpse in result["room_corpse_updates"] {
        let output = corpse_update_text(corpse);
        for player_name in result["room_update_players"] {
            if player_name != "" { send_to_user(player_name, output); }
        }
    }
}

fn finish_entry_auto_skill(ob) {
    let skill = start_automatic_combat_skill(ob);
    if skill["status"] == "mp_fail" {
        send_line(ob, "\x1b[1m당신이 내공진기를 끌어 모으지만 기가 흩어져 버립니다.\x1b[0;37m");
        return false;
    }
    if skill["status"] == "hp_fail" {
        send_line(ob, "\x1b[1m당신의 내공진기가 흩어지며 기의 순환이 멈추어 버립니다.\x1b[0;37m");
        return false;
    }
    if skill["status"] != "ok" {
        return false;
    }
    if get_user_config(ob, "엘피출력") != "1" {
        let vitals = return_vitals(ob);
        send_line(ob, "\x1b[0;37;40m[ " + return_int_string(vitals["hp"]) + "/"
            + return_int_string(vitals["max_hp"]) + ", "
            + return_int_string(vitals["mp"]) + "/"
            + return_int_string(vitals["max_mp"]) + " ] ");
    }
    let weapon_data = get_cast_weapon(ob);
    let weapon = if weapon_data["fist"] {
        "\x1b[36m주먹\x1b[37m"
    } else {
        weapon_data["ansi"] + weapon_data["name"] + "\x1b[0;37m"
    };
    let target = "\x1b[33m" + skill["target_name"] + "\x1b[37m";
    let self_text = entry_skill_text(skill["script"], "당신", target, weapon);
    send_line(ob, "\r\n" + self_text);
    let own_name = get_string(ob, "이름");
    let actor = "\x1b[1m" + own_name + "\x1b[0;37m";
    let room_text = entry_skill_text(skill["script"], actor, target, weapon);
    for player in get_cast_room_players(ob) {
        if player["name"] != own_name && !player["reject_fight"] {
            send_to_user(player["name"], room_text);
        }
    }
    true
}

fn finish_entry_aggression(ob) {
    let result = start_entry_aggression(ob);
    if result["status"] != "ok" {
        return false;
    }
    if get_user_config(ob, "엘피출력") != "1" {
        let vitals = return_vitals(ob);
        send_line(ob, "\x1b[0;37;40m[ " + return_int_string(vitals["hp"]) + "/"
            + return_int_string(vitals["max_hp"]) + ", "
            + return_int_string(vitals["mp"]) + "/"
            + return_int_string(vitals["max_mp"]) + " ] ");
    }
    let room_lines = [];
    let target_start = entry_mob_start(result["target_name"], result["target_start"]);
    send_line(ob, "\r\n" + target_start);
    room_lines.push("\r\n" + target_start);
    if result["caster_was_stand"] {
        let weapon = get_cast_weapon(ob);
        send_line(ob, "당신이 " + weapon["fight_start"]);
        let name = get_string(ob, "이름");
        room_lines.push("\x1b[1m" + name + "\x1b[0;37m" + han_iga(name)
            + " " + weapon["fight_start"]);
    }
    let own_name = get_string(ob, "이름");
    let room_text = join(room_lines, "\r\n");
    for player_name in get_room_players(ob) {
        if player_name != own_name {
            send_to_user(player_name, room_text);
        }
    }
    for linked in result["linked"] {
        send_line(ob, entry_mob_start(linked["name"], linked["script"]));
    }
    for skill in result["immediate_skills"] {
        let mob = "\x1b[33m" + skill["mob"] + "\x1b[37m";
        let text = entry_skill_text(skill["script"], mob, "당신", "\x1b[36m주먹\x1b[37m");
        if skill["kind"] == "defense" && skill["amount"] > 0 {
            send_line(ob, text + " (\x1b[1;31m+ " + skill["amount"].to_string() + "\x1b[0;37m)");
        } else {
            send_line(ob, text);
        }
        let actor = "\x1b[1m" + own_name + "\x1b[0;37m";
        let room_skill = entry_skill_text(skill["script"], mob, actor, "\x1b[36m주먹\x1b[37m");
        for player in get_cast_room_players(ob) {
            if player["name"] != own_name && !player["reject_fight"] {
                send_to_user(player["name"], room_skill);
            }
        }
    }
    finish_entry_auto_skill(ob);
    true
}
