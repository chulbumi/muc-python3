// 비전.rhai - 비전(오의) 설정
// Python: cmds/비전.py
//
// 사용법:
//   비전 - 현재 설정된 비전 표시
//   비전 [무공명] - 비전 설정 (배운 무공만 가능)

fn cmd(ob, line) {
    // 인자가 없으면 현재 비전 표시
    if is_empty(line) {
        let current = get_string(ob, "비전설정");
        if current == "" {
            send_line(ob, "☞ 비전 : 없음");
        } else {
            send_line(ob, "☞ 비전 : [\x1b[1;37m" + current + "\x1b[0;37m]");
        }
        return;
    }

    let skill_name = line;

    // 해당 무공을 배웠는지 확인
    if !has_skill(ob, skill_name) {
        send_line(ob, "☞ 당신은 그런 비전을 배운적이 없습니다.");
        return;
    }

    // 비전 설정
    set(ob, "비전설정", skill_name);
    send_line(ob, "☞ 비전을 지정하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
