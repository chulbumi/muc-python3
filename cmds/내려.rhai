// 내려.rhai - Python: cmds/내려.py
// [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] ob[line+'특성치']--, ob['특성치']++, ob[line] 또는 최고내공/최고체력--.

fn cmd(ob, line) {
    let ln = trim(line);
    if ln == "" {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려");
        return;
    }
    let hay = " 힘 민첩성 맷집 명중 회피 필살 운 내공 체력 ";
    if !contains(hay, " " + ln + " ") {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려");
        return;
    }
    let x = get_int(ob, ln + "특성치");
    let base = if x == 0 && contains(" 명중 회피 필살 운 ", " " + ln + " ") { get_int(ob, ln) } else { x };
    if base <= 0 {
        send_line(ob, "☞ [" + ln + "] 더이상 내릴 수 없습니다.");
        return;
    }
    let xval = base - 1;
    set_value(ob, ln + "특성치", xval);
    set_value(ob, "특성치", get_int(ob, "특성치") + 1);
    if ln == "내공" {
        set_value(ob, "최고내공", get_int(ob, "최고내공") - 10);
    } else if ln == "체력" {
        set_value(ob, "최고체력", get_int(ob, "최고체력") - 100);
    } else {
        set_value(ob, ln, get_int(ob, ln) - 1);
    }
    body_save(ob);
    send_line(ob, "☞ [" + ln + "] 특성치를 내렸습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
