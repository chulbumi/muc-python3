// 수령.rhai - 수령. Python: cmds/수령.py
// [금액], 표두(표국무사), mob['은전'], ob['수령액'], 마지막수령.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [금액] 수령");
        return;
    }
    let amount = to_int(trim(line));
    let result = receive_from_guard(ob, amount);
    let status = result["status"];
    if status == "no_guard" { send_line(ob, "☞ 이곳에 표국무사가 없네요."); return; }
    if status == "invalid_amount" { send_line(ob, "☞ 은전 1개 이상 입력 하셔야 해요."); return; }
    if status == "high_level" { send_line(ob, "☞ 충분한 능력이 있어 보이는데요???"); return; }
    if status == "too_greedy" { send_line(ob, "☞ 너무 욕심이 크군요???"); return; }
    if status == "fund_short" { send_line(ob, "☞ 기부금이 모잘라요^^;"); return; }
    if status == "total_limit" { send_line(ob, "☞ 더이상 수령은 곤란해요^^;"); return; }
    if status == "over_limit" { send_line(ob, "☞ 한도 초과에요!!!"); return; }
    if status == "too_soon" { send_line(ob, "☞ 또 오셨어요???"); return; }
    if status != "ok" { return; }
    send_line(ob, "당신이 은전 " + amount.to_string() + "개를 표국무사에게 수령합니다.\r\n"
        + "현재까지 수령한 기부금 총액은 은전 \x1b[1m"
        + result["total"].to_string() + "\x1b[0;37m개 입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
