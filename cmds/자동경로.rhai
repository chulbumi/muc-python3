// Python cmds/자동경로.py - Player.autoMoveList를 네트워크 경계에 요청

fn cmd(ob, line) {
    let route = trim(line);
    if route == "" {
        if auto_move_count(ob) == 0 {
            send_line(ob, "☞ 사용법: [지정한줄임말] 자동경로");
        } else {
            request_auto_move_route(ob, "");
            send_line(ob, "☞ 자동경로가 삭제되었습니다.");
        }
        return;
    }
    if !alias_has(ob, route) {
        send_line(ob, "☞ 해당 줄임말이 없어요. ^^");
        return;
    }
    request_auto_move_route(ob, alias_get(ob, route));
    send_line(ob, "☞ 자동경로를 설정하였어요. ^^");
}

fn main(ob, line) {
    cmd(ob, line);
}
