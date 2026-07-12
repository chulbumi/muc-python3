// 앞.rhai - 앞으로 이동 (관리자 명령: 대상 플레이어 위치로 순간이동)
// Python: cmds/앞.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if !require_arg(ob, line) { send_line(ob, "☞ 운영자 명령: [대상] 앞"); return; }
    let target = trim(line);
    let pos = get_position_of(target);
    if length(pos["zone"]) == 0 {
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^");
        return;
    }
    let my = get_my_position(ob);
    if my["zone"] == pos["zone"] && my["room"] == pos["room"] {
        send_line(ob, "☞ 같은 자리에요.");
        return;
    }
    let err = set_my_position(ob, pos["zone"], pos["room"]);
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
