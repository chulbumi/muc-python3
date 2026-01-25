// 벗어.rhai - 장비 해제. Python: cmds/벗어.py. item_unequip, item_unequip_all, parse_order_name

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [아이템 이름] 벗어  또는  [모두/전부] 벗어");
        return;
    }
    let t = trim(line);
    if t == "모두" || t == "전부" {
        let n = item_unequip_all(ob);
        if n == 0 {
            send_line(ob, "☞ 착용중인 장비가 없어요.");
        } else {
            send_line(ob, "당신이 착용한 장비 " + int_to_str(n) + "개를 해제했습니다.");
        }
        return;
    }
    let arr = parse_order_name(line);
    let ord = arr[0];
    let nm = arr[1];
    let err = item_unequip(ob, nm, ord);
    if err == "usage" { send_line(ob, "☞ 사용법: [아이템 이름] 벗어  또는  [모두/전부] 벗어"); return; }
    if err == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); return; }
    if err != "" { return; }
    let eul = han_eul(nm);
    send_line(ob, "당신이 \x1b[36m" + nm + eul + "\x1b[37m 착용해제 합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
