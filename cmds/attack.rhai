// attack.rhai - 공격 명령어
// Python: cmds/attack.py
//
// 사용법: attack [대상]
// 설명: 대상을 공격합니다. "쳐" 명령어와 동일합니다.
//
// 참고: 이 명령어는 Rust 내장(combat.rs)으로 처리됩니다.
// 아래 구현은 스크립트 참조용입니다.

fn cmd(ob, line) {
    let hp = get_int(ob, "hp");
    let name = get_string(ob, "이름");

    // 체력 확인
    if hp < 10 {
        send_line(ob, "☞ 체력이 너무 부족하여 공격할 수 없습니다.");
        return;
    }

    // 대상 확인
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 공격");
        return;
    }

    // "시체"는 공격 불가
    if contains(line, "시체") {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }

    // 몹 찾기
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

    // 몹종류 확인
    let mob_type = get_int(mob, "몹종류");
    if mob_type != 1 {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }

    // 이미 전투 중인지 확인
    let in_combat = get_int(ob, "act") == 3; // ACT_FIGHT = 3
    if in_combat {
        send_line(ob, "☞ 이미 공격중이에요. ^_^");
        return;
    }

    // TODO: 실제 전투 시작 로직은 Rust combat.rs에서 처리
    // start_fight, do_attack, calculate_damage efuns 필요
    let iga = ob_iga(ob);
    send_line(ob, "당신이 공격 자세를 취합니다!");
    send_room(ob, name + iga + " 공격 자세를 취합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
