// 방파별호.rhai - 방파 별호 설정 (방주가 멤버에게 별호 부여)
// Python: cmds/방파별호.py
// NOTE: 대상 플레이어의 '방파별호' 속성을 설정해야 함
//       현재는 guild_set_nickname efun이 필요하며 미구현 상태

fn cmd(ob, line) {
    let my_guild = get_attr(ob, "소속");
    if my_guild == "" {
        send_line(ob, "☞ 당신은 소속이 없습니다.");
        return;
    }

    let my_position = get_attr(ob, "직위");
    if my_position != "방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 사용법 : [대상] [무림별호] 방파별호");
        return;
    }

    let args = line_args(line);
    if len(args) < 2 {
        send_line(ob, "☞ 사용법 : [대상] [무림별호] 방파별호");
        return;
    }

    let target = args[0];
    let nickname = args[1];

    if length(nickname) > 10 {
        send_line(ob, "☞ 사용하시려는 별호가 너무 길어요.");
        return;
    }

    let my_name = get_attr(ob, "이름");

    // 현재 방에 있는 플레이어 목록에서 대상 찾기
    let players = get_room_players(ob);
    let target_found = false;
    let i = 0;
    while i < len(players) {
        if players[i] == target {
            target_found = true;
            break;
        }
        i = i + 1;
    }

    if !target_found {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }

    // TODO: 대상이 같은 방파 소속인지 확인
    // TODO: guild_set_nickname efun이 필요함
    // 현재는 대상의 별호를 설정할 수 있는 efun이 없음

    let target_display = if my_name == target { "자신" } else { target };

    send_line(ob, "당신이 " + target_display + "의 방파별호를 『" + nickname + "』로 함을 선포합니다.");
    send_line(ob, "☞ (현재 별호 설정 기능은 추가 efun이 필요합니다)");

    // 방파 전체에게 전송
    let msg = my_name + " 님이 " + target_display + "의 방파별호를 『" + nickname + "』로 함을 선포합니다.";
    send_broadcast_to_guild(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
