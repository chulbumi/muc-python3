// 순위.rhai - [특성치] rank_get_all(ty). 은전 10만 차감.

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
    // Python cmds/순위.py의 사용자-facing 특성치 별칭 변환.
    if ln == "내공" { ln = "최고내공"; }
    if ln == "체력" { ln = "최고체력"; }
    if ln == "민첩" { ln = "민첩성"; }
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
