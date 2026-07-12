// 트윗.rhai - 전역 잡담. Python cmds/트윗.py의 길이·거부 설정·운기조식
// 검사를 유지하고, 전송 대상은 Rust의 접속자 이름 스냅샷으로 제한한다.
// Python: cmds/트윗.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 트윗");
        return;
    }
    if length(trim(line)) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^");
        return;
    }
    if get_user_config(ob, "외침거부") == "1" {
        send_line(ob, "☞ 외침거부중엔 외칠 수 없어요. ^^");
        return;
    }
    if get_bool(ob, "운기조식") {
        send_line(ob, "☞ 운기조식중에 외치게 되면 기가 흐트러집니다.");
        return;
    }
    let position = get_my_position(ob);
    if has_room_property(position["zone"], position["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다.");
        return;
    }
    let name = get_string(ob, "이름");
    let formatted = name + "(\x1b[1;34mtwitter\x1b[0;37m) : " + trim(line);
    add_chat_history(local_time_hm() + formatted);
    for n in get_online_names() {
        if !user_refuses_shout(n) {
            send_to_user(n, formatted);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
