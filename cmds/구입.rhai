// 구입.rhai - 구입 명령어
// Python: cmds/구입.py

fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [물품이름] [수량] 구입"); return; }
    let args = line_args(line);
    let name = args[0];
    let cnt = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    if cnt < 1 { return; }
    let r = merchant_buy(ob, name, cnt);
    let e = r["err"];
    if e == "usage" { send_line(ob, "☞ 사용법: [물품이름] [수량] 구입"); return; }
    if e == "no_merchant" { send_line(ob, "☞ 품목을 보여줄 상인이 없어요. ^^"); return; }
    if e == "not_for_sale" { send_line(ob, "☞ 그런 물건은 팔지 않아요. ^_^"); return; }
    if e == "inv_full" { send_line(ob, "☞ 자네가 가질 물품의 한계라네"); return; }
    if e == "too_heavy" { send_line(ob, "☞ 무거워서 더 이상 가질 수 없어요. ^^"); return; }
    if e == "no_money" { send_line(ob, "☞ 돈이 모자라네요. ^^"); return; }
    if length(e) > 0 { return; }
    let c = r["bought"];
    if c == 0 { return; }
    let dn = r["display_name"];
    let cost = r["total_cost"];
    send_line(ob, "당신이 \x1b[36m" + dn + han_eul(dn) + "\x1b[37m " + c + "개를 은전 " + cost + "개에 구입합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
