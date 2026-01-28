// 올숙리스트.rhai - 모든 아이템 목록 조회
// Python: cmds/올숙리스트.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    send_line(ob, "☞ 모든 아이템 목록 조회 기능은 현재 구현 중입니다.");
    send_line(ob, "   (Item.Items 데이터베이스 조회 필요)");
}

fn main(ob, line) {
    cmd(ob, line);
}
