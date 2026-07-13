// 대여.rhai - 아이템 대여
// Python: cmds/대여.py

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [물품번호] 대여");
        return;
    }
    if !room_has_object(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let result = book_borrow(ob, parse_int_prefix(line));
    if result == "ok" { send_line(ob, "☞ 대여가 완료 되었습니다."); }
    else if result == "borrowed" { send_line(ob, "☞ 현재 대여중 입니다."); }
    else if result == "persist_failed" { send_line(ob, "☞ 대여 할 수 없습니다."); }
    else { send_line(ob, "☞ 대여 가능한 물품이 없습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
