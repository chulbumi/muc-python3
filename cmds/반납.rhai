// 반납.rhai - 대여 아이템 반납
// Python: cmds/반납.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 명령: 반납 [아이템이름]");
        return;
    }
    send_line(ob, "☞ 대여 아이템 반납 기능은 현재 구현 중입니다.");
    send_line(ob, "   (아이템: '" + line + "')");
}

fn main(ob, line) {
    cmd(ob, line);
}
