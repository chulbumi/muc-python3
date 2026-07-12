// 기연.rhai - Python: cmds/기연.py
// ONEITEM.attr 순회: oneitem_list() → "%-16s (%-16s) : %s\r\n" 형식. 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let msg = oneitem_list();
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
