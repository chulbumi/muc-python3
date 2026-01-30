// 무리제외.rhai - 무리에서 제외 (파티 탈퇴/멤버 추방)
// Python: cmds/무리제외.py

// 조사 생성기
fn han_obj(name) {
    let last_char = name[name.len() - 1];
    if last_char >= 0xAC00 && last_char <= 0xD7A3 {
        let code = last_char - 0xAC00;
        let has_jongseong = (code % 28) != 0;
        if has_jongseong { return "을"; }
        else { return "를"; }
    }
    return "를";
}

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [동행원|무리원] 제외");
        return;
    }

    let my_name = get_string(ob, "이름");
    let my_party = get_string(ob, "소속파티");

    // 파티 소속 확인
    if my_party == "" {
        send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^");
        return;
    }

    // 파티장인지 확인
    let is_leader = (my_party == my_name);

    if is_leader {
        // 파티장이 멤버를 제외시키는 경우
        // TODO: 파티 멤버 목록에서 해당 멤버 찾아 제외
        // 현재는 개별 플레이어 데이터에 접근하는 efun이 부족하여 구현 제한

        // "모두" 입력시 전체 해제
        if line == "모두" {
            // TODO: 모든 파티 멤버 추방
            send_line(ob, "☞ 전체 해제 기능은 파티 멤버 관리 efun이 필요합니다.");
            return;
        }

        send_line(ob, "☞ 멤버 제외 기능은 파티 멤버 조회 efun이 필요합니다.");
        send_line(ob, "   (대상: '" + line + "')");
    } else {
        // 파티 멤버가 파티를 탈퇴하는 경우
        if line == "나" || line == my_name {
            if leave_party(ob) {
                remove_follower(ob);
                send_line(ob, "당신은 무리에서 이탈하였습니다.");

                // TODO: 파티장에게 알림 전송 필요
            } else {
                send_line(ob, "☞ 파티 탈퇴에 실패했습니다.");
            }
        } else {
            send_line(ob, "☞ 파티장만이 다른 멤버를 제외시킬 수 있습니다.");
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
