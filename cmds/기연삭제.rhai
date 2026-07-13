// 기연삭제.rhai - Python: cmds/기연삭제.py
// [기연이름] → ONEITEM.index[line], ONEITEM.attr.__delitem__(index), save. 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [기연이름] 기연삭제");
        return;
    }
    let idx = oneitem_get_index_by_name(line);
    if idx == "" {
        send_line(ob, "☞ 그런 아이템은 없습니다.!");
        return;
    }
    if !oneitem_destroy(idx) {
        send_line(ob, "☞ 그런 아이템은 없습니다.!");
        return;
    }
    send_line(ob, "☞ 기연이 삭제되었습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
