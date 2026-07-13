// 반납.rhai - 대여 아이템 반납
// Python: cmds/반납.py

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [물품] 반납");
        return;
    }
    if !room_has_object(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let result = book_return(ob, line);
    if result == "ok" { send_line(ob, "☞ 반납이 완료 되었습니다."); }
    else if result == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); }
    else if result == "not_returnable" { send_line(ob, "☞ 반납 가능한 물품이 아닙니다."); }
    else if result == "catalog_unavailable" { send_line(ob, "☞ 반납 가능한 물품이 없습니다."); }
    else { send_line(ob, "☞ 반납 할 수 없습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
