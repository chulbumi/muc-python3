// 출구숨김.rhai - [출구] 숨김. exit_hide(ob, name). 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [출구] 출구숨김");
        return;
    }
    if exit_hide(ob, trim(line)) {
        send_line(ob, "☞ 출구가 숨겨졌습니다.");
    } else {
        send_line(ob, "☞ 그런 출구가 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
