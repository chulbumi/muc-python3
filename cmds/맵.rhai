// 맵.rhai - 맵 명령어 (관리자 2000)
// Python cmds/맵.py의 DFS 탐색은 map_explore_directions efun이 수행한다.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    let pos = get_my_position(ob);
    if length(pos["zone"]) == 0 {
        send_line(ob, "\r\n* 아무것도 보이지 않습니다.\r\n");
        return;
    }
    let exits = get_exits_string(pos["zone"], pos["room"]);
    if length(exits) == 0 || (len(exits) >= 10 && substring(exits, 0, 10) == "  ○  어느 쪽으로도") {
        send_line(ob, "☞ 아무것도 보이지 않습니다.");
        return;
    }
    let requested = trim(line);
    if requested == "" {
        send_line(ob, "☞ 사용법: [제외할방향] 맵");
        return;
    }
    // Python checks the raw `env.exitList` entry, not a substring of the
    // rendered compass (so `동` must not accidentally match `북동`).
    if !room_has_exit_named(pos["zone"], pos["room"], requested) {
        send_line(ob, "☞ 그 방향으로는 갈수가 없어요!.");
        return;
    }
    let explored = map_explore_directions(ob, requested);
    if len(explored) == 0 { send_line(ob, "☞ 그 방향으로는 갈수가 없어요!."); return; }
    send_line(ob, join(explored, ";") + ";");
}

fn main(ob, line) {
    cmd(ob, line);
}
