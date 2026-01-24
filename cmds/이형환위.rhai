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
        if err != "" {
            send_line(ob, err);
            return;
        }
        view_map_data(ob);
        return;
    }
    send_line(ob, "☞ 어디로 이동하시려구요?");
}

fn main(ob, line) {
    cmd(ob, line);
}
