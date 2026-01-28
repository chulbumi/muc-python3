// 무림별호.rhai - 무림별호 설정
// Python: cmds/무림별호.py

fn cmd(ob, line) {
    if is_empty(line) {
        let nick = get_string(ob, "무림별호");
        if nick == "" {
            send_line(ob, "☞ 당신은 무명객입니다.");
        } else {
            send_line(ob, "☞ 당신의 무림별호는 [" + nick + "]입니다.");
        }
        return;
    }
    send_line(ob, "☞ 무림별호 설정 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
