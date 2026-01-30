// 몹생성.rhai - 몹 생성 (관리자)
// Python: cmds/몹생성.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if is_empty(line) {
        send_line(ob, "사용법: [몹 이름] 생성");
        return;
    }

    // 몹 데이터 확인
    let mob_data = get_mob_data(line);
    if mob_data.is_unit() {
        send_line(ob, "* 생성 실패!!!");
        return;
    }

    // 몹 이름 확인
    let mob_name = get_string(mob_data, "이름");
    if mob_name == "" {
        send_line(ob, "* 생성 실패!!!");
        return;
    }

    // TODO: 실제 몹 생성 로직 (create_user_mob 또는 create_mob)
    // 현재 efun은 스텁으로만 구현됨
    let result = create_user_mob(ob, line);
    if result == "" {
        send_line(ob, "\x1b[1;32m* [" + mob_name + "] 생성 되었습니다.\x1b[0;37m");
    } else {
        send_line(ob, "* " + result);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
