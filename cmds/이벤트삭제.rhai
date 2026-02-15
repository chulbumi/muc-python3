// 이벤트삭제.rhai - 이벤트 삭제 (관리자 레벨 1000)
// Python: cmds/이벤트삭제.py
// 대상 몹에게서 이벤트를 삭제하는 관리자 명령어

fn cmd(ob, line) {
    // 관리자 권한 확인
    let admin_level = get_admin_level(ob);
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    // 명령어 파싱: [대상] [이벤트] 이벤트삭제
    let words = split(line, " ");
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [이벤트] 이벤트삭제");
        return;
    }

    let target_name = words[0];
    let event_key = words[1];

    // 현재 방에서 대상 몹 찾기
    let mob_info = find_mob_in_room(ob, target_name);
    if mob_info == () {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }

    // mob_key 가져오기
    let mob_key = mob_info["mob_key"];

    // 이벤트가 설정되어 있는지 확인
    if !check_mob_event(mob_key, event_key) {
        send_line(ob, `☞ [${event_key}] 이벤트는 설정되어있지 않습니다.`);
        return;
    }

    // 이벤트 삭제
    if del_mob_event(mob_key, event_key) {
        send_line(ob, `☞ [${event_key}] 이벤트가 삭제되었습니다.`);
    } else {
        send_line(ob, "☞ 이벤트 삭제에 실패했습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
