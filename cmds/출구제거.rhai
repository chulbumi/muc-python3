// 출구제거.rhai - [출구] 제거. exit_remove(ob, name). 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [출구] 출구제거");
        return;
    }
    if exit_remove(ob, trim(line)) {
        send_line(ob, "☞ 출구가 제거되었습니다.");
    } else {
        send_line(ob, "☞ 그런 출구가 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
