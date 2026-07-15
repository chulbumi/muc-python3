// Rust/Rhai extension: consumable use alias. Item events are dispatched
// before command lookup, so `[아이템] 사용` events retain priority.

fn use_prompt(player) {
    if !player["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + player["hp"].to_string() + "/"
        + player["max_hp"].to_string() + ", " + player["mp"].to_string()
        + "/" + player["max_mp"].to_string() + " ] "
}

fn permanent_effect_text(result) {
    let parts = [];
    for effect in result["permanent_effects"] {
        let amount = effect["amount"];
        let sign = if amount > 0 { "+" } else { "" };
        parts.push(effect["name"] + " " + sign + amount.to_string());
    }
    join(parts, ", ")
}

fn cmd(ob, line) {
    if line == "" { send_line(ob, "☞ 사용법: [아이템 이름] 사용"); return; }
    let parsed = parse_order_name(line);
    let result = item_use_consumable(ob, parsed[1], parsed[0]);
    let error = result["err"];
    if error == "bad_state" { send_line(ob, "☞ 사용할 수 있는 상황이 아니네요. ^_^"); return; }
    if error == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if error == "not_consumable" { send_line(ob, "☞ 사용할 수 있는 물건이 아니에요. ^_^"); return; }
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
    let buff_script = result["buff_script"];
    if buff_script != "" {
        let buff_message = return_replace(
            return_replace(
                return_replace(buff_script, "$아이템$", item),
                "$효과$", result["buff_name"].to_string()
            ),
            "$지속시간$", result["buff_duration"].to_string()
        );
        send_line(ob, join(split(buff_message, "\n"), "\r\n"));
        room_message += "\r\n" + actor + " " + join(split(buff_message, "\n"), "\r\n");
    }
    let permanent_text = permanent_effect_text(result);
    if permanent_text != "" {
        let permanent_script = result["permanent_script"];
        if permanent_script == "" {
            send_line(ob, "☞ " + permanent_text + " 영구 증가");
        } else {
            let permanent_message = return_replace(
                return_replace(permanent_script, "$아이템$", item),
                "$영구효과$", permanent_text
            );
            send_line(ob, join(split(permanent_message, "\n"), "\r\n"));
            room_message += "\r\n" + actor + " "
                + join(split(permanent_message, "\n"), "\r\n");
        }
    }

    let gain = result["max_mp_gain"];
    if gain > 0 {
        send_line(ob, "\r\n\x1b[1m당신의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돕니다.\x1b[0;37m (\x1b[1;36m+" + gain.to_string() + "\x1b[0;37m)");
        room_message += "\r\n\r\n\x1b[1m\x1b[1m" + own_name + "\x1b[0;37m의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돕니다.\x1b[0;37m (\x1b[1;36m+" + gain.to_string() + "\x1b[0;37m)";
    } else if gain < 0 {
        send_line(ob, "\r\n\x1b[1m당신의 단전에 심한 요동이 일어나며 고통이 몰려오기 시작합니다.\x1b[0;37m (\x1b[1;31m" + gain.to_string() + "\x1b[0;37m)");
        room_message += "\r\n\r\n\x1b[1m\x1b[1m" + own_name + "\x1b[0;37m의 단전에 심한 요동이 일어나며 고통이 몰려오기 시작합니다.\x1b[0;37m (\x1b[1;31m" + gain.to_string() + "\x1b[0;37m)";
    }
    for player in get_cast_room_players(ob) {
        if player["name"] != own_name && !player["reject_fight"] {
            send_raw_to_user(player["name"], "\r\n" + room_message + "\r\n"
                + use_prompt(player));
        }
    }
}

fn main(ob, line) { cmd(ob, line); }
