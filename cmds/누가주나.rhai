// 누가주나.rhai - 특정 아이템을 가진 몹 검색
// Python: cmds/누가주나.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: [아이템인덱스] 누가주나");
        return;
    }
    send_line(ob, "☞ 특정 아이템을 가진 몹 검색 기능은 현재 구현 중입니다.");
    send_line(ob, "   (Mob.Mobs 데이터베이스 조회 필요)");
}

fn main(ob, line) {
    cmd(ob, line);
}
