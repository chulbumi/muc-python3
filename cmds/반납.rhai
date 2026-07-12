// 반납.rhai - 대여 아이템 반납
// Python: cmds/반납.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [물품] 반납");
        return;
    }
    if !room_has_mob(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let result = book_return(ob, line);
    if result == "" { send_line(ob, "☞ 반납이 완료 되었습니다."); }
    else { send_line(ob, result); }
}

fn main(ob, line) {
    cmd(ob, line);
}
