// 순위.rhai - Python: cmds/순위.py
// [특성치] 비관리자: 힘,은전,레벨,체력,내공,민첩,맷집,명중,회피,필살,운,나이. line→최고내공/최고체력/민첩성 매핑. 은전 10만, channel.players에서 c[line] 수집·정렬·출력. get_all_online에 Body특성치 없음→연동 예정.

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
    send_line(ob, "☞ (channel.players+Body특성치/순위 출력 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
