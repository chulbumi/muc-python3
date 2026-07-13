// Python cmds/입문신청.py. Presentation stays in Rhai; the live target Body
// mutation is requested from Rust after command execution.

fn has_applicant(raw, name) {
    let normalized = return_replace(return_replace(return_replace(raw, "\r\n", ","), "\n", ","), "|", ",");
    for saved in split(normalized, ",") {
        if saved == name { return true; }
    }
    false
}

fn guild_apply_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
        + person["max_hp"].to_string() + ", " + person["mp"].to_string() + "/"
        + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    if get_string(ob, "소속") != "" {
        send_line(ob, "☞ 방파에 입문 신청을 할 수 없습니다.");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법 : [방주이름] 입문신청");
        return;
    }

    let target_name = resolve_room_player_name(ob, line);
    if target_name == "" {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }

    let own_name = get_string(ob, "이름");
    if target_name == own_name {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }

    let target_position = get_room_player_attr(ob, target_name, "직위");
    if target_position.is_unit() {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    if target_position != "방주" && target_position != "부방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }

    let mine = get_string(ob, "성격");
    let target_personality = get_room_player_attr(ob, target_name, "성격");
    let target_original = get_room_player_attr(ob, target_name, "기존성격");
    let allowed = if target_personality == "기인" {
        mine == "기인" || mine == "선인" || mine == target_original
    } else if target_personality == "선인" {
        mine != ""
    } else {
        mine == "기인" || mine == "선인" || mine == target_personality
    };
    if !allowed {
        send_line(ob, "☞ 방파에 입문 신청을 할 수 없습니다.");
        return;
    }
    if has_applicant(get_room_player_attr(ob, target_name, "입문신청자"), own_name) {
        send_line(ob, "☞ 이미 입문 신청을 하였습니다.");
        return;
    }

    request_guild_application(ob, target_name);
    let target_a = "\x1b[1m" + target_name + "\x1b[0;37m";
    let actor_a = "\x1b[1m" + own_name + "\x1b[0;37m";
    send_line(ob, "당신이 " + target_a + "의 방파에 입문을 신청합니다.");
    let target_output = "\r\n" + actor_a + han_iga(own_name) + " 당신의 방파에 입문을 신청합니다.\r\n";
    let context = get_party_context();
    for person in context["room_players"] {
        if person["name"] == target_name { target_output += guild_apply_prompt(person); break; }
    }
    send_raw_to_user(target_name, target_output);
}

fn main(ob, line) { cmd(ob, line); }
