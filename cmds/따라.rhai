// 따라.rhai - 자동 따라가기 (Follow system)
// Python: cmds/따라.py

// 조사 생성기
fn han_iga(name) {
    // 한글 파라미터 처리를 위한 간단한 구현
    let last_char = name[name.len() - 1];
    // 한글 유니코드 범위: 가(0xAC00) ~ 힣(0xD7A3)
    // 받침 확인 로직은 간소화
    if last_char >= 0xAC00 && last_char <= 0xD7A3 {
        let code = last_char - 0xAC00;
        let has_jongseong = (code % 28) != 0;
        if has_jongseong { return "이"; }
        else { return "가"; }
    }
    return "가";
}

fn han_un(name) {
    let last_char = name[name.len() - 1];
    if last_char >= 0xAC00 && last_char <= 0xD7A3 {
        let code = last_char - 0xAC00;
        let has_jongseong = (code % 28) != 0;
        if has_jongseong { return "은"; }
        else { return "는"; }
    }
    return "는";
}

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 명령: 따라 [대상이름]");
        send_line(ob, "   (대상이 이동할 때 자동으로 따라감)");
        return;
    }

    let my_name = get_string(ob, "이름");

    // "나" 또는 자기 자신 이름 입력시: 따라가기 중지
    if line == "나" || line == my_name {
        let leader = get_leader(ob);

        if leader != "" {
            // 파티 소속 확인
            let my_party = get_string(ob, "소속파티");
            let party_leader_name = get_string(ob, "파티장");

            if my_party != "" && party_leader_name == to_string(to_int(my_party)) {
                // 파티 대장은 이탈할 수 없음
                send_line(ob, "무리의 대장은 이탈할수 없어요.");
                return;
            } else {
                // 파티 멤버인 경우: 파티 탈퇴 처리
                leave_party(ob);
                remove_follower(ob);

                // TODO: 파티장에게 메시지 전송 필요
                send_line(ob, "당신은 홀로 강호를 주유하기 시작합니다.");
                return;
            }
        } else {
            // 파티 없이 팔로우만 중지
            if remove_follower(ob) {
                send_line(ob, "당신은 홀로 강호를 주유하기 시작합니다.");
            } else {
                send_line(ob, "이미 혼자입니다.");
            }
            return;
        }
    }

    // 대상 확인
    let result = find_target(ob, line);

    if !result["found"].as_bool() {
        send_line(ob, "☞ 그런 대상이 없어요. ^^");
        return;
    }

    let target_name = result["to_target"]["name"].to_string();
    if target_name == "" {
        send_line(ob, "☞ 그런 대상이 없어요. ^^");
        return;
    }

    // TODO: 동행거부 설정 확인 필요

    // 기존 팔로우 제거 후 새로운 팔로우 추가
    remove_follower(ob);
    let err = add_follower(ob, target_name);

    if err != "" {
        send_line(ob, "☞ " + err);
    } else {
        send_line(ob, "당신은 " + target_name + han_un(target_name) + " 따라가기 시작했습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
