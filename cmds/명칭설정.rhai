// 명칭설정.rhai - 명칭 설정 (방파 등)
// Python: cmds/명칭설정.py

fn cmd(ob, line) {
    if get_attr(ob, "직위") != "방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }
    let words = line_args(line);
    if len(words) < 2 || (words[0] != "방주" && words[0] != "부방주" && words[0] != "장로" && words[0] != "방파인") {
        send_line(ob, "☞ 사용법 : [방주|부방주|장로|방파인] [이름] 명칭설정");
        return;
    }
    let guild = get_attr(ob, "소속");
    if guild == "" {
        send_line(ob, "☞ 방파에 소속되어 있지 않습니다.");
        return;
    }
    guild_set(guild, words[0] + "명칭", words[1]);
    let msg = han_iga(get_attr(ob, "이름")) + " " + words[0] + "의 명칭을 \x1b[1m" + words[1] + "\x1b[0;37m" + han_uro(words[1]) + " 변경하여 선포합니다.";
    send_broadcast_to_guild(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
