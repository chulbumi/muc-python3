// 이형환위.rhai - 이형환위. Python: cmds/이형환위.py
// [대상] '비학천룡', ob['위치각인'], getRoom, enterRoom.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 위치이동");
        return;
    }
    if line == "비학천룡" {
        let guards = get_inventory_guards(ob);
        if len(guards) != 0 && guards[0]["name"] != "비학천룡" {
            send_line(ob, "☞ 비학천룡이 없습니다.");
            return;
        }
        let idx = get_body_string(ob, "위치각인");
        if idx == "" {
            send_line(ob, "☞ 각인된 위치가 없습니다.");
            return;
        }
        let spec = parse_room_spec(idx);
        let z = spec["zone"];
        let r = spec["room"];
        let err = perform_summon_move(ob, z, r);
        if err == "fail" { send_line(ob, "* 이동 실패!!!"); return; }
        if err == "same_place" { send_line(ob, "☞ 같은 자리에요. ^^"); return; }
        if err != "" { return; }
        return;
    }
    send_line(ob, "☞ 어디로 이동하시려구요?");
}

fn main(ob, line) {
    cmd(ob, line);
}
