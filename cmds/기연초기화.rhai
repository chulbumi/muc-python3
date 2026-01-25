// 기연초기화.rhai - Python: cmds/기연초기화.py
// ONEITEM.clear(). 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    oneitem_clear();
    send_line(ob, "* 기연아이템 목록이 초기화되었습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
