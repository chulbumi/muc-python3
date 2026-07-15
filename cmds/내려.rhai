// 자유 배분 특성치는 명중/회피/필살/운 네 항목에서만 회수할 수 있다.

fn cmd(ob, line) {
    let ln = trim(line);
    if ln == "" {
        send_line(ob, "☞ 사용법: [명중|회피|필살|운] 내려");
        return;
    }
    let hay = " 명중 회피 필살 운 ";
    if !contains(hay, " " + ln + " ") {
        send_line(ob, "☞ 사용법: [명중|회피|필살|운] 내려");
        return;
    }
    let raw = get_attr(ob, ln + "특성치");
    let missing = raw.is_unit() || raw.to_string() == "";
    let x = if missing { 0 } else { to_int(raw.to_string()) };
    let base = if missing {
        get_int(ob, ln)
    } else {
        x
    };
    if base <= 0 {
        send_line(ob, "☞ [" + ln + "] 더이상 내릴 수 없습니다.");
        return;
    }
    let xval = base - 1;
    set_value(ob, ln + "특성치", xval);
    set_value(ob, "특성치", get_int(ob, "특성치") + 1);
    set_value(ob, ln, get_int(ob, ln) - 1);
    body_save(ob);
    send_line(ob, "☞ [" + ln + "] 특성치를 내렸습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
