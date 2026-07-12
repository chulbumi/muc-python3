// 방제거.rhai - 방 제거 (관리자)
// Python: cmds/방제거.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [방번호] 방제거");
        return;
    }
    if delete_room_definition(line) {
        send_line(ob, "방이 제거되었습니다.");
    } else {
        send_line(ob, "존재하지않는 방입니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
