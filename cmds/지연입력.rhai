// 지연입력.rhai - 지연 입력
// Python: cmds/지연입력.py

fn delayed_execute(ob, command) {
    queue_self_command(ob, command);
}

fn cmd(ob, line) {
    if line == "" { send_line(ob, "☞ 사용법: [입력글] 지연입력"); return; }
    call_out_arg(get_connection_token(ob), "delayed_execute", 1, line);
}

fn main(ob, line) {
    cmd(ob, line);
}
