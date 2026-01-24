// 뒤져.rhai - 뒤져. Python: cmds/뒤져.py
// [대상] ob.env.findObjName(line) → 몹/사람/시체. 시체/죽은몹 objs에서 아이템 옮김. find_target(플레이어만), room objs 검색 efun 없음.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 뒤져");
        return;
    }
    let t = find_target(ob, line);
    if t["found"] == true {
        let to_t = t["to_target"];
        let n = to_t["name"];
        if n != "" {
            send_line(ob, "당신이 " + n + han_eul(n) + " 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
            send_room(ob, get_string(ob, "이름") + han_iga(get_string(ob, "이름")) + " " + n + han_eul(n) + " 몸을 더듬습니다. \"뭐 없나~~ -.-\"");
            return;
        }
    }
    send_line(ob, "☞ 뒤질대상이 없어요. ^^");
}

fn main(ob, line) {
    cmd(ob, line);
}
