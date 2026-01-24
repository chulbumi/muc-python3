// 수령.rhai - 수령. Python: cmds/수령.py
// [금액], 표두(표국무사), mob['은전'], ob['수령액'], 마지막수령.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [금액] 수령");
        return;
    }
    send_line(ob, "☞ 이곳에 표국무사가 없네요.");
}

fn main(ob, line) {
    cmd(ob, line);
}
