// 대여.rhai - 아이템 대여
// Python: cmds/대여.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [물품번호] 대여");
        return;
    }
    if !room_has_mob(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let result = book_borrow(ob, to_int(line));
    if result == "" { send_line(ob, "☞ 대여가 완료 되었습니다."); }
    else { send_line(ob, result); }
}

fn main(ob, line) {
    cmd(ob, line);
}
