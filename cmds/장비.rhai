// 장비.rhai - 장비 명령어. Python: cmds/장비.py
// get_equipped(ob), get_armor(ob), get_att_power(ob), get_item_level_display(slot), fill_space

fn cmd(ob, line) {
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let eq = get_equipped(ob);
    let arm_msg = if len(eq) == 0 { "▷ 당신은 맨몸입니다" } else { "▷ 당신은 장비를 착용하고 있습니다" };
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m" + fill_space(27, arm_msg) + "\x1b[0m\x1b[37m\x1b[40m");
    send_line(ob, "───────────────────────────");
    let mut item_str = "";
    for e in eq {
        let slot = e["slot"];
        let disp = get_item_level_display(slot);
        let n = e["name"];
        item_str = item_str + "[" + disp + "] \x1b[36m" + n + "\x1b[37m\r\n";
    }
    if item_str != "" {
        send_line(ob, item_str);
    } else {
        send_line(ob, "\x1b[36m☞ 혈혈단신 맨몸으로 강호를 주유중입니다.\x1b[37m");
    }
    send_line(ob, "───────────────────────────");
    let arm = get_armor(ob);
    let att = get_att_power(ob);
    send_line(ob, "【방어력】▷ " + int_to_str(arm) + "    【공격력】▷ " + int_to_str(att));
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    cmd(ob, line);
}
