// 표현.rhai - 감정/동작 표현. [동작] 표현. send_emotion(ob, action) → EmotionToRoom
// Python: cmds/표현.py. to_self="당신이 {action}", to_room="{name}{iga} {action}" 은 efun에서

fn cmd(ob, line) {
    let action = trim(line);
    if length(action) == 0 {
        send_line(ob, "☞ 사용법: 표현 [동작] 또는 ' [동작]");
        return;
    }
    send_emotion(ob, action);
}

fn main(ob, line) {
    cmd(ob, line);
}
