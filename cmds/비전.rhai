// 비전.rhai - 비전. Python: cmds/비전.py
// line=='': ob['비전설정']. else: ob['비전이름']에 line 있으면 ob['비전설정']=line.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        let v = get_string(ob, "비전설정");
        if v == "" || v == () {
            send_line(ob, "☞ 비전 : 없음");
        } else {
            send_line(ob, "☞ 비전 : [\x1b[1;37m" + v + "\x1b[0;37m]");
        }
        return;
    }
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
