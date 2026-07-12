// 지연입력.rhai - 지연 입력
// Python: cmds/지연입력.py

fn delayed_execute(ob) {
    let command = get_string(ob, "_지연입력");
    set_value(ob, "_지연입력", "");
    queue_self_command(ob, command);
}

fn cmd(ob, line) {
    if is_empty(line) { send_line(ob, "☞ 사용법: [입력글] 지연입력"); return; }
    set_value(ob, "_지연입력", line);
    call_out(get_string(ob, "이름"), "delayed_execute", 1);
}

fn main(ob, line) {
    cmd(ob, line);
}
