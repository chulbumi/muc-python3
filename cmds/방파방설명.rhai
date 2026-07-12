// 방파방설명.rhai - 방파 방 설명
// Python: cmds/방파방설명.py

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
    begin_room_description(ob, "방 설명 작성을 마치시려면 '.' 를 입력하세요.\r\n:");
}

fn main(ob, line) {
    cmd(ob, line);
}
