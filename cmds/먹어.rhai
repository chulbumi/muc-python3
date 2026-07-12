// Python `cmds/먹어.py`. Rust mutates inventory/vitals; Rhai owns output.

fn cmd(ob, line) {
    if line == "" { send_line(ob, "☞ 사용법: [아이템 이름] 먹어"); return; }
    let parsed = parse_order_name(line);
    let result = item_use_consumable(ob, parsed[1], parsed[0]);
    let error = result["err"];
    if error == "bad_state" { send_line(ob, "☞ 먹을 수 있는 상황이 아니네요. ^_^"); return; }
    if error == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if error == "not_consumable" { send_line(ob, "☞ 먹을 수 있는것이 아니에요. ^_^"); return; }
    if error != "" { return; }

    let item = if result["ansi"] != "" {
        result["ansi"] + result["name"] + "\x1b[0;37m"
    } else { "\x1b[0;36m" + result["name"] + "\x1b[37m" };
    let script = join(split(result["script"], "\n"), "\r\n");
    let message = return_replace(script, "$아이템$", item);
    send_line(ob, "당신이 " + message);
    let own_name = get_string(ob, "이름");
    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    let room_message = actor + " " + message;

    let gain = result["max_mp_gain"];
    if gain > 0 {
        send_line(ob, "\r\n\x1b[1m당신의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돕니다.\x1b[0;37m (\x1b[1;36m+" + gain.to_string() + "\x1b[0;37m)");
        room_message += "\r\n\r\n\x1b[1m\x1b[1m" + own_name + "\x1b[0;37m의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돕니다.\x1b[0;37m (\x1b[1;36m+" + gain.to_string() + "\x1b[0;37m)";
    } else if gain < 0 {
        send_line(ob, "\r\n\x1b[1m당신의 단전에 심한 요동이 일어나며 고통이 몰려오기 시작합니다.\x1b[0;37m (\x1b[1;31m" + gain.to_string() + "\x1b[0;37m)");
        room_message += "\r\n\r\n\x1b[1m\x1b[1m" + own_name + "\x1b[0;37m의 단전에 심한 요동이 일어나며 고통이 몰려오기 시작합니다.\x1b[0;37m (\x1b[1;31m" + gain.to_string() + "\x1b[0;37m)";
    }
    for player in get_cast_room_players(ob) {
        if player["name"] != own_name && !player["reject_fight"] { send_to_user(player["name"], room_message); }
    }

    if alias_has(ob, "내공약") && alias_get(ob, "내공약") == result["name"]
        && alias_has(ob, "내공약하한") && result["remaining"] <= to_int(alias_get(ob, "내공약하한")) {
        send_line(ob, "☞ 알림 : 내공약 개수 부족");
    }
    if alias_has(ob, "체력약") && alias_get(ob, "체력약") == result["name"]
        && alias_has(ob, "체력약하한") && result["remaining"] <= to_int(alias_get(ob, "체력약하한")) {
        send_line(ob, "☞ 알림 : 체력약 개수 부족");
    }
    if alias_has(ob, "체력약") && alias_has(ob, "체력") && alias_has(ob, "연속복용")
        && alias_get(ob, "연속복용") == "켜기"
        && result["hp"] != 0 && get_int(ob, "체력") < to_int(alias_get(ob, "체력")) {
        queue_self_command(ob, line + " 먹어");
    }
}
fn main(ob, line) { cmd(ob, line); }
