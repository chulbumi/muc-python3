// Python `cmds/순위.py`: 현재 접속 중인 일반 사용자의 능력치 순위.

fn cmd(ob, line) {
    let attribute = line;
    if attribute == "" {
        send_line(ob, "☞ 사용법: [특성치] 순위");
        return;
    }
    if get_int(ob, "관리자등급") < 1000 {
        let allowed = ["힘", "은전", "레벨", "체력", "내공", "민첩", "맷집", "명중", "회피", "필살", "운", "나이"];
        if !allowed.contains(attribute) {
            send_line(ob, "☞ 사용법: [특성치] 순위");
            return;
        }
    }
    if attribute == "내공" { attribute = "최고내공"; }
    if attribute == "체력" { attribute = "최고체력"; }
    if attribute == "민첩" { attribute = "민첩성"; }
    if get_int(ob, "은전") < 100000 {
        send_line(ob, "☞ 은전이 부족해요.");
        return;
    }
    set_value(ob, "은전", get_int(ob, "은전") - 100000);
    let message = "";
    let count = 0;
    for entry in live_rank_entries(attribute) {
        count += 1;
        if get_int(ob, "관리자등급") >= 1000 {
            message += pad_start(entry["name"].to_string(), 10, " ") + " "
                + fill_space(13, entry["value"].to_string()) + "  ";
            if count % 3 == 0 { message += "\r\n"; }
        } else {
            message += "[" + pad_start(count.to_string(), 2, "0") + "] "
                + fill_space(10, entry["name"].to_string()) + " ";
            if count % 5 == 0 { message += "\r\n"; }
        }
    }
    send_line(ob, message);
}

fn main(ob, line) { cmd(ob, line); }
