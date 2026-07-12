// Python: cmds/채널입장.py

fn channel_member_prompt(member) {
    if !member["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ " + int_to_str(member["hp"]) + "/"
        + int_to_str(member["max_hp"]) + ", " + int_to_str(member["mp"]) + "/"
        + int_to_str(member["max_mp"]) + " ] "
}

fn cmd(ob, line) {
    if is_adult_channel_member(ob) {
        // Client.lineReceived() calls prompt(True) after the command.
        send_line(ob, "☞ 이미 입장하셨습니다.\r\n");
        return;
    }
    if !adult_channel_join(ob) {
        send_line(ob, "☞ 이미 입장하셨습니다.\r\n");
        return;
    }

    let self_id = get_adult_channel_self_id();
    let actor = "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m";
    let notice = "\x1b[1;31m①⑨\x1b[0;37m " + actor + "님이 입장하셨습니다.";

    if is_adult_channel_auto_join(ob) {
        // Player.getStart(): old members are notified before append/self output.
        for member in get_adult_channel_members() {
            if member["id"] != self_id {
                adult_channel_send(ob, member["id"], "\r\n" + notice + "\r\n" + channel_member_prompt(member));
            }
        }
        adult_channel_send(ob, self_id, "☞ 채널에 입장합니다.\r\n\r\n");
    } else {
        // cmds/채널입장.py: self output precedes old-member notifications.
        adult_channel_send(ob, self_id, "☞ 채널에 입장합니다.\r\n\r\n");
        for member in get_adult_channel_members() {
            if member["id"] != self_id {
                adult_channel_send(ob, member["id"], "\r\n" + notice + "\r\n" + channel_member_prompt(member));
            }
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
