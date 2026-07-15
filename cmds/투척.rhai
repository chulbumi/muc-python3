// 소모성 암기 투척. 상태와 수치는 efun이 처리하고 문구는 이 파일이 소유한다.

fn concealed_actor(ob) {
    let name = get_string(ob, "이름");
    "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name)
}

fn concealed_prompt(player) {
    if !player["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + player["hp"].to_string() + "/"
        + player["max_hp"].to_string() + ", " + player["mp"].to_string()
        + "/" + player["max_mp"].to_string() + " ] "
}

fn send_concealed_room(ob, text) {
    let own_name = get_string(ob, "이름");
    for player in get_cast_room_players(ob) {
        if player["name"] != own_name {
            send_raw_to_user(player["name"], "\r\n" + text + "\r\n"
                + concealed_prompt(player));
        }
    }
}

fn concealed_mob_start(name, script) {
    "\x1b[33m" + name + "\x1b[37m" + han_iga(name) + " " + script
}

fn concealed_skill_script(line, attacker, target) {
    let text = return_replace(line, "[공]", attacker);
    text = return_replace(text, "[방]", target);
    text = return_replace(text, "[무]", "\x1b[36m주먹\x1b[37m");
    text = post_position_once(text);
    text = post_position_once(text);
    post_position_once(text)
}

fn cmd(ob, line) {
    if length(trim(line)) == 0 {
        send_line(ob, "☞ 사용법: [대상] [암기] 투척");
        return;
    }
    if combat_room_has_attr(ob, "전투금지") {
        send_line(ob, "☞ 이곳에선 모든 전투가 금지되어 있어요. ^^");
        return;
    }

    let result = throw_concealed_weapon(ob, trim(line));
    let status = result["status"];
    if status == "usage" {
        send_line(ob, "☞ 사용법: [대상] [암기] 투척");
        return;
    }
    if status == "missing_item" {
        send_line(ob, "☞ 던질 만한 암기가 소지품에 없어요.");
        return;
    }
    if status == "cooldown" {
        send_line(ob, "☞ 아직 다음 암기를 꺼낼 틈이 나지 않았어요.");
        return;
    }
    if status == "other_target" {
        send_line(ob, "☞ 현재의 비무에 신경을 집중하세요. @_@");
        return;
    }
    if status != "ok" {
        send_line(ob, "☞ 그 대상에게 암기를 날릴 수 없어요.");
        return;
    }

    if result["started"] {
        finish_entry_auto_skill(ob);
    }

    let item = "\x1b[1;36m" + result["item_name"] + "\x1b[0;37m";
    let target = "\x1b[33m" + result["target_name"] + "\x1b[37m";
    let damage = result["damage"].to_string();
    let own_line;
    let room_line;
    if result["surprise"] {
        own_line = "당신이 소매를 떨치자 " + item + " 한 점이 섬광처럼 날아가 "
            + target + "의 허점을 꿰뚫습니다. (\x1b[1;31m- " + damage + "\x1b[0;37m)";
        room_line = concealed_actor(ob) + " 소매를 떨치자 " + item
            + " 한 점이 섬광처럼 날아가 " + target + "의 허점을 꿰뚫습니다.";
    } else {
        own_line = "당신이 난전 속에서 " + item + "을 날리지만, " + target
            + "의 경계에 막혀 위력이 크게 줄어듭니다. (\x1b[1;31m- "
            + damage + "\x1b[0;37m)";
        room_line = concealed_actor(ob) + " 난전 속에서 " + item + "을 날리지만, "
            + target + "의 경계에 막혀 위력이 크게 줄어듭니다.";
    }
    send_line(ob, own_line);
    send_concealed_room(ob, room_line);

    if result["started"] && result["target_was_stand"] && !result["died"] {
        send_line(ob, concealed_mob_start(result["target_name"], result["target_start"]));
    }
    if result["started"] {
        for linked in result["linked"] {
            send_line(ob, concealed_mob_start(linked["name"], linked["script"]));
        }
        for skill in result["immediate_skills"] {
            let mob_a = "\x1b[33m" + skill["mob"] + "\x1b[37m";
            let text = concealed_skill_script(skill["script"], mob_a, "당신");
            if skill["kind"] == "defense" && skill["amount"] > 0 {
                send_line(ob, text + " (\x1b[1;31m+ " + skill["amount"].to_string()
                    + "\x1b[0;37m)");
            } else {
                send_line(ob, text);
            }
        }
    }
    if result["died"] {
        if result["death_script"] != "" {
            send_line(ob, "\x1b[1;37m" + result["death_script"] + "\x1b[0;37m");
        } else {
            send_line(ob, target + han_iga(result["target_name"])
                + " 비틀거리다 바닥에 쓰러집니다.");
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
