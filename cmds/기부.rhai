// 기부.rhai - 기부. Python: cmds/기부.py
// 표두(표국무사)에게 은전 기부. find_target/find_mob_in_room 없음 → 사용법 + 조건 메시지.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [금액] 기부");
        return;
    }
    let m = to_int(line);
    if m <= 0 {
        send_line(ob, "☞ 은전 1개 이상 입금 하셔야 해요.");
        return;
    }
    let have = get_int(ob, "은전");
    if have < m {
        send_line(ob, "☞ 소지한 은전이 부족해요.");
        return;
    }
    // 표두(표국무사) in room: find_target은 플레이어만. 몹 검색/표두 전용 efun 없음.
    send_line(ob, "☞ 이곳에 표국무사가 없네요.");
}

fn main(ob, line) {
    cmd(ob, line);
}
