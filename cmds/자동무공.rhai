// 자동무공.rhai - 자동무공. Python: cmds/자동무공.py
// line=='': ob['자동무공']. else: ob.skillList, MUGONG, ob['자동무공']=s.name.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        let v = get_string(ob, "자동무공");
        if v == "" || v == () {
            send_line(ob, "☞ 자동무공 : 없음");
        } else {
            send_line(ob, "☞ 자동무공 : [\x1b[1;37m" + v + "\x1b[0;37m]");
        }
        return;
    }
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
