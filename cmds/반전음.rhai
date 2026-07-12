// Python: cmds/반전음.py

fn reply_message(words) {
    let msg = "";
    for word in words {
        // Python: msg += words[i] + ' '
        msg += word.to_string() + " ";
    }
    msg
}

fn reply_prompt(target) {
    if !target["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ "
        + target["hp"].to_string() + "/" + target["max_hp"].to_string()
        + ", " + target["mp"].to_string() + "/" + target["max_mp"].to_string()
        + " ] "
}

fn send_python_reply_tell(ob, target, msg) {
    let sender_name = get_body_string(ob, "이름");
    let target_name = target["name"].to_string();
    let tag = "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] ";
    let msg1 = tag + target_name + "에게 보냄 : " + msg;
    let msg2 = tag + sender_name + " : " + msg;
    let sender_output = msg1 + "\r\n";
    let recipient_output = "\r\n" + msg2 + "\r\n" + reply_prompt(target);
    if target["self"] {
        recipient_output += "\r\n";
    } else {
        sender_output += "\r\n";
    }
    deliver_tell(ob, target["token"].to_string(), sender_output, recipient_output, msg2);
}

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [내용] 반전음(:)");
        return;
    }
    let words = line_args(line);
    let target = find_reply_tell_player(ob);
    if !target["found"] {
        send_line(ob, "☞ 전음이 전달될만한 상대가 없어요. ^^");
        return;
    }
    if get_user_config(ob, "전음거부") == "1" || target["refuses_tell"] {
        send_line(ob, "☞ 전음 거부중이에요. ^^");
        return;
    }
    let pos = get_my_position(ob);
    if has_room_property(pos["zone"], pos["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다.");
        return;
    }

    send_python_reply_tell(ob, target, reply_message(words));
}

fn main(ob, line) {
    cmd(ob, line);
}
