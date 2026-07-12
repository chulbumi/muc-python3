// 사용자몹제거.rhai - 사용자 생성 몹 삭제 (현재 방)
// Python: cmds/사용자몹제거.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "사용법: [사용자 이름] 사용자몹제거");
        return;
    }
    if remove_room_user_mob(ob, line) {
        send_line(ob, "사용자몹이 제거되었습니다.");
    } else {
        send_line(ob, "그런 몹이 없어요!");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
