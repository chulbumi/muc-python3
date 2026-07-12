// Python cmds/무공.py의 출력/대상 선택을 그대로 구성한다.

fn learned_skill_level(entries, skill_name) {
    for entry in entries {
        if entry["name"] == skill_name {
            return entry["level"];
        }
    }
    return -1;
}

fn build_skill_message(view) {
    let categories = [
        "초급", "중급", "상급", "고급", "특급",
        "절정", "초절정", "회복", "방어", "기타"
    ];
    let msg = "";
    let first_category = true;

    for category in categories {
        let configured = get_murim_config_list(category + "무공");
        if len(configured) == 0 {
            continue;
        }
        if !first_category {
            msg += "\r\n";
        }
        first_category = false;
        msg += "\x1b[1m\x1b[40m\x1b[32m▷ " + category
            + "무공\x1b[0m\x1b[40m\x1b[37m";

        let count = 0;
        for configured_name in configured {
            let skill_name = trim(configured_name.to_string());
            if skill_name == "" {
                continue;
            }
            let level = learned_skill_level(view["skills"], skill_name);
            if level < 0 {
                continue;
            }
            if count == 0 || count % 3 == 0 {
                msg += "\r\n";
            }
            let cell = skill_name + "(" + level.to_string() + "성)";
            msg += " ◇ " + fill_space_euc_kr(20, cell) + " ";
            count += 1;
        }
    }
    msg
}

fn build_secret_message(view) {
    let msg = "";
    let training = view["secret_training"];
    if training != "" {
        msg = "\x1b[1m\x1b[33m" + training
            + "\x1b[0m\x1b[40m\x1b[37m(수련중)";
    }

    let count = 0;
    for secret_name in view["secret_names"] {
        if count == 0 {
            if msg != "" {
                msg += "\r\n";
            }
        } else if count % 3 == 0 {
            msg += "\r\n";
        }
        msg += " ◇ " + fill_space_euc_kr(20, secret_name.to_string()) + " ";
        count += 1;
    }
    msg
}

fn cmd(ob, line) {
    let view = get_mugong_view(ob, line);
    if !view["ok"] {
        send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네");
        return;
    }

    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let title = if view["self"] {
        "◁ 당신의 무공 ▷"
    } else {
        "◁ " + view["name"] + "의 무공 ▷"
    };
    send_line(ob, "\x1b[0m\x1b[47m\x1b[30m" + fill_space(71, title)
        + "\x1b[0m\x1b[40m\x1b[37m");
    send_line(ob, "───────────────────────────────────────");

    if !view["has_skill_list"] {
        send_line(ob, "☞ 깨우친 무공이 없습니다.");
    } else {
        send_line(ob, build_skill_message(view));
    }
    send_line(ob, "───────────────────────────────────────");

    send_line(ob, "\x1b[1m\x1b[40m\x1b[32m▷ 비전\x1b[0m\x1b[40m\x1b[37m");
    if view["secret_training"] == "" && len(view["secret_names"]) == 0 {
        send_line(ob, "☞ 오의를 깨우친 무공이 없습니다.");
    } else {
        send_line(ob, build_secret_message(view));
    }
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
