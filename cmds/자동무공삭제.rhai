// 자동무공삭제.rhai - 자동 무공 삭제
// Python: cmds/자동무공삭제.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 자동무공삭제 [무공이름]");
        return;
    }
    send_line(ob, "☞ 자동 무공 삭제 기능은 현재 구현 중입니다.");
    send_line(ob, "   (자동 무공 목록에서 '" + line + "' 삭제)");
}

fn main(ob, line) {
    cmd(ob, line);
}
