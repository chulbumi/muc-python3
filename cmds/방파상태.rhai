// 방파상태.rhai - 방파상태. Python: cmds/방파상태.py
// ob['소속'], GUILD[ob['소속']], 부방주/장로/방파인.

fn cmd(ob, line) {
    let so = get_string(ob, "소속");
    if so == "" || so == () {
        send_line(ob, "☞ 당신은 소속이 없습니다.");
        return;
    }
    send_line(ob, "☞ (GUILD." + so + " 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
