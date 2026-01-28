// 성올려.rhai - 성향 수치 올리기 (관리자)
// Python: cmds/성올려.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 성올려 [대상] [증감량]");
        return;
    }
    send_line(ob, "☞ 성향 수치 조절 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
