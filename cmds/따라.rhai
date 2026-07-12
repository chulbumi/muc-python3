// Python: cmds/따라.py and Body.addFollow/delFollow/addFollower/delFollower

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

fn notify_stopped_following(ob, actor, old_target) {
    send_line(ob, "당신이 " + object_eul(old_target) + " 따라다니는 것을 그만둡니다.");
    let raw = "\r\n" + object_iga(actor) + " 당신과 따라다니는 것을 그만둡니다.\r\n"
        + party_prompt(old_target);
    party_send_raw(ob, old_target["id"], raw);
}

fn leave_member_party(ob, context) {
    let actor = context["self"];
    let leader = context["party_leader"];
    let leave_notice = "당신의 무리에서 \x1b[1m" + actor["name"]
        + "\x1b[0m\x1b[40m\x1b[37m" + han_iga(actor["name"]) + " 나갔습니다.";

    // sendToParty(..., ex=leader, prompt=True) after removing this member.
    for member in context["party_members"] {
        if member["id"] == actor["id"] {
            continue;
        }
        party_send_raw(ob, member["id"], "\r\n" + leave_notice + "\r\n" + party_prompt(member));
    }

    party_send_raw(ob, leader["id"], "\r\n" + leave_notice + "\r\n");
    send_line(ob, "당신은 \x1b[1m" + leader["name"]
        + "\x1b[0m\x1b[40m\x1b[37m의 무리에서 이탈 하였습니다.");

    if len(context["party_members"]) == 1 {
        party_send_raw(ob, leader["id"], "\r\n무리가 해제 되었습니다\r\n" + party_prompt(leader));
    } else {
        let prompt = party_prompt(leader);
        if prompt != "" {
            party_send_raw(ob, leader["id"], prompt);
        }
    }
    party_request_leave(ob);
}

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 동행");
        return;
    }

    let context = get_party_context();
    let actor = context["self"];
    if line == "나" || line == actor["name"] {
        let leader = context["party_leader"];
        if leader["id"] != "" {
            if leader["id"] == actor["id"] {
                send_line(ob, "무리의 대장은 이탈할수 없어요.");
                return;
            }
            leave_member_party(ob, context);
            return;
        }

        let old_target = context["follow"];
        if old_target["id"] != "" {
            if party_request_stop_following(ob) {
                notify_stopped_following(ob, actor, old_target);
            }
        }
        send_line(ob, "당신은 홀로 강호를 주유하기 시작합니다.");
        return;
    }

    let target = find_follow_player(line);
    if !target["lookup_supported"] {
        return;
    }
    if target["id"] == () || target["id"] == "" || !target["is_player"] {
        send_line(ob, "☞ 그런 대상이 없어요. ^^");
        return;
    }
    if target["refuses_follow"] {
        send_line(ob, object_iga(target) + " 동행거부중 입니다.");
        return;
    }
    if !party_request_follow(ob, target["id"]) {
        return;
    }

    let old_target = context["follow"];
    if old_target["id"] != "" {
        notify_stopped_following(ob, actor, old_target);
    }

    party_send_raw(
        ob,
        target["id"],
        "\r\n" + object_iga(actor) + " 당신을 따라다니기 시작합니다.\r\n" + party_prompt(target)
    );
    send_line(ob, "당신은 " + object_eul(target) + " 따라다니기 시작합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
