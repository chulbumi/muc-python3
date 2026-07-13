// Python `cmds/외쳐2.py`: 밍밍 꼬리말을 붙이는 전역 외침 변형.

fn shout2_recipient_tail(player) {
    let config = player["설정상태"].to_string();
    if config_text_is_enabled(config, "엘피출력") || player["interactive"] != 1 { return "\r\n"; }
    "\r\n\r\n\x1b[0;37;40m[ " + player["현재체력"].to_string() + "/"
        + player["현재최고체력"].to_string() + ", " + player["현재내공"].to_string() + "/"
        + player["현재최고내공"].to_string() + " ] "
}

fn cmd(ob, line) {
    if line == "" { send_line(ob, "☞ 사용법: [내용] 외침(,)"); return; }
    if length(line) > 160 { send_line(ob, "☞ 너무 길어요. ^^"); return; }
    if get_user_config(ob, "외침거부") == "1" {
        send_line(ob, "☞ 외침거부중엔 외칠 수 없어요. ^^"); return;
    }
    if get_int(ob, "act") == 4 {
        send_line(ob, "☞ 운기조식중에 외치게 되면 기가 흐트러집니다."); return;
    }
    let position = get_my_position(ob);
    // Python dereferences ob.env here.  With no room that command aborts before
    // chat history or recipient delivery; keep the observable no-output state.
    if position["zone"] == "" { return; }
    if has_room_property(position["zone"], position["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다."); return;
    }
    let personality = get_string(ob, "성격");
    let kind = if personality == "선인" { "\x1b[1;36m창룡후\x1b[0;37m" }
        else if personality == "기인" { "\x1b[1;32m사자후\x1b[0;37m" }
        else { "\x1b[32m외 침\x1b[37m" };
    let own_name = get_string(ob, "이름");
    let message = "\x1b[1m" + own_name + "\x1b[0;37m(" + kind + ") : " + line;
    let time = local_time_hm();
    add_chat_history_limit(ob, time + message, 22);
    for player in get_all_online_players() {
        let config = player["설정상태"].to_string();
        if config_text_is_enabled(config, "외침거부") { continue; }
        let shown = if config_text_is_enabled(config, "잡담시간보기") { time + message } else { message };
        let prefix = if player["이름"].to_string() == own_name { "" } else { "\r\n" };
        let tail = if player["이름"].to_string() == own_name { "\r\n" } else { shout2_recipient_tail(player) };
        send_raw_to_user(player["이름"].to_string(), prefix + shown + " \x1b[1;32m밍밍이지렁~\x1b[0;37m" + tail);
    }
}

fn main(ob, line) { cmd(ob, line); }
