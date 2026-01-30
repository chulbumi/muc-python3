// 방파입문.rhai - 방파 입문 (방주가 대상을 방파에 입문시킴)
// Python: cmds/방파입문.py
// NOTE: 현재 guild_join efun은 호출자 본인만 가입 가능
//       대상을 입문시키는 기능은 guild_add_member efun이 필요하며
//       현재는 직접 파일을 수정해야 함 (미구현 상태로 메시지 표시)

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법 : [대상] 방파입문");
        return;
    }

    let my_position = get_attr(ob, "직위");
    if my_position != "방주" && my_position != "부방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }

    let my_name = get_attr(ob, "이름");
    if my_name == line {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }

    // 현재 방에 있는 플레이어 목록에서 대상 찾기
    let players = get_room_players(ob);
    let target_found = false;
    let i = 0;
    while i < len(players) {
        if players[i] == line {
            target_found = true;
            break;
        }
        i = i + 1;
    }

    if !target_found {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }

    let my_guild = get_attr(ob, "소속");
    if my_guild == "" {
        send_line(ob, "☞ 당신은 방파에 소속되어 있지 않습니다.");
        return;
    }

    // TODO: 입문신청자 확인 필요 (별도 속성 저장 필요)
    // TODO: guild_add_member efun이 필요함
    // 현재는 대상을 입문시킬 수 있는 efun이 없음

    send_line(ob, "☞ " + line + " 님을 " + my_guild + "에 입문시킵니다.");
    send_line(ob, "☞ (현재 대상 입문 기능은 추가 efun이 필요합니다)");
    send_line(ob, "☞ 임시조치: " + line + " 님에게 [방파이름] 가입 명령을 사용하도록 안내하세요");
}

fn main(ob, line) {
    cmd(ob, line);
}
