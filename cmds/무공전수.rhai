// 무공전수.rhai - 대상에게 무공 전수 (관리자 lv 2000)
// Python: cmds/무공전수.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] [무공이름] 무공전수");
        return;
    }

    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [무공이름] 무공전수");
        return;
    }

    let target_name = words[0];
    let mugong_name = words[1];

    // 대상이 자기 자신인지 확인
    let my_name = get_string(ob, "이름");
    if target_name == my_name {
        send_line(ob, "☞ 자기 자신에게 무공을 전수할 수 없습니다.");
        return;
    }

    // 무공 데이터 확인
    let mugong = get_skill_data(mugong_name);
    if mugong.is_unit() {
        send_line(ob, "☞ 그런 무공을 습득한 적이 없습니다.");
        return;
    }

    // 대상이 이미 해당 무공을 가지고 있는지 확인
    if check_player_skill(target_name, mugong_name) {
        send_line(ob, "☞ 이미 무공을 익히셨는걸요");
        return;
    }

    // 무공 전수
    let result = teach_skill_to_player(ob, target_name, mugong_name);
    if result == "not_found" {
        send_line(ob, "☞ 그런 대상이 없어요!");
    } else if result == "not_implemented" {
        send_line(ob, "☞ 무공 전수 기능은 현재 구현 중입니다.");
    } else if result == "ok" {
        send_line(ob, "☞ 무공이 전수되었습니다.");
    } else {
        send_line(ob, "☞ 전수 실패: " + result);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
