// 생성.rhai - 관리자 아이템 생성 (Python: cmds/생성.py)
fn cmd(ob, line) {
    if !require_admin(ob, 2000) { send_line(ob, "☞ 관리자만 사용할 수 있어요."); return; }
    if !require_arg(ob, line) { send_line(ob, "사용법: [아이템 이름] [갯수] 생성"); return; }
    let args = line_args(line);
    let key = args[0];
    let cnt = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    if cnt < 1 { return; }
    if cnt > 100 { return; }

    let display = item_create(ob, key);
    if display == "" {
        send_line(ob, "* 생성 실패!!!");
        return;
    }
    let n = 1;
    while n < cnt {
        item_create(ob, key);
        n = n + 1;
    }
    send_line(ob, "\x1b[1;32m* [" + display + "] 생성 되었습니다.\x1b[0;37m");
}

fn main(ob, line) {
    cmd(ob, line);
}
