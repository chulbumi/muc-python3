// 앞.rhai - 앞으로 이동 (관리자 명령: 대상 플레이어 위치로 순간이동)
// Python: cmds/앞.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { return; }
    if !require_arg(ob, line, "☞ 운영자 명령: [대상] 앞") { return; }
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
    if length(err) > 0 {
        send_line(ob, err);
        return;
    }
    view_map_data(ob);
}

fn main(ob, line) {
    cmd(ob, line);
}
