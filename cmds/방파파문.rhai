// 방파파문.rhai - 방파 파문 (방주가 멤버를 추방)
// Python: cmds/방파파문.py
// NOTE: 현재 guild_leave efun은 호출자 본인만 탈퇴 가능
//       대상을 파문시키는 기능은 guild_kick efun이 필요하며
//       현재는 미구현 상태로 메시지 표시

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법 : [대상] 방파파문");
        return;
    }

    let my_position = get_attr(ob, "직위");
    if my_position != "방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }

    let my_name = get_attr(ob, "이름");
    if my_name == line {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }

    let my_guild = get_attr(ob, "소속");
    if my_guild == "" {
        send_line(ob, "☞ 당신은 방파에 소속되어 있지 않습니다.");
        return;
    }

    // TODO: 대상이 같은 방파 소속인지 확인
    // TODO: guild_kick efun이 필요함
    // 현재는 대상을 파문시킬 수 있는 efun이 없음

    send_line(ob, "☞ " + line + " 님을 방파에서 파문시킵니다.");
    send_line(ob, "☞ (현재 대상 파문 기능은 추가 efun이 필요합니다)");
    send_line(ob, "☞ 임시조치: " + line + " 님에게 탈퇴 명령을 사용하도록 안내하세요");
}

fn main(ob, line) {
    cmd(ob, line);
}
