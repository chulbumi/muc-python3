// 이동동.rhai - 이동동 명령어 (관리자 2000: [존:방] 순간이동)
// Python: cmds/이동동.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) { return; }
    if !require_arg(ob, line, "☞ 사용법: [존이름:맵번호] 이동") { return; }
    let spec = parse_room_spec(line);
    if length(spec["zone"]) == 0 {
        send_line(ob, "* 이동 실패!!!");
        return;
    }
    let err = set_my_position(ob, spec["zone"], spec["room"]);
    if length(err) > 0 {
        send_line(ob, err);
        return;
    }
    view_map_data(ob);
}

fn main(ob, line) {
    cmd(ob, line);
}
