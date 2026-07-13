// 정리.rhai - 플레이어 강제 종료 (관리자)
// Python: cmds/정리.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [사용자명] 정리");
        return;
    }
    send_line(ob, line);
    let found = false;
    for player_name in get_connected_player_names() {
        if player_name == line { found = true; break; }
    }
    if found && disconnect_player_for_cleanup(ob, line) {
        send_line(ob, "☞ 정리하였습니다. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
