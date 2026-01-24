// 지도.rhai - 지도 명령어 (현재 방 출구 나침반)
// Python: cmds/지도.py (간소화: 그리드 대신 출구 문자열)

fn cmd(ob, line) {
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
