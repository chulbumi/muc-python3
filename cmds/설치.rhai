// 설치.rhai - 방 설치
// Python: cmds/설치.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 설치 [방이름]");
        return;
    }
    send_line(ob, "☞ 방 설치 기능은 현재 구현 중입니다.");
    send_line(ob, "   (현재 위치에 '" + line + "' 방 설치)");
}

fn main(ob, line) {
    cmd(ob, line);
}
