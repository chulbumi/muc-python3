// 원자적 런타임 월드 편집. 구조/권한/캐시 갱신은 Rust efun이,
// 관리자에게 보이는 사용법과 결과 문구는 이 핫리로드 스크립트가 담당한다.
fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let pair = python_split_once(trim(line));
    if len(pair) < 2 || is_empty(pair[0]) || is_empty(pair[1]) {
        send_line(ob, "☞ 사용법: [방|출구|몹|몹배치|아이템|고정물|이벤트] [JSON] 월드편집");
        return;
    }
    let outcome = world_edit_json(ob, pair[0], pair[1]);
    if outcome["ok"] {
        send_line(ob, "☞ 월드 편집이 즉시 반영되었습니다. (" + outcome["detail"] + ")");
    } else {
        send_line(ob, "☞ 월드 편집 실패: " + outcome["detail"]);
    }
}

fn main(ob, line) { cmd(ob, line); }
