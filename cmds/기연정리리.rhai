// 기연정리리.rhai - 기연정리리. Python: cmds/기연정리리.py
// 기연정리와 동일 + ONEITEM.attr.__delitem__(index).

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 사용법: [기연이름] 기연정리");
            return;
        }
        send_line(ob, "☞ 기연 정리(상세): ONEITEM 연동 후 지원 예정.");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
