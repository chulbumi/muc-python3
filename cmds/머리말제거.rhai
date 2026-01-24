// 머리말제거.rhai - 머리말제거. Python: cmds/머리말제거.py
// ob['머리말'] = '';

fn cmd(ob, line) {
    set_value(ob, "머리말", "");
    send_line(ob, "☞ 머리말을 제거 하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
