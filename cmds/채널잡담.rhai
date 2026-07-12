// Python: cmds/채널잡담.py

fn channel_member_prompt(member) {
    if !member["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ " + int_to_str(member["hp"]) + "/"
        + int_to_str(member["max_hp"]) + ", " + int_to_str(member["mp"]) + "/"
        + int_to_str(member["max_mp"]) + " ] "
}

fn cmd(ob, line) {
    if length(line) == 0 {
        send_line(ob, "☞ 사용법: [내용] 채널잡담([)\r\n");
        return;
    }
    if !is_adult_channel_member(ob) {
        send_line(ob, "☞ 먼저 채널에 입장하세요.\r\n");
        return;
    }
    if length(line) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^\r\n");
        return;
    }
    if get_user_config(ob, "외침거부") == "1" {
        send_line(ob, "☞ 외침거부중엔 외칠 수 없어요. ^^\r\n");
        return;
    }
    if get_int(ob, "act") == 4 {
        send_line(ob, "☞ 운기조식중에 외치게 되면 기가 흐트러집니다.\r\n");
        return;
    }

    let position = get_my_position(ob);
    if position["zone"] == "" {
        return;
    }
    if has_room_property(position["zone"], position["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다.\r\n");
        return;
    }

    let self_id = get_adult_channel_self_id();
    let actor = "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m";
    let text = "\x1b[1;31m①⑨\x1b[0;37m " + actor + ": " + line;
    for member in get_adult_channel_members() {
        if !member["active"] || member["외침거부"] {
            continue;
        }
        if member["id"] == self_id {
            adult_channel_send(ob, self_id, text + "\r\n\r\n");
        } else {
            adult_channel_send(ob, member["id"], "\r\n" + text + "\r\n" + channel_member_prompt(member));
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
