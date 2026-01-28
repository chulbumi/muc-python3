// 호위.rhai - NPC 호위 보기
// Python: cmds/호위.py

fn cmd(ob, line) {
    let name = if is_empty(line) {
        "당신"
    } else {
        line
    };

    if is_empty(line) || line == "나" || line == get_string(ob, "이름") {
        // 자기 자신의 호위 보기
        // TODO: target.objs에서 '종류'=='호위'인 것들 찾기
        // 현재는 기본 메시지만 표시
        let 분노 = get_int(ob, "분노");
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        send_line(ob, "☞ 당신의 호위 상태");
        send_line(ob, "   호위수: 0, 분노: " + 분노);
        send_line(ob, "────────────────────────────");
        send_line(ob, "당신은 호위를 거느리지 않고 있습니다.");
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        return;
    }

    // 다른 대상의 호위 보기
    send_line(ob, "☞ 다른 플레이어의 호위는 현재 조회할 수 없습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
