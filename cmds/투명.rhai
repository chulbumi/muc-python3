// 투명.rhai - 투명. Python: cmds/투명.py
// ob['투명상태'] 0→1, 1→0. 관리자 1000.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        let v = get_int(ob, "투명상태");
        if v == 0 {
            set_value(ob, "투명상태", 1);
            send_line(ob, "☞ 투명상태가 되었습니다");
        } else {
            set_value(ob, "투명상태", 0);
            send_line(ob, "☞ 투명상태가 해제되었습니다");
        }
    } else {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
