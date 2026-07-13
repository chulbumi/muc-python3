// 순위초기화.rhai - rank_clear(ty). ty 빈 문자열이면 전체. 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let rank_type = trim(line);
    if rank_type == "" {
        rank_clear("");
        send_line(ob, "* 전체가 초기화 되었습니다.");
        return;
    }
    if rank_clear(rank_type) {
        send_line(ob, "* 초기화 되었습니다.");
    } else {
        send_line(ob, "☞ 그런 순위는 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
