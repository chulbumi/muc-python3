// 봐.rhai - 보기/조사 명령어
// arg 없음: view_map_data(ob) efun — 방 이름·설명·출구·몹·바닥 아이템
// arg 있음: find_target(ob, line) → lines send_line, to_target이면 send_to_player (당분간 no-op)

fn main(ob, line) {
    if line.trim() == "" {
        let r = view_map_data(ob);
        if r["ok"] { send_line(ob, r["text"]); }
        else {
            if r["err"] == "no_position" { send_line(ob, "\x1b[1;31m위치 정보가 없습니다.\x1b[0;37m"); }
            else if r["err"] == "room_error" { send_line(ob, "\x1b[1;31m방 정보를 읽을 수 없습니다.\x1b[0;37m"); }
            else if r["err"] == "unknown_room" { send_line(ob, "\x1b[1;37m[" + r["zone"] + ":" + int_to_str(r["room"]) + "]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n"); }
        }
        return;
    }
    let r = find_target(ob, line);
    if r["err"] == "no_position" { send_line(ob, "위치 정보가 없습니다."); return; }
    if r["err"] == "not_found" { send_line(ob, "\x1b[1;31m☞ 당신의 안광으로는 그런것을 볼수 없다네\x1b[0;37m"); return; }
    for l in r["lines"] {
        send_line(ob, l);
    }
    let tt = r["to_target"];
    if tt["name"] != "" {
        send_to_player(tt["name"], tt["msg"]);
    }
}
