// 방설명.rhai - 방 설명 설정 (관리자)
// Python: cmds/방설명.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^");
        return;
    }
    begin_room_description(ob, "방 설명 작성을 마치시려면 '.' 를 입력하세요.\r\n:");
}

fn main(ob, line) {
    cmd(ob, line);
}
