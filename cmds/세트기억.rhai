// 세트기억.rhai - 장비 세트 저장
// Python: cmds/세트기억.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 명령: 세트기억 [세트이름]");
        send_line(ob, "   (현재 착용 중인 장비를 세트로 저장)");
        return;
    }
    send_line(ob, "☞ 장비 세트 저장 기능은 현재 구현 중입니다.");
    send_line(ob, "   (세트이름: '" + line + "')");
}

fn main(ob, line) {
    cmd(ob, line);
}
