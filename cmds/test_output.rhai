// test_output.rhai - Test output command
fn cmd(ob, line) {
    send_line(ob, "TEST OUTPUT");
    send_line(ob, "LINE 2");
}

fn main(ob, line) {
    cmd(ob, line);
}
