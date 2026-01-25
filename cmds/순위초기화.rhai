// 순위초기화.rhai - rank_clear(ty). ty 빈 문자열이면 전체. 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        rank_clear("");
        send_line(ob, "* 전체가 초기화 되었습니다.");
        return;
    }
    if rank_clear(trim(line)) {
        send_line(ob, "* 초기화 되었습니다.");
    } else {
        send_line(ob, "☞ 그런 순위는 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
