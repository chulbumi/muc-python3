// 방제작.rhai - 방 제작 (관리자)
// Python: cmds/방제작.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^^*");
        return;
    }
    send_line(ob, "☞ 방 제작 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
