// 대여.rhai - 아이템 대여
// Python: cmds/대여.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 명령: 대여 [아이템이름] [대상이름]");
        send_line(ob, "   (아이템을 다른 사용자에게 대여)");
        return;
    }
    send_line(ob, "☞ 아이템 대여 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
