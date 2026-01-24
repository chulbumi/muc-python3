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
    if r["err"] != "" {
        send_line(ob, r["err"]);
        return;
    }
    let name = r["name"];
    let eul = han_eul(name);
    send_line(ob, "당신이 \x1b[36m" + name + eul + "\x1b[37m 먹습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
