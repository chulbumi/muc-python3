// Python: cmds/채널퇴장.py and Player.logout()

fn channel_member_prompt(member) {
    if !member["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ " + int_to_str(member["hp"]) + "/"
        + int_to_str(member["max_hp"]) + ", " + int_to_str(member["mp"]) + "/"
        + int_to_str(member["max_mp"]) + " ] "
}

fn cmd(ob, line) {
    if !is_adult_channel_member(ob) {
        send_line(ob, "☞ 먼저 채널에 입장하세요.");
        return;
    }
    if !adult_channel_leave(ob) {
        send_line(ob, "☞ 먼저 채널에 입장하세요.");
        return;
    }

    let self_id = get_adult_channel_self_id();
    let actor = "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m";
    let notice = "\x1b[1;31m①⑨\x1b[0;37m " + actor + "님이 퇴장하셨습니다.";

    // Python removes self first, then iterates the remaining insertion order.
    for member in get_adult_channel_members() {
        if member["id"] != self_id {
            adult_channel_send(ob, member["id"], "\r\n" + notice + "\r\n" + channel_member_prompt(member));
        }
    }

    // Player.logout() performs the same removal/notice without a self line.
    if !is_adult_channel_disconnect(ob) {
        adult_channel_send(ob, self_id, "☞ 채널에서 퇴장합니다.\r\n\r\n");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
