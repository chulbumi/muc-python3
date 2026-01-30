// 무공.rhai - 무공 보기/사용
// Python: cmds/무공.py
//
// 사용법:
//   무공 - 배운 무공 목록 표시
//   무공 [스킬명] [대상] - 무공 사용

fn cmd(ob, line) {
    // 인자가 없으면 무공 목록 표시
    if is_empty(line) {
        show_skill_list(ob);
        return;
    }

    // 관리자가 다른 사람의 무공을 볼 수 있음
    if require_admin(ob, 1000) {
        let words = line_args(line);
        if len(words) >= 1 {
            let target_name = words[0];
            // TODO: 다른 플레이어의 무공 보기 기능 구현 필요
            let target = get_player_by_name(target_name);
            if !target.is_unit() {
                send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                send_line(ob, "◁ " + target_name + "의 무공 ▷");
                show_other_skill_list(ob, target);
                return;
            }
        }
    }

    // 무공 사용
    use_skill_command(ob, line);
}

fn show_skill_list(ob) {
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    send_line(ob, "◁ 당신의 무공 ▷");
    send_line(ob, "───────────────────────────────────────");

    let skills = get_skill_list(ob);

    if len(skills) == 0 {
        send_line(ob, "☞ 깨우친 무공이 없습니다.");
    } else {
        let count = 0;
        for skill in skills {
            let skill_name = to_string(skill);
            let level = get_skill_level(ob, skill_name);
            let buf = skill_name + "(" + to_string(level) + "성)";
            send_line(ob, " ◇ " + buf);
            count += 1;
        }
    }

    send_line(ob, "───────────────────────────────────────");
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn show_other_skill_list(ob, target) {
    // TODO: get_skill_list는 다른 플레이어에게는 사용 불가
    // player_has_skill 등을 사용하여 구현 필요
    send_line(ob, "☞ 다른 플레이어의 무공 보기 기능은 구현 중입니다.");
}

fn use_skill_command(ob, line) {
    let words = line_args(line);
    if len(words) == 0 {
        send_line(ob, "☞ 사용법: 무공 [스킬명] 또는 무공 [스킬명] [대상]");
        return;
    }

    let skill_name = words[0];
    let target = if len(words) >= 2 { words[1] } else { "" };

    // 무공 사용
    let result = use_skill(ob, skill_name, target);

    if result != "" {
        send_line(ob, "☞ " + result);
    } else {
        // 성공 메시지는 use_skill에서 처리
        send_line(ob, "당신은 " + skill_name + "을(를) 시전합니다!");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
