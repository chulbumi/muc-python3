// 이벤트.rhai - Python: cmds/이벤트.py
// [대상] findObjName(line)→target, for l in target['이벤트설정리스트']: sendLine(l). findObjName/이벤트설정리스트 미지원.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 이벤트");
        return;
    }
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
