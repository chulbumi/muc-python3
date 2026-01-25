// 먹어.rhai - 소모품 사용. Python: cmds/먹어.py. item_use_consumable(ob,name,order)

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [아이템 이름] 먹어");
        return;
    }
    let arr = parse_order_name(line);
    let ord = arr[0];
    let nm = arr[1];
    let r = item_use_consumable(ob, nm, ord);
    let e = r["err"];
    if e == "usage" { send_line(ob, "☞ 사용법: [아이템 이름] 먹어"); return; }
    if e == "bad_state" { send_line(ob, "☞ 먹을 수 있는 상황이 아니네요. ^_^"); return; }
    if e == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if e == "not_consumable" { send_line(ob, "☞ 먹을 수 있는것이 아니에요. ^_^"); return; }
    if e != "" { return; }
    let name = r["name"];
    let eul = han_eul(name);
    send_line(ob, "당신이 \x1b[36m" + name + eul + "\x1b[37m 먹습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
