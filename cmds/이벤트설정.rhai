// 이벤트설정.rhai - 이벤트 설정 (관리자 레벨 1000)
// Python: cmds/이벤트설정.py
// 같은 방의 대상에게 이벤트를 설정하는 관리자 명령어

fn cmd(ob, line) {
    // 관리자 권한 확인
    let admin_level = get_admin_level(ob);
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    // 명령어 파싱: [대상] [이벤트] 이벤트설정
    let words = python_split_once(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [이벤트] 이벤트설정");
        return;
    }

    let target_name = words[0];
    let event_key = words[1];

    let result = set_room_target_event(ob, target_name, event_key);
    if result == "missing" {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }
    if result == "already_set" {
        send_line(ob, "☞ 이미 설정되어 있습니다.");
        return;
    }
    send_line(ob, `☞ [${event_key}] 이벤트가 설정되었습니다.`);
}

fn main(ob, line) {
    cmd(ob, line);
}
