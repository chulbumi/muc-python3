// 지도.rhai - 지도 명령어 (현재 방 출구 나침반)
// Python: cmds/지도.py (11x12 recursive room grid)

fn cmd(ob, line) {
    let pos = get_my_position(ob);
    if length(pos["zone"]) == 0 {
        send_line(ob, "\r\n* 아무것도 보이지 않습니다.\r\n");
        return;
    }
    let map_text = get_python_map(pos["zone"], pos["room"]);
    if length(map_text) == 0 {
        send_line(ob, "☞ 아무것도 보이지 않습니다.");
        return;
    }
    send_line(ob, map_text);
}

fn main(ob, line) {
    cmd(ob, line);
}
