// 몹제거.rhai - 몹 제거 (관리자)
// Python: cmds/몹제거.py
// Admin level 2000

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if is_empty(line) {
        send_line(ob, "사용법: [몹 이름] 몹제거");
        return;
    }

    // 현재 방에서 몹 찾기
    let mob = find_mob_in_room(ob, line);
    if mob.is_unit() {
        send_line(ob, "그런 몹이 없어요!");
        return;
    }

    // 몹 데이터 확인
    let mob_name = get_string(mob, "이름");
    if mob_name == "" {
        send_line(ob, "그런 몹이 없어요!");
        return;
    }

    // 몹 제거 시도
    if remove_room_mob(ob, line) {
        send_line(ob, "몹이 제거되었습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
