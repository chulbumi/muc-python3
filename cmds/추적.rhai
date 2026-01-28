// 추적.rhai - 몹 추적 (관리자 lv 1000)
// Python: cmds/추적.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 사용법: [몹이름] [존이름] 추적");
        return;
    }

    let words = split(line, " ");
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [몹이름] [존이름] 추적");
        return;
    }

    let mob_name = words[0];
    let zone_name = words[1];

    // TODO: 존의 모든 방을 순회하며 몹 찾기
    // 현재는 메시지로 안내
    send_line(ob, "☞ 추적 기능은 현재 구현 중입니다.");
    send_line(ob, "   (몹: " + mob_name + ", 존: " + zone_name + ")");
}

fn main(ob, line) {
    cmd(ob, line);
}
