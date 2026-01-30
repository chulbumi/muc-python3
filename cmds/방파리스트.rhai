// 방파리스트.rhai - 방파 리스트 출력
// Python: cmds/방파리스트.py

fn cmd(ob, line) {
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    send_line(ob, "☆ 방파 리스트");
    send_line(ob, "───────────────────────────────────────");

    let ids = guild_list();
    let admin_level = get_attr(ob, "관리자등급");
    let is_admin = to_int(admin_level) >= 1000;

    for i in 0..len(ids) {
        let gid = ids[i];
        let n = guild_get(gid, "이름");
        let leader = guild_get(gid, "방주");

        let guild_name = if length(n) > 0 { n } else { gid };
        let buf = "[" + guild_name + "]";

        if is_admin {
            let map = guild_get(gid, "방파맵");
            let map_info = if length(map) > 0 { " " + map } else { "" };
            send_line(ob, "  " + buf + " : " + leader + map_info);
        } else {
            send_line(ob, "  " + buf + " : " + leader);
        }
    }

    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
