// 표현.rhai - 감정/동작 표현. [동작] 표현. send_emotion(ob, action) → EmotionToRoom
// Python: cmds/표현.py. to_self="당신이 {action}", to_room="{name}{iga} {action}" 은 efun에서

fn cmd(ob, line) {
    let action = trim(line);
    if length(action) == 0 {
        send_line(ob, "☞ 사용법: [내용] 표현(')");
        return;
    }
    let position = get_my_position(ob);
    if position["zone"] == "" { return; }
    if has_room_property(position["zone"], position["room"], "모든통신금지") {
        send_line(ob, "☞ 이지역에서는 어떠한 통신도 불가능합니다.");
        return;
    }
    send_emotion(ob, action);
}

fn main(ob, line) {
    cmd(ob, line);
}
