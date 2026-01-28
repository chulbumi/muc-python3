// 꺼내.rhai - 보관함에서 물건 꺼내기
// Python: cmds/꺼내.py
// 사용법: [보관함이름] [물품] 꺼내

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 꺼내");
        return;
    }

    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 꺼내");
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

    // 은전 꺼내기
    if item_type == "은전" {
        let mut amount = 1;
        if len(words) >= 3 {
            amount = to_int(words[2]);
            if amount < 1 { amount = 1; }
        }

        // 돈 꺼내기
        if box_withdraw_money(ob, box_name, amount) {
            send_line(ob, "당신이 " + box_name + "에서 은전 " + amount + "냥을 꺼냅니다.");
            body_save(ob);
        } else {
            send_line(ob, "☞ 꺼낼 돈이 모자라네요.");
        }
        return;
    }

    // TODO: 보관함 내용물 확인 및 꺼내기
    // box.objs에서 아이템을 꺼내서 ob에 추가
    send_line(ob, "☞ '" + item_type + "' 꺼내기 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
