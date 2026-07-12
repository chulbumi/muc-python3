// 현판걸어.rhai - 현판 걸기 (방파 생성)
// Python: cmds/현판걸어.py
// Uses: guild_create(ob, guild_name)

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법 : [방파이름] 현판걸어");
        return;
    }

    let guild_name = trim(line);

    if length(guild_name) > 10 {
        send_line(ob, "방파이름이 너무 길어요!.");
        return;
    }

    if length(guild_name) < 2 {
        send_line(ob, "방파이름이 너무 짧아요!.");
        return;
    }

    let my_level = get_attr(ob, "레벨");
    if to_int(my_level) < 400 {
        send_line(ob, "☞ 당신은 방파를 세울 수 없습니다.");
        return;
    }

    let my_guild = get_attr(ob, "소속");
    if my_guild != "" {
        send_line(ob, "☞ 당신은 이미 방파에 소속되어 있습니다.");
        return;
    }

    if get_room_attr(ob, "방파자리") == "" || get_room_attr(ob, "방파주인") != "" {
        send_line(ob, "☞ 이곳엔 현판을 걸 수 없습니다.");
        return;
    }
    if get_attr(ob, "방파금지") != "" || to_int(get_attr(ob, "은전")) < get_murim_config_int("방파세울은전") {
        send_line(ob, "☞ 방파를 세우는데는 은전 10,000,000개 이상이 필요합니다.");
        return;
    }
    let guilds = guild_list();
    let i = 0;
    while i < len(guilds) {
        if guild_get(guilds[i], "이름") == guild_name { send_line(ob, "☞ 존재하는 방파이름입니다."); return; }
        i = i + 1;
    }

    // guild_create efun 호출
    let result = guild_create(ob, guild_name);
    if result != "" {
        send_line(ob, "☞ " + result);
        return;
    }

    // 성공 메시지
    let my_name = get_attr(ob, "이름");
    let my_nickname = get_attr(ob, "무림별호");
    let my_personality = get_attr(ob, "성격");

    let color = if my_personality == "정파" { "1;32" } else { "1;31" };
    let nickname = if my_nickname == "" { "무소속" } else { my_nickname };

    send_line(ob, "\x1b[" + color + "m[" + nickname + "]\x1b[0m \x1b[1;36m" + my_name + "\x1b[0;37m 방파 \x1b[1m『" + guild_name + "』\x1b[0m을 창설했습니다.");

    set_obj_attr(ob, "방", "방파주인", guild_name);
    item_create(ob, "보관함");
    set_attr(ob, "은전", to_int(get_attr(ob, "은전")) - get_murim_config_int("방파세울은전"));
    send_line(ob, "당신이 현판을 세우는데 은전 " + get_murim_config_int("방파세울은전") + "개를 사용합니다.");
    send_notice(ob, guild_name + " 방파가 창설되었습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
