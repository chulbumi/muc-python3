// 꼬리말제거.rhai - 꼬리말제거. Python: cmds/꼬리말제거.py
// ob['꼬리말'] = ''; set_value로 Body에 반영.

fn cmd(ob, line) {
    set_value(ob, "꼬리말", "");
    send_line(ob, "☞ 꼬리말을 제거 하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
