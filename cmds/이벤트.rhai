// 이벤트.rhai - Python: cmds/이벤트.py
// [대상] findObjName(line)→target, for l in target['이벤트설정리스트']: sendLine(l). findObjName/이벤트설정리스트 연동 예정.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 이벤트");
        return;
    }
    send_line(ob, "☞ 그런 대상이 없어요! (findObjName/이벤트설정리스트 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
