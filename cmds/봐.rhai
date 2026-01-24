// 봐.rhai - 보기/조사 명령어
// arg 없음: view_map_data(ob) efun — 방 이름·설명·출구·몹·바닥 아이템
// arg 있음: find_target(ob, line) → lines send_line, to_target이면 send_to_player (당분간 no-op)

fn main(ob, line) {
    if line.trim() == "" {
        view_map_data(ob);
        return;
    }
    let r = find_target(ob, line);
    for l in r["lines"] {
        send_line(ob, l);
    }
    let tt = r["to_target"];
    if tt["name"] != "" {
        send_to_player(tt["name"], tt["msg"]);
    }
}
