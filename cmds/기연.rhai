// 기연.rhai - Python: cmds/기연.py
// ONEITEM.attr 순회: oneitem_list() → "%-16s (%-16s) : %s\r\n" 형식. 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    let msg = oneitem_list();
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
