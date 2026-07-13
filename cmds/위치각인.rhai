// 위치각인.rhai - 위치각인. Python: cmds/위치각인.py
// [대상] '비학천룡'이면 getGuardNum, ob['위치각인']=ob.env.index. get_my_position으로 zone:room.

fn cmd(ob, line) {
    let target = trim(line);
    if target == "" {
        send_line(ob, "☞ 사용법: [대상] 위치각인");
        return;
    }
    if target == "비학천룡" {
        let guards = get_inventory_guards(ob);
        if len(guards) != 0 && guards[0]["name"] != "비학천룡" {
            send_line(ob, "☞ 비학천룡이 없습니다.");
            return;
        }
        let pos = get_my_position(ob);
        let s = pos["zone"].to_string() + ":" + pos["room"].to_string();
        set_value(ob, "위치각인", s);
        send_line(ob, "☞ 현재 위치가 각인되었습니다.");
        return;
    }
    send_line(ob, "☞ 어디에 각인하시려구요?");
}

fn main(ob, line) {
    cmd(ob, line);
}
