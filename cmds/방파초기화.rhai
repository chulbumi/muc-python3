// 방파초기화.rhai - [방파이름] guild_remove(id). 관리자 2000. 전체 초기화는 별도.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [방파이름] 방파초기화");
        return;
    }
    let id = trim(line);
    if guild_remove(id) {
        send_line(ob, "* 방파가 초기화되었습니다.");
    } else {
        send_line(ob, "☞ 그런 방파는 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
