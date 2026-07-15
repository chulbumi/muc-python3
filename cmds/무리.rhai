// Python: cmds/무리.py, Player.sendToParty/logout and Body follower methods

fn party_prompt(person) {
    if !person["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ " + int_to_str(person["hp"]) + "/"
        + int_to_str(person["max_hp"]) + ", " + int_to_str(person["mp"]) + "/"
        + int_to_str(person["max_mp"]) + " ] "
}

fn object_iga(person) {
    "\x1b[1m" + person["name"] + "\x1b[0;37m" + han_iga(person["name"])
}

fn object_eul(person) {
    "\x1b[1m" + person["name"] + "\x1b[0;37m" + han_eul(person["name"])
}

fn object_eun(person) {
    "\x1b[1m" + person["name"] + "\x1b[0;37m" + han_eun(person["name"])
}

fn command_name(person) {
    "\x1b[1m" + person["name"] + "\x1b[0m\x1b[40m\x1b[37m"
}

fn pad_left(value, width) {
    let text = value.to_string();
    let result = "";
    let count = width - length(text);
    let i = 0;
    while i < count {
        result += " ";
        i += 1;
    }
    result + text
}

fn get_hp_bar(person) {
    let bars = [
        "\x1b[0m\x1b[41m\x1b[30m\x1b[0m\x1b[47m\x1b[30m         \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[41m\x1b[30m \x1b[0m\x1b[47m\x1b[30m        \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[41m\x1b[30m  \x1b[0m\x1b[47m\x1b[30m       \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[41m\x1b[30m   \x1b[0m\x1b[47m\x1b[30m      \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[43m\x1b[30m    \x1b[0m\x1b[47m\x1b[30m     \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[43m\x1b[30m     \x1b[0m\x1b[47m\x1b[30m    \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[43m\x1b[30m      \x1b[0m\x1b[47m\x1b[30m   \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[42m\x1b[30m       \x1b[0m\x1b[47m\x1b[30m  \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[42m\x1b[30m        \x1b[0m\x1b[47m\x1b[30m \x1b[0m\x1b[40m\x1b[37m",
        "\x1b[0m\x1b[42m\x1b[30m         \x1b[0m\x1b[47m\x1b[30m\x1b[0m\x1b[40m\x1b[37m"
    ];
    let hp = person["hp"];
    let max_hp = person["max_hp"];
    let index = (hp * 9) / max_hp;
    if index < 0 {
        index = 0;
    } else if index > 9 {
        index = 9;
    }
    pad_left((hp * 100) / max_hp, 3) + " " + bars[index]
}

fn party_row(person, leader) {
    let nick = person["nickname"];
    if nick == "" {
        nick = "무명객";
    }
    let prefix = if leader { "▶ " } else { "　 " };
    let msg = "";
    if person["personality"] != "기인" {
        msg = prefix + "\x1b[1m\x1b[33m\x1b[40m" + fill_space(12, nick)
            + "\x1b[0m\x1b[37m\x1b[40m " + fill_space(10, person["name"]) + "  ";
    } else {
        msg = prefix + fill_space(12, nick) + " " + fill_space(10, person["name"]) + "  ";
    }
    msg += fill_space(19, get_hp_bar(person)) + "  ";
    msg + pad_left(person["mp"], 5) + "/" + fill_space(5, person["max_mp"].to_string())
}

fn contains_person(people, id) {
    for person in people {
        if person["id"] == id {
            return true;
        }
    }
    false
}

fn view_party(ob, context) {
    let actor = context["self"];
    let leader = context["party_leader"];
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if leader["id"] == "" {
        send_line(ob, "◁ \x1b[1m" + actor["name"] + "\x1b[0m\x1b[40m\x1b[37m의 동행 ▷");
    } else {
        send_line(ob, "◁ \x1b[1m" + leader["name"] + "\x1b[0m\x1b[40m\x1b[37m의 무리 ▷");
    }
    send_line(ob, "────────────────────────────");

    if leader["id"] != "" {
        send_line(ob, party_row(leader, true));
        for member in context["party_members"] {
            send_line(ob, party_row(member, false));
        }
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
    send_line(ob, "동행중");
    for follower in context["followers"] {
        if leader["id"] != "" && contains_person(context["party_members"], follower["id"]) {
            continue;
        }
        send_line(ob, party_row(follower, false));
    }
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn add_party(ob, line, context) {
    let actor = context["self"];
    let leader = context["party_leader"];
    if leader["id"] != "" && leader["id"] != actor["id"] {
        send_line(ob, "☞ 이미 당신은 무리중이여서 무리를 만들 수 없어요. ^^");
        return;
    }

    let all_mode = line == "모두";
    let count_added = 0;
    let count_matched = 0;
    let count_party = 0;
    let selected = [];
    let soul_count = if context.contains("soul_member_count") {
        context["soul_member_count"]
    } else { 1 };
    for member in context["followers"] {
        if !all_mode && member["name"] != line {
            continue;
        }
        count_matched += 1;
        if !member["is_player"] {
            if !all_mode {
                send_line(ob, object_eun(member) + " 이곳에 없어서 무리에 참여하지 못합니다.");
            }
            continue;
        }
        if member["follow_id"] != actor["id"] {
            if !all_mode {
                send_line(ob, object_eun(member) + " 이미 따르는자가 있습니다.");
            }
            continue;
        }
        if member["party_leader_id"] != "" {
            if !all_mode {
                send_line(ob, object_eul(member) + " 따르는 무리가 있으므로 안됩니다.");
                count_party += 1;
            }
            continue;
        }
        if soul_count + len(context["party_members"]) + len(selected) >= 4 {
            if !all_mode {
                send_line(ob, "☞ 무리는 자신을 포함해 최대 4명까지 구성할 수 있습니다.");
            }
            continue;
        }

        selected.push(member["id"]);
        send_line(ob, "당신의 무리에 " + command_name(member) + han_iga(member["name"]) + " 들어옵니다.");
        party_send_raw(
            ob,
            member["id"],
            "\r\n당신이 " + command_name(actor) + "의 무리에 들어갑니다.\r\n" + party_prompt(member)
        );

        let room_notice = command_name(member) + han_iga(member["name"]) + " "
            + command_name(actor) + "의 무리에 들어갑니다.";
        for observer in context["room_players"] {
            if observer["id"] == actor["id"] || observer["id"] == member["id"] {
                continue;
            }
            party_send_raw(ob, observer["id"], "\r\n" + room_notice + "\r\n" + party_prompt(observer));
        }
        count_added += 1;
    }

    party_request_add_members(ob, selected);
    if count_matched == 0 {
        send_line(ob, "☞ 당신을 따르는 대상이 아닙니다.");
    } else if count_added == 0 && count_party == 0 {
        send_line(ob, "☞ 추가된 무리원이 없습니다.");
    }
}

fn logout_party(ob, context) {
    let actor = context["self"];
    let leader = context["party_leader"];
    party_request_disconnect(ob);

    if leader["id"] != "" {
        if leader["id"] == actor["id"] && len(context["party_members"]) != 0 {
            let successor = context["party_members"][0];
            if len(context["party_members"]) == 1 {
                party_send_raw(ob, successor["id"], "\r\n무리가 해제 되었습니다\r\n" + party_prompt(successor));
            } else {
                let changed = object_iga(successor) + " 무리의 대장으로 변경 되었습니다.";
                let i = 1;
                while i < len(context["party_members"]) {
                    let member = context["party_members"][i];
                    party_send_raw(ob, member["id"], "\r\n" + changed + "\r\n" + party_prompt(member));
                    i += 1;
                }
                party_send_raw(
                    ob,
                    successor["id"],
                    "\r\n당신은 무리의 대장으로 변경 되었습니다.\r\n" + party_prompt(successor)
                );
            }
        } else if leader["id"] != actor["id"] {
            let remaining = [];
            for member in context["party_members"] {
                if member["id"] != actor["id"] {
                    remaining.push(member);
                }
            }
            if len(remaining) == 0 {
                party_send_raw(ob, leader["id"], "\r\n무리가 해제 되었습니다\r\n" + party_prompt(leader));
            } else {
                let departed = "\r\n" + object_iga(actor) + " 무리에서 이탈 하였습니다.";
                // This is a bound `leader.sendToParty(...)` call in Python:
                // leader == self, so the leader gets only sendLine(line) here.
                party_send_raw(ob, leader["id"], departed + "\r\n");
                for member in remaining {
                    party_send_raw(ob, member["id"], "\r\n" + departed + "\r\n" + party_prompt(member));
                }
                // The explicit leader.lpPrompt() after sendToParty is the
                // leader's sole prompt for this notification.
                let prompt = party_prompt(leader);
                if prompt != "" {
                    party_send_raw(ob, leader["id"], prompt);
                }
            }
        }
    }

    let old_target = context["follow"];
    if old_target["id"] != "" {
        party_send_raw(
            ob,
            actor["id"],
            "당신이 " + object_eul(old_target) + " 따라다니는 것을 그만둡니다.\r\n"
        );
        party_send_raw(
            ob,
            old_target["id"],
            "\r\n" + object_iga(actor) + " 당신과 따라다니는 것을 그만둡니다.\r\n"
                + party_prompt(old_target)
        );
    }

    for follower in context["followers"] {
        if follower["follow_id"] == "" {
            continue;
        }
        party_send_raw(
            ob,
            follower["id"],
            "\r\n당신이 " + object_eul(actor) + " 따라다니는 것을 그만둡니다.\r\n"
                + party_prompt(follower)
        );
        party_send_raw(
            ob,
            actor["id"],
            "\r\n" + object_iga(follower) + " 당신과 따라다니는 것을 그만둡니다.\r\n"
                + party_prompt(actor)
        );
    }
}

fn cmd(ob, line) {
    let context = get_party_context();
    if is_party_disconnect(ob) {
        logout_party(ob, context);
        return;
    }
    if line == "" {
        // Python compares follower (always a list) to None, so even an empty
        // relation renders the 동행 frame rather than the intended error.
        view_party(ob, context);
        return;
    }
    add_party(ob, line, context);
}

fn main(ob, line) {
    cmd(ob, line);
}
