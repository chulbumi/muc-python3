// 자유 배분 특성치는 명중/회피/필살/운 네 항목에만 사용할 수 있다.

fn cmd(ob, line) {
    let ln = line;
    if ln == "" {
        send_line(ob, "☞ 사용법: [명중|회피|필살|운] 올려");
        return;
    }
    let hay = " 명중 회피 필살 운 ";
    if !contains(hay, " " + ln + " ") {
        send_line(ob, "☞ 사용법: [명중|회피|필살|운] 올려");
        return;
    }
    if get_int(ob, "특성치") <= 0 {
        send_line(ob, "☞ 더이상 올릴 수 있는 여유 특성치가 없습니다.");
        return;
    }
    if get_int(ob, ln) >= 100 {
        send_line(ob, "☞ 더이상 올릴 수 없습니다.");
        return;
    }
    let key = ln + "특성치";
    let x = get_int(ob, key);
    let base = if python_value_is_empty(ob, key) { get_int(ob, ln) } else { x };
    let xval = base + 1;
    set_value(ob, ln + "특성치", xval);
    set_value(ob, "특성치", get_int(ob, "특성치") - 1);
    set_value(ob, ln, get_int(ob, ln) + 1);
    body_save(ob);
    send_line(ob, "☞ [" + ln + "] 특성치를 올렸습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
