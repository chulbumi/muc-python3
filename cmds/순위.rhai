// 순위.rhai - 순위. Python: cmds/순위.py
// [특성치] 순위. ob.channel.players, RANK, 은전 10만. get_all_online_players 유사하나 RANK/정렬/은전 차감 없음.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [특성치] 순위");
        return;
    }
    let hay = " 힘 은전 레벨 체력 내공 민첩 맷집 명중 회피 필살 운 나이 ";
    if !contains(hay, " " + line + " ") {
        send_line(ob, "☞ 사용법: [특성치] 순위");
        return;
    }
    send_line(ob, "☞ 은전이 부족해요. (은전 10만, RANK 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
