// 등록.rhai - 방파 등록
// Python: cmds/등록.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [물품] 등록");
        return;
    }
    if !room_has_mob(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let result = book_register(ob, line);
    if result == "ok" { send_line(ob, "☞ 등록 되었습니다."); }
    else if result == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); }
    else { send_line(ob, "☞ 등록할 수 없습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
