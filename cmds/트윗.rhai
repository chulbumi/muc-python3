// Python `cmds/트윗.py`: 수신자 설정별 전역 twitter 잡담.

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [내용] 외침(,)");
        return;
    }
    if length(line) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^");
        return;
    }
    if get_user_config(ob, "외침거부") == "1" {
        send_line(ob, "☞ 외침거부중엔 외칠 수 없어요. ^^");
        return;
    }
    if get_int(ob, "act") == 4 {
        send_line(ob, "☞ 운기조식중에 외치게 되면 기가 흐트러집니다.");
        return;
    }
    let position = get_my_position(ob);
    if has_room_property(position["zone"], position["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다.");
        return;
    }

    let own_name = get_string(ob, "이름");
    let message = "\x1b[1m" + own_name + "\x1b[0;37m(\x1b[1;34mtwitter\x1b[0;37m) : " + line;
    let ansi_message = chat_user_ansi(message, true);
    let plain_message = chat_user_ansi(message, false);
    let time = local_time_hm();
    add_chat_history(ob, time + ansi_message + "\x1b[0;37m");

    for player in get_all_online_players() {
        let config = player["설정상태"].to_string();
        if config_text_is_enabled(config, "외침거부") { continue; }
        let shown = if config_text_is_enabled(config, "사용자안시거부") {
            plain_message
        } else {
            ansi_message
        };
        let shown = if config_text_is_enabled(config, "잡담시간보기") {
            time + shown
        } else {
            shown
        };
        let prefix = if player["이름"].to_string() == own_name { "" } else { "\r\n" };
        send_to_user(player["이름"].to_string(), prefix + shown + "\x1b[0;37;40m");
    }
}

fn main(ob, line) { cmd(ob, line); }
