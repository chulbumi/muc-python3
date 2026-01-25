// 이형환위.rhai - 이형환위. Python: cmds/이형환위.py
// [대상] '비학천룡', ob['위치각인'], getRoom, enterRoom.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 위치이동");
        return;
    }
    if line == "비학천룡" {
        let idx = get_body_string(ob, "위치각인");
        if idx == "" {
            send_line(ob, "☞ 각인된 위치가 없습니다.");
            return;
        }
        let spec = parse_room_spec(idx);
        let z = spec["zone"];
        let r = spec["room"];
        let err = set_my_position(ob, z, r);
        if err == "fail" { send_line(ob, "* 이동 실패!!!"); return; }
        if err == "same_place" { send_line(ob, "☞ 같은 자리에요. ^^"); return; }
        if err != "" { return; }
        let r = view_map_data(ob);
        if r["ok"] { send_line(ob, r["text"]); }
        else { if r["err"] == "no_position" { send_line(ob, "\x1b[1;31m위치 정보가 없습니다.\x1b[0;37m"); } else if r["err"] == "room_error" { send_line(ob, "\x1b[1;31m방 정보를 읽을 수 없습니다.\x1b[0;37m"); } else if r["err"] == "unknown_room" { send_line(ob, "\x1b[1;37m[" + r["zone"] + ":" + int_to_str(r["room"]) + "]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n"); } }
        return;
    }
    send_line(ob, "☞ 어디로 이동하시려구요?");
}

fn main(ob, line) {
    cmd(ob, line);
}
