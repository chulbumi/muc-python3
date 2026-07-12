// Python `cmds/기부.py`: 같은 방 표두에게 은전을 기탁한다.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [금액] 기부");
        return;
    }
    let result = donate_to_guard(ob, to_int(line));
    if result["status"] == "no_guard" {
        send_line(ob, "☞ 이곳에 표국무사가 없네요.");
        return;
    }
    if result["status"] == "invalid_amount" {
        send_line(ob, "☞ 은전 1개 이상 입금 하셔야 해요.");
        return;
    }
    send_line(ob, "당신이 은전 " + result["amount"].to_string() + "개를 표국무사에게 기탁합니다.\r\n"
        + "현재까지 모여진 기부금 총액은 은전 \x1b[1m" + result["total"].to_string()
        + "\x1b[0;37m개 입니다.");
}

fn main(ob, line) { cmd(ob, line); }
