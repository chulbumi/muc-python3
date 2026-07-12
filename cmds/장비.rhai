// 장비.rhai - 장비 명령어. Python: cmds/장비.py

fn arm_description(arm) {
    if arm >= 0 && arm <= 10 {
        return "당신은 거의 벗다시피 하고 있습니다.";
    }
    if arm <= 31 {
        return "당신은 불쌍함이 몸에서 철철 흐르고 있습니다.";
    }
    if arm <= 57 {
        return "당신은 주위의 동정을 사기에 충분합니다.";
    }
    if arm <= 88 {
        return "당신은 너무나 형편없는 방어구를 착용하고 있습니다.";
    }
    if arm <= 119 {
        return "당신은 형편없는 방어구를 착용하고 있습니다.";
    }
    if arm <= 135 {
        return "당신은 초라한 방어구를 착용하고 있습니다.";
    }
    if arm <= 176 {
        return "당신의 방어구는 형식만 갖추고 있습니다.";
    }
    if arm <= 217 {
        return "당신은 그럭저럭 쓸만한 방어구를 착용하고 있습니다.";
    }
    if arm <= 262 {
        return "당신은 쓸만한 방어구를 착용하고 있습니다.";
    }
    if arm <= 313 {
        return "당신은 튼튼한 방어구를 착용하고 있습니다.";
    }
    if arm <= 368 {
        return "당신은 강력한 방어구를 착용하고 있습니다.";
    }
    if arm <= 429 {
        return "당신은 아주 강력한 방어구를 착용하고 있습니다.";
    }
    if arm <= 495 {
        return "당신은 어마어마한 방어구를 착용하고 있습니다.";
    }
    if arm <= 566 {
        return "당신은 환상적인 방어구를 착용하고 있습니다.";
    }
    if arm <= 642 {
        return "당신을 다른 무림인들이 똑바로 바라보지 못합니다.";
    }
    if arm <= 723 {
        return "당신의 위엄에 모든 사람이 고개를 숙입니다.";
    }
    if arm <= 804 {
        return "당신을 모든 무림인이 우러러 봅니다.";
    }
    if arm <= 885 {
        return "당신의 몸에서 은은한 광채가 뿜어져 나옵니다.";
    }
    "당신의 몸을 눈부신 광채가 찬란히 보호 합니다."
}

fn cmd(ob, line) {
    let eq = get_equipped(ob);
    let arm = get_armor(ob);
    let out = "━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n";
    let arm_msg = "▷ " + arm_description(arm);
    out += "\x1b[0m\x1b[44m\x1b[1m\x1b[37m"
        + fill_space_euc_kr(54, arm_msg) + "\x1b[0m\x1b[37m\x1b[40m\r\n";
    out += "───────────────────────────\r\n";

    let item_str = "";
    for e in eq {
        let slot = e["slot"];
        let disp = get_item_level_display(slot);
        let n = e["name"];
        let shown_name = n;
        if !e["is_han"] {
            shown_name += "(" + e["alias"] + ")";
        }
        item_str += "[" + disp + "] \x1b[36m" + shown_name + "\x1b[37m\r\n";
    }

    if item_str != "" {
        out += item_str;
    } else {
        out += "\x1b[36m☞ 혈혈단신 맨몸으로 강호를 주유중입니다.\x1b[37m\r\n";
    }

    out += "───────────────────────────\r\n";
    let att = get_att_power(ob);
    out += "【방어력】▷ " + int_to_str(arm) + "    【공격력】▷ " + int_to_str(att) + "\r\n";
    out += "━━━━━━━━━━━━━━━━━━━━━━━━━━━";
    send_line(ob, out);
}

fn main(ob, line) {
    cmd(ob, line);
}
