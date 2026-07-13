// 판매.rhai - 판매 명령어 (Python: cmds/판매.py)
// 상인(물건구입)이 있는 방에서 [이름] [수량] 판매

fn sell_prompt(player) {
    if !player["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + player["hp"].to_string() + "/"
        + player["max_hp"].to_string() + ", " + player["mp"].to_string()
        + "/" + player["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    let normalized = trim(line);
    if normalized == "" { send_line(ob, "☞ 사용법: [물품이름] [수량] 판매"); return; }
    let args = line_args(normalized);
    let parsed = parse_order_name(args[0]);
    let order = parsed[0];
    let name = parsed[1];
    let cnt = if len(args) >= 2 { parse_int_prefix(args[1]) } else { 1 };
    let cnt = if cnt < 1 { 1 } else { if cnt > 100 { 100 } else { cnt } };

    let percent = get_merchant_buy_percent(ob);
    if percent <= 0 {
        send_line(ob, "☞ 그런 물건을 살 상인이 없어요. ^_^");
        return;
    }

    let batch_mode = normalized;
    if batch_mode == "속성0" || batch_mode == "속성1" || batch_mode == "속성2"
            || batch_mode == "속성3" || batch_mode == "일반"
            || batch_mode == "장비" || batch_mode == "모두" {
        let sold = item_sell_all(ob, batch_mode, percent);
        if sold.len() == 0 {
            send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
            return;
        }
        let own_name = get_string(ob, "이름");
        let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
        for item in sold {
            let item_name = item["post"];
            let text = item_name + " 1개를 은전 " + item["price"].to_string() + "개에 판매합니다.";
            send_line(ob, "당신이 " + text);
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name && !player["reject_fight"] {
                    send_raw_to_user(player["name"], "\r\n" + actor + " " + text
                        + "\r\n" + sell_prompt(player));
                }
            }
        }
        return;
    }

    let ret = item_sell(ob, name, order, cnt, percent);
    if ret[3] == "cant_sell" {
        send_line(ob, "☞ 그 아이템은 팔 수가 없어요~");
        return;
    }
    if ret[0] == 0 {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }
    let item_name = ret[4];
    send_line(ob, "당신이 " + item_name + " " + ret[0] + "개를 은전 " + ret[1] + "개에 판매합니다.");
    // Python의 단일 판매 방 문구는 여러 개를 팔아도 `1개`로 고정되는 원본 동작이다.
    let own_name = get_string(ob, "이름");
    let room_text = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name)
        + " " + item_name + " 1개를 은전 " + ret[1] + "개에 판매합니다.";
    for player in get_cast_room_players(ob) {
        if player["name"] != own_name && !player["reject_fight"] {
            send_raw_to_user(player["name"], "\r\n" + room_text + "\r\n" + sell_prompt(player));
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
