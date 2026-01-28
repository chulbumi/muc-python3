// 이벤트.rhai - 이벤트 확인 (관리자)
// Python: cmds/이벤트.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 이벤트");
        return;
    }
    send_line(ob, "☞ 이벤트 확인 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
