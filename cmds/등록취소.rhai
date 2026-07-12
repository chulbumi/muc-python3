// 등록취소.rhai - 방파 등록 취소
// Python: cmds/등록취소.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [물품번호] 등록취소");
        return;
    }
    if !room_has_mob(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let result = book_cancel(ob, to_int(line));
    if result == "" { send_line(ob, "☞ 등록 취소 되었습니다."); } else { send_line(ob, result); }
}

fn main(ob, line) {
    cmd(ob, line);
}
