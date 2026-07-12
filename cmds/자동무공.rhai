// 자동무공.rhai - 자동 무공 조회/설정
// Python: cmds/자동무공.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        let current = get_body_string(ob, "자동무공");
        if current == "" {
            send_line(ob, "☞ 자동무공 : 없음");
        } else {
            send_line(ob, "☞ 자동무공 : [\x1b[1;37m" + current + "\x1b[0;37m]");
        }
        return;
    }

    let selected = "";
    for skill_name in get_skill_list(ob) {
        if skill_name == line {
            selected = skill_name;
            break;
        }
    }
    if selected == "" {
        for skill_name in get_skill_list(ob) {
            if starts_with(skill_name, line) {
                selected = skill_name;
                break;
            }
        }
    }

    if selected == "" {
        send_line(ob, "☞ 그런 무공을 습득한 적이 없습니다.");
        return;
    }
    let skill = get_skill_data(selected);
    if skill.is_unit() || get_string(skill, "종류") != "전투" {
        send_line(ob, "☞ 자동무공을 할 수 없는 무공입니다.");
        return;
    }

    set_value(ob, "자동무공", selected);
    send_line(ob, "☞ 자동무공을 지정하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
