// 입어.rhai - 장비 착용. Python: cmds/입어.py. item_equip(ob,name,order), parse_order_name

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [아이템 이름] 입어");
        return;
    }
    let arr = parse_order_name(line);
    let ord = arr[0];
    let nm = arr[1];
    let err = item_equip(ob, nm, ord);
    if err == "usage" { send_line(ob, "☞ 사용법: [아이템 이름] 입어"); return; }
    if err == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if err == "not_equippable" { send_line(ob, "☞ 착용할 수 있는것이 아니에요."); return; }
    if err == "slot_used" { send_line(ob, "☞ 더 이상 착용이 불가능해요."); return; }
    if err != "" { return; }
    let eul = han_eul(nm);
    send_line(ob, "당신이 \x1b[36m" + nm + eul + "\x1b[37m 착용합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
