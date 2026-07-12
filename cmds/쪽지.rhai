// Python: cmds/쪽지.py, objs/player.py Player.write_memo

fn view_memo(ob) {
    let memos = take_received_notes(ob);
    if len(memos) == 0 {
        send_line(ob, "도착한 쪽지가 없습니다.");
        return;
    }

    let msg = "┌────────────────────────────────────┐\r\n"
        + "│◁                    무           림           첩                    ▷│\r\n"
        + "└────────────────────────────────────┘\r\n";
    for i in 0..len(memos) {
        let memo = memos[i];
        msg += "\x1b[33m보 낸 이\x1b[37m : " + memo["작성자"] + "\r\n";
        msg += "\x1b[33m제    목\x1b[37m : " + memo["제목"] + "\r\n";
        msg += "\x1b[33m작성시각\x1b[37m : " + memo["시간"] + "\r\n\r\n";
        msg += memo["내용"] + "\r\n";
        msg += " ─────────────────────────────────────";
        if i + 1 < len(memos) {
            msg += "\r\n";
        }
    }
    send_line(ob, msg);
}

fn cmd(ob, line) {
    let position = get_my_position(ob);
    if position["zone"] != "낙양성" || position["room"] != "11" {
        send_line(ob, "정보수집소에서 할 수 있습니다.");
        return;
    }

    if line == "" {
        view_memo(ob);
        return;
    }

    let words = parse_note_header(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [이름] [제목] 쪽지");
        return;
    }
    let name = words[0];
    let subject = words[1];

    for connected_name in get_connected_player_names() {
        if connected_name == name {
            send_line(ob, "접속중인 사용자에게는 보낼 수 없습니다.");
            return;
        }
    }

    let prompt = "[" + name + "]님에게 쪽지를 작성합니다. 끝내시려면 '.'를 치세요.\r\n"
        + "분량 제한은 10줄입니다.\r\n:";
    let result = begin_note_edit(
        ob,
        name,
        subject,
        prompt,
        "사용자가 접속하였으므로 작성을 마칩니다.\r\n",
        "제한용량을 초과하였습니다.\r\n",
        "쪽지 작성을 마칩니다.\r\n",
        ":"
    );
    if result == "not_found" {
        send_line(ob, "존재하지않는 사용자입니다.");
    } else if result == "duplicate" {
        send_line(ob, "한번 보냈던 사용자에게는 다시 보낼 수 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
