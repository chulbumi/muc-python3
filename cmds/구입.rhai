// 구입.rhai - 구입 명령어
// Python: cmds/구입.py

fn cmd(ob, line) {
    if !require_arg(ob, line, "☞ 사용법: [물품이름] [수량] 구입") { return; }
    let args = line_args(line);
    let name = args[0];
    let cnt = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    if cnt < 1 { return; }
    let r = merchant_buy(ob, name, cnt);
    if length(r["err"]) > 0 {
        send_line(ob, r["err"]);
        return;
    }
    let c = r["bought"];
    if c == 0 { return; }
    let dn = r["display_name"];
    let cost = r["total_cost"];
    send_line(ob, "당신이 \x1b[36m" + dn + han_eul(dn) + "\x1b[37m " + c + "개를 은전 " + cost + "개에 구입합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
