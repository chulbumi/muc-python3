// 방파방이름.rhai - 방파 방 이름
// Python: cmds/방파방이름.py

fn cmd(ob, line) {
    let position = get_attr(ob, "직위");
    if position != "방주" && position != "부방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [이름] 방파방이름");
        return;
    }
    let owner = get_room_attr(ob, "방파주인");
    if owner == "" || owner != get_string(ob, "소속") {
        send_line(ob, "☞ 무림인은 아무곳에나 이름을 새기지 않는다네.");
        return;
    }
    if line.len() > 20 {
        send_line(ob, "☞ 너무 길어요.");
        return;
    }
    if !set_obj_attr(ob, "방", "이름", line) {
        send_line(ob, "☞ 무림인은 아무곳에나 이름을 새기지 않는다네.");
        return;
    }
    send_line(ob, "이름이 변경 되었습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
