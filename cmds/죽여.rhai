// 죽여.rhai - 몹 처치 (관리자 lv 2000)
// Python: cmds/죽여.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 죽여");
        return;
    }

    // 몹 찾기 (현재 방에서)
    let mob = find_mob_in_room(ob, line);
    if mob.is_unit() {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }

    // 공격 금지 확인
    if get_bool(mob, "공격금지") {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }

    // 몹 처치 시도
    if kill_mob(ob, line) {
        let name = get_string(ob, "이름");
        send_line(ob, "당신이 " + line + "을(를) 처치했습니다.");
    } else {
        send_line(ob, "☞ 처치할 수 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
