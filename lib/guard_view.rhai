// Shared Python 호위.py presentation used by both `호위` and `호위 봐`.

fn guard_view_bar(percent) {
    let bars = [
        "\x1b[37m━━━━━━━━━━\x1b[37m", "\x1b[31m━\x1b[37m━━━━━━━━━\x1b[37m",
        "\x1b[31m━━\x1b[37m━━━━━━━━\x1b[37m", "\x1b[31m━━━\x1b[37m━━━━━━━\x1b[37m",
        "\x1b[33m━━━━\x1b[37m━━━━━━\x1b[37m", "\x1b[33m━━━━━\x1b[37m━━━━━\x1b[37m",
        "\x1b[33m━━━━━━\x1b[37m━━━━\x1b[37m", "\x1b[32m━━━━━━━\x1b[37m━━━\x1b[37m",
        "\x1b[32m━━━━━━━━\x1b[37m━━\x1b[37m", "\x1b[32m━━━━━━━━━\x1b[37m━\x1b[37m",
        "\x1b[32m━━━━━━━━━━\x1b[37m"
    ];
    bars[min(10, max(0, percent / 10))]
}

fn guard_view_render(name, anger, guards) {
    let last = guards[len(guards) - 1];
    let message = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n";
    let title = "☞ " + name + "의 호위 : " + last["name"] + ", 호위수 : "
        + len(guards).to_string() + ", 분노 : " + anger.to_string();
    message += "\x1b[1;44m" + fill_space(56, title) + "\x1b[0;40m\r\n";
    message += "────────────────────────────\r\n";
    message += join(split(last["description"], "\n"), "\r\n") + "\r\n";
    message += "────────────────────────────\r\n";
    let i = 0;
    while i < len(guards) {
        let guard = guards[i];
        let hp = if guard["max_hp"] > 0 { guard["hp"] * 100 / guard["max_hp"] } else { 0 };
        message += "\x1b[1;32m·\x1b[0;36m" + pad_start((i + 1).to_string(), 2, " ") + "."
            + guard["name"] + "\x1b[0;37m ː " + guard_view_bar(hp) + " (" + hp.to_string() + ")\r\n";
        i += 1;
    }
    message + "━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

fn guard_view_command(ob, line) {
    if line == "" {
        let guards = get_inventory_guards(ob);
        if len(guards) == 0 { send_line(ob, "당신은 호위를 거느리지 않고 있습니다."); return; }
        send_line(ob, guard_view_render("당신", get_int(ob, "분노"), guards));
        return;
    }
    let target_name = resolve_room_player_name(ob, line);
    if target_name == "" {
        send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네");
        return;
    }
    if target_name == get_string(ob, "이름") {
        let guards = get_inventory_guards(ob);
        if len(guards) == 0 { send_line(ob, "당신은 호위를 거느리지 않고 있습니다."); return; }
        send_line(ob, guard_view_render(target_name, get_int(ob, "분노"), guards));
        return;
    }
    for player in get_room_players_admin(ob) {
        if player["name"] != target_name { continue; }
        let guards = player["guards"];
        if len(guards) == 0 {
            send_line(ob, "\x1b[1m" + target_name + "\x1b[0;37m" + han_eun(target_name)
                + " 호위를 거느리지 않고 있습니다.");
            return;
        }
        send_line(ob, guard_view_render(target_name, player["anger"], guards));
        return;
    }
    send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네");
}
