// Python: cmds/전음.py

fn tell_message(words, start) {
    let msg = "";
    let i = start;
    while i < len(words) {
        // Python: msg += words[i] + ' '
        msg += words[i].to_string() + " ";
        i += 1;
    }
    msg
}

fn tell_prompt(target) {
    if !target["show_prompt"] {
        return "";
    }
    "\r\n\x1b[0;37;40m[ "
        + target["hp"].to_string() + "/" + target["max_hp"].to_string()
        + ", " + target["mp"].to_string() + "/" + target["max_mp"].to_string()
        + " ] "
}

fn send_python_tell(ob, target, msg) {
    let sender_name = get_body_string(ob, "이름");
    let target_name = target["name"].to_string();
    let tag = "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] ";
    let msg1 = tag + target_name + "에게 보냄 : " + msg;
    let msg2 = tag + sender_name + " : " + msg;

    // sender_output/recipient_output은 Python sendLine/lpPrompt 호출이 실제
    // 소켓에 쓰는 순서와 CRLF를 Rhai에서 완성한 값이다.
    let sender_output = msg1 + "\r\n";
    let recipient_output = "\r\n" + msg2 + "\r\n" + tell_prompt(target);
    if target["self"] {
        // 자기 전음 뒤 lineReceived의 발신자 prompt가 이어질 줄바꿈.
        recipient_output += "\r\n";
    } else {
        // 일반 발신자 lineReceived의 prompt(True) 앞 줄바꿈.
        sender_output += "\r\n";
    }
    deliver_tell(ob, target["token"].to_string(), sender_output, recipient_output, msg2);
}

fn cmd(ob, line) {
    let words = line_args(line);
    if length(line) == 0 || len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [내용] 전음(/)");
        return;
    }

    let target = find_tell_player(words[0].to_string());
    if !target["found"] {
        send_line(ob, "☞ 전음이 전달될만한 상대가 없어요. ^^");
        return;
    }
    if get_user_config(ob, "전음거부") == "1" || target["refuses_tell"] {
        send_line(ob, "☞ 전음 거부중이에요. ^^");
        return;
    }
    let pos = get_my_position(ob);
    if pos["zone"] == "" { return; }
    if has_room_property(pos["zone"], pos["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다.");
        return;
    }

    send_python_tell(ob, target, tell_message(words, 1));
}

fn main(ob, line) {
    cmd(ob, line);
}
