// 아이템삭제.rhai - 아이템 삭제 (관리자)
// Python: cmds/아이템삭제.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "사용법: [아이템 인덱스] 아이템삭제");
        return;
    }
    if delete_item_definition(line) {
        send_line(ob, "아이템이 삭제되었습니다.");
    } else {
        send_line(ob, "존재하지않는 아이템입니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
