// Python: cmds/무리제외.py

fn party_prompt(person) {
    if !person["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ " + int_to_str(person["hp"]) + "/"
        + int_to_str(person["max_hp"]) + ", " + int_to_str(person["mp"]) + "/"
        + int_to_str(person["max_mp"]) + " ] "
}

fn command_name(person) {
    "\x1b[1m" + person["name"] + "\x1b[0m\x1b[40m\x1b[37m"
}

fn contains_id(ids, id) {
    for value in ids {
        if value == id {
            return true;
        }
    }
    false
}

fn contains_person(people, id) {
    for person in people {
        if person["id"] == id {
            return true;
        }
    }
    false
}

fn send_to_remaining_party(ob, context, removed_ids, message) {
    // The sender is the leader: sendToParty writes its own line without the
    // extra blank line/prompt, then remaining PartyMember in list order.
    send_line(ob, message);
    for party_member in context["party_members"] {
        if contains_id(removed_ids, party_member["id"]) {
            continue;
        }
        party_send_raw(
            ob,
            party_member["id"],
            "\r\n" + message + "\r\n" + party_prompt(party_member)
        );
    }
}

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [동행원|무리원] 제외");
        return;
    }

    let context = get_party_context();
    let actor = context["self"];
    let leader = context["party_leader"];
    if leader["id"] == "" {
        send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^");
        return;
    }
    if leader["id"] != actor["id"] {
        send_line(ob, "☞ 당신을 따르는 무리가 없어요. ^^");
        return;
    }

    let all_mode = line == "모두";
    let removed_ids = [];
    let count = 0;
    for member in context["followers"] {
        if !all_mode && member["name"] != line {
            continue;
        }
        removed_ids.push(member["id"]);
        if contains_person(context["party_members"], member["id"]) {
            let notice = "당신의 무리에서 " + command_name(member)
                + han_eul(member["name"]) + " 제외시킵니다.";
            send_to_remaining_party(ob, context, removed_ids, notice);
            party_send_raw(
                ob,
                member["id"],
                "\r\n" + command_name(actor) + "의 무리에서 당신을 제외시킵니다.\r\n"
                    + party_prompt(member)
            );
        } else {
            let notice = "당신이 " + command_name(member) + han_eul(member["name"])
                + " 더이상 따라다니지 못하게 합니다.";
            send_to_remaining_party(ob, context, removed_ids, notice);
            party_send_raw(
                ob,
                member["id"],
                "\r\n" + command_name(actor)
                    + "의 무리에서 당신을 더이상 따라다니지 못하게 합니다.\r\n"
                    + party_prompt(member)
            );
        }
        count += 1;
    }

    if count == 0 {
        send_line(ob, "☞ 당신을 따르는 그런 대상이 없어요. ^^");
        return;
    }
    party_request_remove_followers(ob, removed_ids);
}

fn main(ob, line) {
    cmd(ob, line);
}
