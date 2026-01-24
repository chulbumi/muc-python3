// 기연.rhai - 기연. Python: cmds/기연.py
// ONEITEM.attr 순회. get_global("oneitem") 등 연동 없음 → 관리자 안내.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        // ONEITEM.attr, ONEITEM.getName(index), ONEITEM.attr[index] (owner) 없음
        send_line(ob, "☞ 기연: 단일아이템 목록. (ONEITEM 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
