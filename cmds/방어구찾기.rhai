// 방어구찾기.rhai - 방어구 찾기 (관리자)
// Python: cmds/방어구찾기.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    send_line(ob, "☞ 방어구 찾기 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
