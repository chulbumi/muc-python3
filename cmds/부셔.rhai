// 부셔.rhai - 아이템 부수기 (Python: cmds/부셔.py)
fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [물품이름] [수량] 부숴"); return; }
    let args = line_args(line);
    let count = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    let count = if count < 1 { 1 } else { if count > 100 { 100 } else { count } };
    let parsed = parse_order_name(args[0]);
    let order = parsed[0];
    let name = parsed[1];
    let c = if order != 1 { 1 } else { count };
    let n = item_destroy_busha(ob, name, order, c);
    if n == -1 {
        send_line(ob, "☞ 부셔지지 않네요. ^^");
        return;
    }
    if n == 0 {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }
    send_line(ob, format_item_action_self(name, "부셔버립니다.", n));
}

fn main(ob, line) {
    cmd(ob, line);
}
