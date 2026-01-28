// 무리.rhai - 무리/파티 시스템
// Python: cmds/무리.py
// 사용법: [대상이름/모두] 무리  (또는 그냥 '무리'로 조회)

fn view_party(ob) {
    // 파티/무리 현황 표시
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let name = get_string(ob, "이름");
    let nick = get_string(ob, "무림별호");
    if nick == "" { nick = "무명객"; }
    send_line(ob, "◁ " + name + "의 무리 ▷");
    send_line(ob, "────────────────────────────");

    // TODO: PartyMember 목록 표시
    let members = get_party_members(ob);
    if len(members) == 0 {
        send_line(ob, "무리원이 없습니다.");
    } else {
        for m in members {
            send_line(ob, "　 " + m);
        }
    }

    // TODO: follower 목록 표시
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn cmd(ob, line) {
    if is_empty(line) {
        // 무리 조회
        let in_party = is_party_leader(ob);
        let has_members = len(get_party_members(ob)) > 0;

        if !in_party && !has_members {
            send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^");
        } else {
            view_party(ob);
        }
        return;
    }

    // 무리 추가
    send_line(ob, "☞ 무리 추가 기능은 현재 구현 중입니다.");
    // TODO: follower에서 대상 찾아 PartyMember에 추가
}

fn main(ob, line) {
    cmd(ob, line);
}
