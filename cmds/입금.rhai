// Python cmds/입금.py의 보험료 상태 전이/출력

fn cmd(ob, line) {
    let amount = trim(line);
    if amount == "" {
        send_line(ob, "☞ 사용법: [금액] 입금");
        return;
    }
    let result = deposit_insurance(ob, parse_int_prefix(amount));
    if !result["has_agent"] {
        send_line(ob, "☞ 이곳에 표국무사가 없네요.");
        return;
    }
    if result["status"] == "invalid_amount" {
        send_line(ob, "☞ 은전 1개 이상 입금 하셔야 해요.");
        return;
    }
    let msg = "당신이 은전 " + int_to_str(result["paid"]) + "개를 표국무사에게 입금합니다.\r\n\r\n";
    msg += "당신의 보험료 총액은 은전 \x1b[1m" + int_to_str(result["premium"])
        + "\x1b[0;37m개이며\r\n보험 혜택은 \x1b[1m" + int_to_str(result["count"])
        + "\x1b[0m\x1b[40m\x1b[37m번 받으실 수 있습니다.";
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
