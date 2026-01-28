// 정리.rhai - 플레이어 강제 종료 (관리자)
// Python: cmds/정리.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^^*");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [사용자명] 정리");
        return;
    }
    send_line(ob, "☞ 플레이어 강제 종료 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
