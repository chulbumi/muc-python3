// 판매.rhai - 판매 명령어 (Python: cmds/판매.py)
// 상인(물건구입)이 있는 방에서 [이름] [수량] 판매

fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [아이템 이름] [수량] 판매"); return; }
    let args = line_args(line);
    let parsed = parse_order_name(args[0]);
    let order = parsed[0];
    let name = parsed[1];
    let cnt = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    let cnt = if cnt < 1 { 1 } else { if cnt > 100 { 100 } else { cnt } };

    let percent = get_merchant_buy_percent(ob);
    if percent <= 0 {
        send_line(ob, "☞ 그런 물건을 살 상인이 없어요. ^_^");
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
    send_line(ob, "당신이 " + ret[2] + " " + ret[0] + "개를 은전 " + ret[1] + "개에 판매합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
