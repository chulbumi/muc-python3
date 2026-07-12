// 이동동.rhai - 이동동 명령어 (관리자 2000: [존:방] 순간이동)
// Python: cmds/이동동.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [존이름:맵번호] 이동"); return; }
    let spec = parse_room_spec(line);
    if length(spec["zone"]) == 0 {
        send_line(ob, "* 이동 실패!!!");
        return;
    }
    let err = set_my_position(ob, spec["zone"], spec["room"]);
    if err == "fail" { send_line(ob, "* 이동 실패!!!"); return; }
    if err == "same_place" { send_line(ob, "☞ 같은 자리에요. ^^"); return; }
    if err != "" { return; }
    let r = view_map_data(ob);
    if r["ok"] { for row in split(r["text"], "\r\n") { send_line(ob, row); } }
    else { if r["err"] == "no_position" { send_line(ob, "\x1b[1;31m위치 정보가 없습니다.\x1b[0;37m"); } else if r["err"] == "room_error" { send_line(ob, "\x1b[1;31m방 정보를 읽을 수 없습니다.\x1b[0;37m"); } else if r["err"] == "unknown_room" { send_line(ob, "\x1b[1;37m[" + r["zone"] + ":" + int_to_str(r["room"]) + "]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n"); } }
}

fn main(ob, line) {
    cmd(ob, line);
}
