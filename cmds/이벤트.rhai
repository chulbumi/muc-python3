// 이벤트.rhai - 이벤트. Python: cmds/이벤트.py
// [대상] findObjName, target['이벤트설정리스트']. 관리자 1000.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 사용법: [대상] 이벤트");
            return;
        }
        send_line(ob, "☞ 그런 대상이 없어요! (findObjName/이벤트설정리스트 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
