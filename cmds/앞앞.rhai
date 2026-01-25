// 앞앞.rhai - Python: cmds/앞앞.py
// [대상] → get_position_of(이름), set_my_position(ob, zone, room). 같은자리/실패 시 메시지, 성공 시 view_map_data.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: [대상] 앞");
        return;
    }
    let t = get_position_of(trim(line));
    let zs = t["zone"].to_string();
    let rs = t["room"].to_int();
    if zs == "" {
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^");
        return;
    }
    let ret = set_my_position(ob, zs, rs);
    if ret == "fail" { send_line(ob, "* 이동 실패!!!"); return; }
    if ret == "same_place" { send_line(ob, "☞ 같은 자리에요. ^^"); return; }
    if ret != "" { return; }
    let r = view_map_data(ob);
    if r["ok"] { send_line(ob, r["text"]); }
    else { if r["err"] == "no_position" { send_line(ob, "\x1b[1;31m위치 정보가 없습니다.\x1b[0;37m"); } else if r["err"] == "room_error" { send_line(ob, "\x1b[1;31m방 정보를 읽을 수 없습니다.\x1b[0;37m"); } else if r["err"] == "unknown_room" { send_line(ob, "\x1b[1;37m[" + r["zone"] + ":" + int_to_str(r["room"]) + "]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n"); } }
}

fn main(ob, line) {
    cmd(ob, line);
}
