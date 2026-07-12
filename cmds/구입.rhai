// Python `cmds/구입.py`: 일반 물품, 자동 약품 수량, 호위 조건 구매.

fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [물품이름] [수량] 구입"); return; }
    if !room_has_merchant(ob) {
        send_line(ob, "☞ 품목을 보여줄 상인이 없어요. ^^");
        return;
    }
    let args = line_args(line);
    let name = args[0];
    let count = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    let count = if count <= 0 { 1 } else { if count > 50 { 50 } else { count } };
    if name == "체력약" && alias_has(ob, "체력약") {
        name = alias_get(ob, "체력약");
        if alias_has(ob, "체력약개수") {
            count = to_int(alias_get(ob, "체력약개수")) - inventory_named_count(ob, name);
        }
        if count <= 0 { send_line(ob, "☞ 구매할 물품이 충분합니다. ^_^"); return; }
        if count > 200 { count = 200; }
    }
    if name == "내공약" && alias_has(ob, "내공약") {
        name = alias_get(ob, "내공약");
        if alias_has(ob, "내공약개수") {
            count = to_int(alias_get(ob, "내공약개수")) - inventory_named_count(ob, name);
        }
        if count <= 0 { send_line(ob, "☞ 구매할 물품이 충분합니다. ^_^"); return; }
        if count > 200 { count = 200; }
    }
    let result = merchant_buy(ob, name, count);
    let error = result["err"];
    if error == "not_for_sale" { send_line(ob, "☞ 그런 물건은 팔지 않아요. ^_^"); return; }
    if error == "inv_full" { send_line(ob, "☞ 자네가 가질 물품의 한계라네"); return; }
    if error == "too_heavy" { send_line(ob, "☞ 무거워서 더 이상 가질 수 없어요. ^^"); return; }
    if error == "no_money" { send_line(ob, "☞ 돈이 모자라네요. ^^"); return; }
    if error == "guard_evil_only" { send_line(ob, "☞ 해당 호위는 사파원만 사용 가능합니다."); return; }
    if error == "guard_right_only" { send_line(ob, "☞ 해당 호위는 정파원만 사용 가능합니다."); return; }
    if error == "guard_level" { send_line(ob, "☞ 해당 호위를 살수가 없습니다.(레벨)"); return; }
    if error == "guard_limit" { send_line(ob, "☞ 해당 호위를 살수가 없습니다.(동종개수제한)"); return; }
    if error == "guard_unavailable" { send_line(ob, "☞ 해당 호위를 살수가 없습니다."); return; }
    if length(error) > 0 { return; }

    let actual = result["display_name"].to_string();
    let own_name = get_string(ob, "이름");
    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    let self_text = "";
    let room_text = "";
    if result["guard"] {
        let item = "\x1b[36m" + actual + han_eul(actual) + "\x1b[37m";
        self_text = "당신이 " + item + " 구입합니다.";
        room_text = actor + " " + item + " 구입합니다.";
    } else {
        let count = result["bought"].to_int();
        if count == 0 { return; }
        let cost = result["total_cost"].to_int();
        let item = "\x1b[36m" + actual + "\x1b[0;37m";
        self_text = "당신이 " + item + " " + count.to_string()
            + "개를 은전 " + cost.to_string() + "개에 구입합니다.";
        room_text = actor + " " + item + " " + count.to_string()
            + "개를 은전 " + cost.to_string() + "개에 구입합니다.";
    }
    send_line(ob, self_text);
    for player in get_cast_room_players(ob) {
        if player["name"] != own_name && !player["reject_fight"] {
            send_to_user(player["name"], room_text);
        }
    }
}

fn main(ob, line) { cmd(ob, line); }
