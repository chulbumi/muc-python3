// Python: cmds/무리말.py and Player.sendToParty

fn party_prompt(person) {
    if !person["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ " + int_to_str(person["hp"]) + "/"
        + int_to_str(person["max_hp"]) + ", " + int_to_str(person["mp"]) + "/"
        + int_to_str(person["max_mp"]) + " ] "
}

fn cmd(ob, line) {
    if length(line) == 0 {
        send_line(ob, "☞ 사용법: [내용] 무리말(;)");
        return;
    }
    if length(line) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^");
        return;
    }

    let position = get_my_position(ob);
    if position["zone"] == "" {
        return;
    }
    if has_room_property(position["zone"], position["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다.");
        return;
    }

    let context = get_party_context();
    let actor = context["self"];
    let leader = context["party_leader"];
    if leader["id"] == "" {
        send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^");
        return;
    }

    let message = "";
    if leader["id"] == actor["id"] {
        message = "\x1b[1m\x1b[40m\x1b[32m◀\x1b[0m\x1b[40m\x1b[37m"
            + actor["name"] + "\x1b[1m\x1b[40m\x1b[32m▶\x1b[0m\x1b[40m\x1b[37m " + line;
        send_line(ob, message);
    } else {
        message = "\x1b[1m\x1b[40m\x1b[32m◁\x1b[0m\x1b[40m\x1b[37m"
            + actor["name"] + "\x1b[1m\x1b[40m\x1b[32m▷\x1b[0m\x1b[40m\x1b[37m " + line;
        party_send_raw(ob, leader["id"], "\r\n" + message + "\r\n" + party_prompt(leader));
    }

    for member in context["party_members"] {
        if member["id"] == actor["id"] {
            send_line(ob, message);
        } else {
            party_send_raw(ob, member["id"], "\r\n" + message + "\r\n" + party_prompt(member));
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
