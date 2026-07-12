// 투명.rhai - 투명. Python: cmds/투명.py
// ob['투명상태'] 0→1, 1→0. 관리자 1000.

fn cmd(ob, line) {
    send_line(ob, int_to_str(get_timestamp()));
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let v = get_int(ob, "투명상태");
    if v == 0 {
        set_value(ob, "투명상태", 1);
        send_line(ob, "☞ 투명상태가 되었습니다");
    } else {
        set_value(ob, "투명상태", 0);
        send_line(ob, "☞ 투명상태가 해제되었습니다");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
