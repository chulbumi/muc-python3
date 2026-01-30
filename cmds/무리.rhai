// 무리.rhai - 무리/파티 시스템
// Python: cmds/무리.py
// 사용법: [대상이름/모두] 무리  (또는 그냥 '무리'로 조회)

// 조사 생성기
fn han_iga(name) {
    let last_char = name[name.len() - 1];
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

// HP 바 생성
fn get_hp_bar(ob) {
    let hp = get_int(ob, "hp");
    let max_hp = get_int(ob, "max_hp");
    if max_hp <= 0 { max_hp = 100; }
    if hp < 0 { hp = 0; }

    let percent = (hp * 100) / max_hp;
    if percent > 100 { percent = 100; }

    // 간단한 HP 바 표시
    let bars = "==========";
    let filled = percent / 10;
    let bar = "[" + bars[0..filled] + " " + bars[filled..10] + "]";

    return " " + to_string(percent) + "% " + bar;
}

// 파티/무리 현황 표시
fn view_party(ob) {
    let name = get_string(ob, "이름");
    let nick = get_string(ob, "무림별호");
    if nick == "" { nick = "무명객"; }
    let personality = get_string(ob, "성격");

    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 내 파티 확인
    let my_party = get_string(ob, "소속파티");
    let is_leader = (my_party == name);

    if is_leader {
        send_line(ob, "◁ " + name + "의 무리 ▷");
    } else if my_party != "" {
        send_line(ob, "◁ " + my_party + "의 무리 ▷");
    } else {
        send_line(ob, "◁ " + name + "의 동행 ▷");
    }
    send_line(ob, "────────────────────────────");

    // 파티 멤버 표시
    let members = get_party_members(ob);
    if len(members) > 0 {
        // 파티장(본인) 표시
        if personality != "기인" {
            send_line(ob, "▶ " + nick + " " + name + get_hp_bar(ob));
        } else {
            send_line(ob, "▶ " + nick + " " + name + get_hp_bar(ob));
        }

        // 다른 멤버 표시
        for i in 1..len(members) {
            // TODO: 다른 멤버의 상세 정보를 가져오는 방법 필요
            // 현재는 이름만 표시
            send_line(ob, "　 " + members[i]);
        }
    }

    // 팔로워(동행중) 표시
    send_line(ob, "동행중");
    // TODO: follower 목록 가져오는 efun 필요
    // 현재는 표시 불가

    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

// 무리 추가 (파티 멤버 초대)
fn add_party(ob, line) {
    let my_name = get_string(ob, "이름");
    let my_party = get_string(ob, "소속파티");

    // 이미 다른 파티에 소속된 경우
    if my_party != "" && my_party != my_name {
        send_line(ob, "☞ 이미 당신은 무리중이여서 무리를 만들 수 없어요. ^^");
        return;
    }

    // 내가 파티장이 아니면 새로운 파티 생성
    if my_party == "" {
        let err = create_party(ob, my_name);
        if err != "" {
            send_line(ob, "☞ " + err);
            return;
        }
    }

    // TODO: follower에서 대상 찾아 파티에 추가
    // 현재는 follower 관리를 위한 efun이 부족하여 구현 제한
    send_line(ob, "☞ 무리 추가 기능은 follower 시스템과 연동되어야 합니다.");
    send_line(ob, "   (대상: '" + line + "')");
}

fn cmd(ob, line) {
    if is_empty(line) {
        // 무리 조회
        let my_party = get_string(ob, "소속파티");
        let members = get_party_members(ob);

        if my_party == "" && len(members) == 0 {
            send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^");
        } else {
            view_party(ob);
        }
        return;
    }

    // 무리 추가
    add_party(ob, line);
}

fn main(ob, line) {
    cmd(ob, line);
}
