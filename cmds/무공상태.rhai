// Python cmds/무공상태.py - 활성 방어무공 상태

fn pad_num(value, width) {
    let text = value.to_string();
    let result = "";
    let count = width - len(text);
    while count > 0 {
        result += " ";
        count -= 1;
    }
    result + text
}

fn cmd(ob, line) {
    let target_view = get_mugong_view(ob, line);
    if !target_view["ok"] {
        send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네");
        return;
    }
    let target_name = target_view["name"].to_string();
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    send_line(ob, " \x1b[1m\x1b[32mЖ \x1b[37m" + target_name
        + "\x1b[0;37m의 무공집결 상태 \x1b[1m\x1b[32mЖ\x1b[0m\x1b[37m");

    let active = get_target_active_mugong_status(ob, line);
    if len(active) == 0 {
        send_line(ob, "──────────────────────────────");
        send_line(ob, get_murim_config("무공시전없음").to_string());
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        return;
    }
    send_line(ob, "────────┬──────┬──────────────");
    let bars = [
        "\x1b[37m━━━━━━━━━━\x1b[37m", "\x1b[31m━\x1b[37m━━━━━━━━━\x1b[37m",
        "\x1b[31m━━\x1b[37m━━━━━━━━\x1b[37m", "\x1b[31m━━━\x1b[37m━━━━━━━\x1b[37m",
        "\x1b[33m━━━━\x1b[37m━━━━━━\x1b[37m", "\x1b[33m━━━━━\x1b[37m━━━━━\x1b[37m",
        "\x1b[33m━━━━━━\x1b[37m━━━━\x1b[37m", "\x1b[32m━━━━━━━\x1b[37m━━━\x1b[37m",
        "\x1b[32m━━━━━━━━\x1b[37m━━\x1b[37m", "\x1b[32m━━━━━━━━━\x1b[37m━\x1b[37m",
        "\x1b[32m━━━━━━━━━━\x1b[37m"
    ];
    for active_skill in active {
        let level = active_skill["level"];
        let duration = active_skill["defense_time"]
            + active_skill["defense_time_increase"] * (level - 1);
        if duration <= 0 { duration = 1; }
        let index = (active_skill["time"] * 10) / duration;
        if index < 0 { index = 0; }
        if index >= len(bars) { index = len(bars) - 1; }
        let skill_data = get_skill_data(active_skill["name"]);
        let display = get_string(skill_data, "방어상태출력");
        let bar = pad_num(active_skill["time"], 5) + "ː" + bars[index];
        send_line(ob, "\x1b[1m\x1b[40m\x1b[36m·\x1b[0m\x1b[40m\x1b[37m"
            + fill_space(14, active_skill["name"])
            + "│" + fill_space(12, display) + "│ " + bar);
    }
    send_line(ob, "━━━━━━━━┷━━━━━━┷━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
