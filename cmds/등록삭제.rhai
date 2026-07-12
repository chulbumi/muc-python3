// 등록삭제.rhai - 방파 등록 삭제
// Python: cmds/등록삭제.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [물품번호] 등록취소");
        return;
    }
    if !room_has_mob(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let result = book_delete(ob, to_int(line));
    if result == "ok" { send_line(ob, "☞ 등록 삭제 되었습니다."); }
    else if result == "not_owner" { send_line(ob, "☞ 자신이 등록한 물품이 아닙니다."); }
    else { send_line(ob, "☞ 등록 취소 가능한 물품이 없습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
