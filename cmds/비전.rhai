// 비전.rhai - 비전(오의) 설정
// Python: cmds/비전.py
fn cmd(ob, line) {
    if line == "" {
        let current = get_string(ob, "비전설정");
        if current == "" {
            send_line(ob, "☞ 비전 : 없음");
        } else {
            send_line(ob, "☞ 비전 : [\x1b[1;37m" + current + "\x1b[0;37m]");
        }
        return;
    }

    if !has_vision(ob, line) {
        send_line(ob, "☞ 당신은 그런 비전을 배운적이 없습니다.");
        return;
    }

    set_vision(ob, line);
    send_line(ob, "☞ 비전을 지정하였습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
