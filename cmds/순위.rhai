// 순위.rhai - [특성치] rank_get_all(ty). 은전 10만 차감. channel.players+Body 수집은 추후.

fn cmd(ob, line) {
    let ln = trim(line);
    if ln == "" {
        send_line(ob, "☞ 사용법: [특성치] 순위");
        return;
    }
    if get_int(ob, "관리자등급") < 1000 {
        let hay = " 힘 은전 레벨 체력 내공 민첩 맷집 명중 회피 필살 운 나이 ";
        if !contains(hay, " " + ln + " ") {
            send_line(ob, "☞ 사용법: [특성치] 순위");
            return;
        }
    }
    if get_int(ob, "은전") < 100000 {
        send_line(ob, "☞ 은전이 부족해요.");
        return;
    }
    set_value(ob, "은전", get_int(ob, "은전") - 100000);
    send_line(ob, rank_get_all(ln));
}

fn main(ob, line) {
    cmd(ob, line);
}
