// 등록.rhai - 방파 등록
// Python: cmds/등록.py

fn cmd(ob, line) {
    let item_name = trim(line);
    if item_name == "" {
        send_line(ob, "☞ 사용법: [물품] 등록");
        return;
    }
    if !room_has_object(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let parsed = parse_order_name(item_name);
    let result = book_register(ob, parsed[1], parsed[0]);
    if result == "ok" { send_line(ob, "☞ 등록 되었습니다."); }
    else if result == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); }
    else { send_line(ob, "☞ 등록할 수 없습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
