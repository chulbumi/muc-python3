// 기부.rhai - 기부. Python: cmds/기부.py
// 표두(표국무사) 검색은 Rust efun이 담당하고, 상태 변경 결과의 문구는 여기서 출력한다.

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
