// 위치각인.rhai - 위치각인. Python: cmds/위치각인.py
// [대상] '비학천룡'이면 getGuardNum, ob['위치각인']=ob.env.index. get_my_position으로 zone:room.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 위치각인");
        return;
    }
    if line == "비학천룡" {
        let pos = get_my_position(ob);
        let s = pos["zone"] + ":" + int_to_str(pos["room"]);
        set_value(ob, "위치각인", s);
        send_line(ob, "☞ 현재 위치가 각인되었습니다.");
        return;
    }
    send_line(ob, "☞ 어디에 각인하시려구요?");
}

fn main(ob, line) {
    cmd(ob, line);
}
