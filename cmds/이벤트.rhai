// 이벤트.rhai - 이벤트 확인 (관리자)
// Python: cmds/이벤트.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 이벤트");
        return;
    }
    let events = get_obj_attr(ob, line, "이벤트설정리스트");
    if events.is_unit() {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }
    if events.is_string() {
        for event_line in split(events, "\r\n") {
            send_line(ob, event_line);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
