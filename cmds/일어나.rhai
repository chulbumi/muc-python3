// 일어나.rhai - 일어나기. Python: cmds/일어나.py. set_act(ob,"서")

fn cmd(ob, line) {
    let a = get_int(ob, "act");
    if a == 2 {
        set_act(ob, "서");
        send_line(ob, "당신이 운기조식을 마치며 벌떡 일어섭니다.");
    } else if a == 0 {
        send_line(ob, "☞ 이미 서있는 상태입니다. ^^");
    } else {
        send_line(ob, "☞ 운기조식중인 상태가 아닙니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
