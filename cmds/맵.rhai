// 맵.rhai - 맵 명령어 (관리자 2000, 제외방향 탐색은 추후)
// Python: cmds/맵.py - 현재는 출구 나침반으로 연동

fn cmd(ob, line) {
    if !require_admin(ob, 2000) { return; }
    let pos = get_my_position(ob);
    if length(pos["zone"]) == 0 {
        send_line(ob, "\r\n* 아무것도 보이지 않습니다.\r\n");
        return;
    }
    let exits = get_exits_string(pos["zone"], pos["room"]);
    if length(exits) == 0 || starts_with(exits, "  ○  어느 쪽으로도") {
        send_line(ob, "☞ 아무것도 보이지 않습니다.");
        return;
    }
    let rn = get_room_name(pos["zone"], pos["room"]);
    send_line(ob, "\r\n" + rn + "\r\n\r\n" + exits);
}

fn main(ob, line) {
    cmd(ob, line);
}
