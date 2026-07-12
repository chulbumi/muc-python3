// 방파리스트.rhai - 방파 리스트 출력
// Python: cmds/방파리스트.py

fn cmd(ob, line) {
    let msg = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n"
        + "☆ 방파 리스트\r\n"
        + "───────────────────────────────────────\r\n";
    let ids = guild_list();
    let is_admin = get_int(ob, "관리자등급") >= 1000;
    for i in 0..len(ids) {
        let gid = ids[i];
        let n = guild_get(gid, "이름");
        let leader = guild_get(gid, "방주이름");
        if leader == "" { leader = guild_get(gid, "방주리스트"); }
        let guild_name = if length(n) > 0 { n } else { gid };
        let buf = "[" + guild_name + "]";
        let buf_pad = buf + repeat(" ", if length(buf) < 12 { 12 - length(buf) } else { 0 });
        let leader_pad = leader + repeat(" ", if length(leader) < 30 { 30 - length(leader) } else { 0 });
        let count = guild_get(gid, "방파원수");
        let count_pad = repeat(" ", if length(count) < 3 { 3 - length(count) } else { 0 }) + count;
        if is_admin {
            let map = guild_get(gid, "방파맵");
            msg += buf_pad + " : " + leader_pad + "   " + count_pad + " 명 " + map + "\r\n";
        } else {
            msg += buf_pad + " : " + leader_pad + "   " + count_pad + " 명\r\n";
        }
    }
    msg += "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
