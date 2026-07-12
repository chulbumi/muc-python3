// 기연초기화.rhai - Python: cmds/기연초기화.py
// ONEITEM.clear(). 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    oneitem_clear();
    send_line(ob, "* 기연아이템 목록이 초기화되었습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
