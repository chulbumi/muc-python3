// 사용자몹제거.rhai - 사용자 생성 몹 삭제 (현재 방)
// Python: cmds/사용자몹제거.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    send_line(ob, "☞ 현재 방의 사용자 생성 몹을 모두 삭제하는 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
