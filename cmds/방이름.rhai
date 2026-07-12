// 방이름.rhai - 방 이름 설정 (관리자)
// Python: cmds/방이름.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [이름] 방이름");
        return;
    }
    if set_obj_attr(ob, "방", "이름", line) {
        send_line(ob, "방이 이름이 변경 되었습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
