// 분노.rhai - 분노 시스템
// Python: cmds/분노.py
//
// 사용법:
//   분노 - 현재 분노도 표시
//   분노 [값] - 분노도 설정 (관리자 전용)

fn cmd(ob, line) {
    let in_combat = get_int(ob, "act") == 3; // ACT_FIGHT = 3

    // 인자가 없으면 현재 분노도 표시
    if is_empty(line) {
        let anger = get_anger(ob);
        send_line(ob, "☞ 현재 분노도: " + to_string(anger));
        if anger >= 100 {
            send_line(ob, "   (분노도 100 이상: 특수 스킬 사용 가능)");
        }
        return;
    }

    // 관리자 권한 확인
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    // 분노도 설정
    let value = to_int(line);
    if set_anger(ob, value) {
        send_line(ob, "☞ 분노도를 " + to_string(value) + "(으)로 설정했습니다.");
    } else {
        send_line(ob, "☞ 분노도 설정에 실패했습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
