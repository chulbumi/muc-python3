// 앞앞.rhai - Python: cmds/앞앞.py
// [대상] → get_position_of(이름), set_my_position(ob, zone, room). 같은자리/실패 시 메시지, 성공 시 view_map_data.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: [대상] 앞");
        return;
    }
    let t = get_position_of(trim(line));
    let zs = t["zone"].to_string();
    let rs = t["room"].to_string();
    if zs == "" {
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^");
        return;
    }
    let my = get_my_position(ob);
    if my["zone"] == zs && my["room"].to_string() == rs {
        send_line(ob, "☞ 같은 자리에요.");
        return;
    }
    clear_summon_combat(ob);
    set_my_position(ob, zs, rs);
}

fn main(ob, line) {
    cmd(ob, line);
}
