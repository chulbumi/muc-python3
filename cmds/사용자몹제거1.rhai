// 사용자몹제거1.rhai - 사용자 생성 몹 삭제 (대상 지정)
// Python: cmds/사용자몹제거1.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 사용자몹제거1 [대상]");
        return;
    }
    if remove_user_mob(ob, line) {
        send_line(ob, "사용자몹이 제거되었습니다.");
    } else {
        send_line(ob, "그런 몹이 없어요!");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
