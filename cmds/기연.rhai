// 기연.rhai - Python: cmds/기연.py
// for index in ONEITEM.attr: name=ONEITEM.getName(index), owner=ONEITEM.attr[index], msg += '%-16s (%-16s) : %s\\r\\n'. sendLine(msg). ONEITEM 연동 예정.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        return;
    }
    // ONEITEM.attr, ONEITEM.getName(index), ONEITEM.attr[index] (owner) 연동 예정. 순회 결과가 없으면 msg는 빈 문자열.
    send_line(ob, "☞ (ONEITEM.attr 순회: name (index) : owner. ONEITEM 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
