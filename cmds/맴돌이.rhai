// 맴돌이.rhai - [출구] 목적지를 자기 방으로. exit_set_wander(ob, name). 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [출구] 맴돌이");
        return;
    }
    if exit_set_wander(ob, trim(line)) {
        send_line(ob, "☞ 출구가 맴돌이 되었습니다.");
    } else {
        send_line(ob, "☞ 그런 출구가 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
