// Python cmds/조회.py의 보험료 조회

fn cmd(ob, line) {
    let view = get_insurance_view(ob);
    if !view["has_agent"] {
        send_line(ob, "☞ 이곳에 표국무사가 없네요.");
        return;
    }
    let msg = "당신의 보험료 총액은 은전 \x1b[1m" + int_to_str(view["premium"])
        + "\x1b[0;37m개이며\r\n보험 혜택은 \x1b[1m" + int_to_str(view["count"])
        + "\x1b[0m\x1b[40m\x1b[37m번 받으실 수 있습니다.\r\n";
    msg += "보험혜택이 적용되는 금액은 은전 \x1b[1m" + int_to_str(view["threshold"])
        + "\x1b[0;37m개 이상이며\r\n";
    msg += "한번의 출장 처리시엔 은전 \x1b[1m" + int_to_str(view["trip_cost"])
        + "\x1b[0;37m개가 소요됩니다.";
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
