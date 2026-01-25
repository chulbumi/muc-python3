// 방파리스트.rhai - guild_list() 순회, guild_get(id,"이름") 출력.

fn cmd(ob, line) {
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    send_line(ob, "☆ 방파 리스트");
    send_line(ob, "───────────────────────────────────────");
    let ids = guild_list();
    for i in 0..len(ids) {
        let gid = ids[i];
        let n = guild_get(gid, "이름");
        let buf = if length(n) > 0 { "[" + n + "]" } else { "[" + gid + "]" };
        send_line(ob, "  " + buf);
    }
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
