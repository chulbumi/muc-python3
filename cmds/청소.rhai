// 청소.rhai - 메모리 청소 (관리자)
// Python: cmds/청소.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    // Python calls gc.collect(1) and prints the collected-object count.  Rust
    // has no tracing GC; keep the command's observable contract without
    // claiming a collection that did not happen.
    send_line(ob, "* 청소중입니다. 0");
}

fn main(ob, line) {
    cmd(ob, line);
}
