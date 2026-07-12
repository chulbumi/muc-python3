// Python `cmds/줘.py`: 같은 방 플레이어에게 화폐나 소지품을 전달한다.

fn cmd(ob, line) {
    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [물품] 주다");
        return;
    }
    let requested_target = words[0];
    let item_arg = words[1];
    let count = if len(words) >= 3 { to_int(words[2]) } else { 1 };
    let count = if count < 1 { 1 } else { if count > 50 { 50 } else { count } };
    let my_name = get_string(ob, "이름");

    if item_arg == "은전" || item_arg == "금전" {
        let target_name = find_give_player(ob, requested_target);
        if target_name == "" {
            send_line(ob, "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^");
            return;
        }
        let have = get_int(ob, item_arg);
        if have == 0 {
            send_line(ob, "☞ 돈이 모자라네요. ^^");
            return;
        }
        let amount = if count > have { have } else { count };
        if target_name == my_name {
            send_line(ob, "당신이 \x1b[1m" + my_name + "\x1b[0m에게 " + item_arg + " " + amount.to_string() + "개를 줍니다.");
            send_line(ob, "\r\n\x1b[1m" + my_name + "\x1b[0;37m" + han_iga(my_name)
                + " 당신에게 " + item_arg + " " + amount.to_string() + "개를 줍니다.");
            return;
        }
        if item_arg == "은전" { request_give_silver(ob, target_name, amount); }
        else { request_give_gold(ob, target_name, amount); }
        return;
    }

    let parsed = parse_order_name(item_arg);
    let order = parsed[0].to_int();
    let selected = give_item_presentation(ob, parsed[1], order);
    if !selected["found"] {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }
    let target_name = find_give_player(ob, requested_target);
    if target_name == "" {
        send_line(ob, "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^");
        return;
    }
    if target_name == my_name {
        send_line(ob, "당신이 \x1b[36m" + selected["post"].to_string()
            + "\x1b[37m 가지고 장난합니다. '@_@'");
        return;
    }
    let give_count = if order != 1 { 1 } else { count };
    request_give_item(ob, target_name, selected["name"].to_string(), order, give_count);
}

fn main(ob, line) { cmd(ob, line); }
