// 숙련도.rhai - 무기 숙련도 표. Python: cmds/숙련도.py
// 1=검, 2=도, 3=창, 4=기타, 5=맨손

fn cmd(ob, line) {
    let s1 = get_int(ob, "1 숙련도");
    let s2 = get_int(ob, "2 숙련도");
    let s3 = get_int(ob, "3 숙련도");
    let s4 = get_int(ob, "4 숙련도");
    let s5 = get_int(ob, "5 숙련도");

    send_line(ob, "\x1b[1m ★ 당신의 무기 숙련도 ★\x1b[0m\x1b[40m\x1b[37m");
    send_line(ob, "┏─────┬─────┓");
    send_line(ob, "│◁  검  ▷│\x1b[1m" + pad_start(int_to_str(s1), 10, " ") + "\x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "├─────┼─────┤");
    send_line(ob, "│◁  도  ▷│\x1b[1m" + pad_start(int_to_str(s2), 10, " ") + "\x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "├─────┼─────┤");
    send_line(ob, "│◁  창  ▷│\x1b[1m" + pad_start(int_to_str(s3), 10, " ") + "\x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "├─────┼─────┤");
    send_line(ob, "│◁ 기타 ▷│\x1b[1m" + pad_start(int_to_str(s4), 10, " ") + "\x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "├─────┼─────┤");
    send_line(ob, "│◁ 맨손 ▷│\x1b[1m" + pad_start(int_to_str(s5), 10, " ") + "\x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "┗─────┴─────┛");
}

fn main(ob, line) {
    cmd(ob, line);
}
