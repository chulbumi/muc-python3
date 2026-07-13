// 올려.rhai - Python: cmds/올려.py
// [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 특성치 1 소모, ob[line], ob[line+'특성치'] 증가. 명중/회피/필살/운 100 상한, 민첩성 2800 상한.

fn cmd(ob, line) {
    let ln = line;
    if ln == "" {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 올려");
        return;
    }
    let hay = " 힘 민첩성 맷집 명중 회피 필살 운 내공 체력 ";
    if !contains(hay, " " + ln + " ") {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 올려");
        return;
    }
    if get_int(ob, "특성치") == 0 {
        send_line(ob, "☞ 더이상 올릴 수 있는 여유 특성치가 없습니다.");
        return;
    }
    let l1 = " 명중 회피 필살 운 ";
    if contains(l1, " " + ln + " ") && get_int(ob, ln) >= 100 {
        send_line(ob, "☞ 더이상 올릴 수 없습니다.");
        return;
    }
    if ln == "민첩성" && get_int(ob, "민첩성") >= 2800 {
        send_line(ob, "☞ 더이상 올릴 수 없습니다.");
        return;
    }
    let key = ln + "특성치";
    let x = get_int(ob, key);
    let base = if python_value_is_empty(ob, key) && contains(l1, " " + ln + " ") { get_int(ob, ln) } else { x };
    let xval = base + 1;
    set_value(ob, ln + "특성치", xval);
    set_value(ob, "특성치", get_int(ob, "특성치") - 1);
    if ln == "내공" {
        set_value(ob, "최고내공", get_int(ob, "최고내공") + 10);
    } else if ln == "체력" {
        set_value(ob, "최고체력", get_int(ob, "최고체력") + 100);
    } else {
        set_value(ob, ln, get_int(ob, ln) + 1);
    }
    body_save(ob);
    send_line(ob, "☞ [" + ln + "] 특성치를 올렸습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
