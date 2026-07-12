// Python cmds/입문신청.py. Presentation stays in Rhai; the live target Body
// mutation is requested from Rust after command execution.

fn has_applicant(raw, name) {
    let normalized = return_replace(return_replace(raw, "\r\n", ","), "\n", ",");
    for saved in split(normalized, ",") {
        if saved == name { return true; }
    }
    false
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

    let present = false;
    for name in get_room_players(ob) {
        if name == line { present = true; break; }
    }
    if !present {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }

    let own_name = get_string(ob, "이름");
    if line == own_name {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }

    let target_position = get_room_player_attr(ob, line, "직위");
    if target_position.is_unit() {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    if target_position != "방주" && target_position != "부방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }

    let mine = get_string(ob, "성격");
    let target_personality = get_room_player_attr(ob, line, "성격");
    let target_original = get_room_player_attr(ob, line, "기존성격");
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
    if has_applicant(get_room_player_attr(ob, line, "입문신청자"), own_name) {
        send_line(ob, "☞ 이미 입문 신청을 하였습니다.");
        return;
    }

    request_guild_application(ob, line);
    let target_a = "\x1b[1m" + line + "\x1b[0;37m";
    let actor_a = "\x1b[1m" + own_name + "\x1b[0;37m";
    send_line(ob, "당신이 " + target_a + "의 방파에 입문을 신청합니다.");
    send_to_user(line, actor_a + han_iga(own_name) + " 당신의 방파에 입문을 신청합니다.");
}

fn main(ob, line) { cmd(ob, line); }
