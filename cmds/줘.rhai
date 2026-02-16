// 줘.rhai - 아이템 주기 (Python: cmds/줘.py)
// 사용법: [대상] [물품] 주다, [대상] 은전 [수량] 주다

fn cmd(ob, line) {
    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [물품] 주다");
        return;
    }

    let target_name = words[0];
    let item_name = words[1];

    // Check if target is valid
    let r = find_target(ob, target_name);
    let found = r["found"];
    if !found {
        send_line(ob, "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^");
        return;
    }
    let to_target = r["to_target"];
    let found_name = to_target["name"].to_string();
    if found_name == "" {
        send_line(ob, "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^");
        return;
    }

    // Check if giving to self
    let my_name = get_string(ob, "이름");
    if found_name == my_name {
        send_line(ob, "☞ 자신에게 줄 수는 없어요. ^^");
        return;
    }

    let count = if len(words) >= 3 { to_int(words[2]) } else { 1 };
    let count = if count < 1 { 1 } else { if count > 50 { 50 } else { count } };

    // 은전 주기
    if item_name == "은전" {
        let have = get_int(ob, "은전");
        if have == 0 {
            send_line(ob, "☞ 돈이 모자라네요. ^^");
            return;
        }
        let give = if count > have { have } else { count };
        let err = request_give_silver(ob, found_name, give);
        if err == "no_money" {
            send_line(ob, "☞ 돈이 모자라네요. ^^");
            return;
        }
        // Success message
        send_line(ob, "당신이 " + found_name + "에게 은전 " + give + "개를 줍니다.");
        send_to_player(found_name, "\r\n" + my_name + han_iga(my_name) + " 당신에게 은전 " + give + "개를 줍니다.");
        return;
    }

    // 금전 주기
    if item_name == "금전" {
        let have = get_int(ob, "금전");
        if have == 0 {
            send_line(ob, "☞ 돈이 모자라네요. ^^");
            return;
        }
        let give = if count > have { have } else { count };
        let err = request_give_gold(ob, found_name, give);
        if err == "no_money" {
            send_line(ob, "☞ 돈이 모자라네요. ^^");
            return;
        }
        send_line(ob, "당신이 " + found_name + "에게 금전 " + give + "개를 줍니다.");
        send_to_player(found_name, "\r\n" + my_name + han_iga(my_name) + " 당신에게 금전 " + give + "개를 줍니다.");
        return;
    }

    // 아이템 주기
    // parse_order_name returns [order, name]
    let parsed = parse_order_name(item_name);
    let order = to_int(parsed[0]);
    let name = parsed[1].to_string();
    let give_count = if order != 1 { 1 } else { count };

    let err = request_give_item(ob, found_name, name, order, give_count);
    if err == "no_item" {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }

    // Success
    let item_post = name + han_eul(name);
    if give_count == 1 {
        send_line(ob, "당신이 " + found_name + "에게 " + item_post + " 줍니다.");
        send_to_player(found_name, "\r\n" + my_name + han_iga(my_name) + " 당신에게 " + item_post + " 줍니다.");
    } else {
        send_line(ob, "당신이 " + found_name + "에게 " + name + " " + give_count + "개를 줍니다.");
        send_to_player(found_name, "\r\n" + my_name + han_iga(my_name) + " 당신에게 " + name + " " + give_count + "개를 줍니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
