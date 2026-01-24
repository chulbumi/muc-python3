// 기연초기화.rhai - 기연초기화. Python: cmds/기연초기화.py
// ONEITEM.clear(); 관리자 2000.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        // ONEITEM.clear() 해당 전역 없음
        send_line(ob, "* 기연아이템 목록이 초기화되었습니다. (ONEITEM 연동 시 반영)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
