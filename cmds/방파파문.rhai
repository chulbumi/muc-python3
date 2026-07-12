// 방파파문.rhai - 방파 파문 (방주가 멤버를 추방)
// Python: cmds/방파파문.py
// guild_kick_member updates the guild roster and queues an online attribute
// update; it does not disconnect the target player.

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

    let result = guild_kick_member(ob, line);
    if result == "wrong_guild" { send_line(ob, "☞ 당신의 소속이 아닙니다."); return; }
    if result != "ok" { send_line(ob, "☞ 그런 무림인은 아애 존재하지 않습니다."); return; }
    let msg = han_iga(get_attr(ob, "이름")) + " " + han_obj(line) + " 방파에서 파문시킴을 선포합니다.";
    send_to_player(line, "\r\n당신은 파문되었습니다.");
    send_broadcast_to_guild(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
