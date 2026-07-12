// Python cmds/방파상태.py - 방파 상태

fn split_members(raw) {
    let result = [];
    for value in split(raw, "|") {
        if trim(value) != "" { result.push(trim(value)); }
    }
    if len(result) == 0 && raw != "" { result.push(raw); }
    result
}

fn cmd(ob, line) {
    let guild_name = get_string(ob, "소속");
    if guild_name == "" {
        send_line(ob, "☞ 당신은 소속이 없습니다.");
        return;
    }
    let info = guild_get_info(ob);
    if info.is_unit() {
        send_line(ob, "☞ 당신은 소속이 없습니다.");
        return;
    }
    let msg = get_murim_config("방파상태출력상단").to_string();
    msg += get_murim_config("방파상태출력").to_string();
    msg += "\x1b[1;31m\x1b[1;47m" + guild_name + "\x1b[0;37;40m\r\n";
    msg += get_murim_config("방파상태출력하단").to_string();
    msg += "\r\n  [\x1b[1m\x1b[31m방  주\x1b[0m\x1b[40m\x1b[37m]     "
        + fill_space(11, info["방주"].to_string());
    let count = 1;
    for member in split_members(info["부방주"].to_string()) {
        msg += "  [\x1b[1m\x1b[33m부방주\x1b[0m\x1b[40m\x1b[37m]     " + fill_space(11, member);
    }
    for member in split_members(info["장로"].to_string()) {
        msg += "  [\x1b[1m\x1b[32m장  로\x1b[0m\x1b[40m\x1b[37m]     " + fill_space(11, member);
    }
    for member in split_members(info["제자"].to_string()) {
        msg += "  [\x1b[1m방파원\x1b[0m\x1b[40m\x1b[37m]     " + fill_space(11, member);
    }
    msg += "\r\n\r\n" + get_murim_config("방파상태출력하단").to_string();
    msg += "\r\n방파총인원 : " + fill_space(8, info["멤버수"].to_string());
    let active = 0;
    for player in get_all_online_players() {
        if player["소속"] == guild_name && player["투명상태"] != 1 { active += 1; }
    }
    msg += "☞ 현재 " + int_to_str(active) + "명이 활동중 입니다.";
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
