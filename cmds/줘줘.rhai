// 줘줘.rhai - 여러 아이템 주기
// Python: cmds/줘줘.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let words = split(trim(line), " ");
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [물품] 주다");
        return;
    }
    let target = words[0];
    let item = words[1];
    if item == "은전" {
        let amount = if len(words) >= 3 { to_int(words[2]) } else { 1 };
        let result = request_give_silver(ob, target, if amount > 0 { amount } else { 1 });
        if result == "" {
            send_line(ob, "당신이 " + target + "에게 은전을 줍니다.");
        } else {
            send_line(ob, "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^");
        }
        return;
    }
    let count = if len(words) >= 3 { to_int(words[2]) } else { 1 };
    let result = request_give_item(ob, target, item, 1, if count > 0 { count } else { 1 });
    if result == "no_item" {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }
    if result != "" {
        send_line(ob, "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^");
        return;
    }
    send_line(ob, "당신이 " + target + "에게 " + item + "을(를) 줍니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
