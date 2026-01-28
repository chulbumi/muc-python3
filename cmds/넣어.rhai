// 넣어.rhai - 보관함에 물건 넣기
// Python: cmds/넣어.py
// 사용법: [보관함이름] [물품/은전/모두] 넣어

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 넣어");
        return;
    }

    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 넣어");
        return;
    }

    let box_name = words[0];
    let item_type = words[1];

    // 방에서 보관함 찾기
    let found_box_name = find_box_in_room(ob);
    if found_box_name.is_empty() {
        send_line(ob, "☞ 이곳에는 보관함이 없네");
        return;
    }

    // 이름이 일치하는지 확인
    if found_box_name != box_name {
        send_line(ob, "☞ 이곳에는 '" + box_name + "' 보관함이 없네");
        return;
    }

    // 은전 넣기
    if item_type == "은전" {
        let mut amount = 1;
        if len(words) >= 3 {
            amount = to_int(words[2]);
            if amount < 1 { amount = 1; }
        }

        let my_money = get_int(ob, "은전");
        if my_money < amount {
            send_line(ob, "☞ 돈이 모자라네요. ^^");
            return;
        }

        // 돈 보관
        if box_deposit_money(ob, box_name, amount) {
            send_line(ob, "당신이 " + box_name + "에 은전 " + amount + "냥을 보관합니다.");
            body_save(ob);
        } else {
            send_line(ob, "☞ 보관 실패!");
        }
        return;
    }

    // 모두 넣기
    if item_type == "모두" {
        send_line(ob, "☞ '모두' 기능은 현재 구현 중입니다.");
        return;
    }

    // 속성아이템 넣기
    if item_type == "속성아이템" {
        send_line(ob, "☞ '속성아이템' 기능은 현재 구현 중입니다.");
        return;
    }

    // 약초 넣기
    if item_type == "약초" {
        send_line(ob, "☞ '약초' 기능은 현재 구현 중입니다.");
        return;
    }

    // 속성방어구 넣기
    if item_type == "속성방어구" {
        send_line(ob, "☞ '속성방어구' 기능은 현재 구현 중입니다.");
        return;
    }

    // 속성무기 넣기
    if item_type == "속성무기" {
        send_line(ob, "☞ '속성무기' 기능은 현재 구현 중입니다.");
        return;
    }

    // 개별 아이템 넣기
    let mut count = 1;
    if len(words) >= 3 {
        count = to_int(words[2]);
        if count < 1 { count = 1; }
    }

    send_line(ob, "☞ 아이템 보관 기능은 현재 구현 중입니다.");
    // TODO: item_drop으로 방에 버리고, 보관함 오브젝트에 추가
}

fn main(ob, line) {
    cmd(ob, line);
}
