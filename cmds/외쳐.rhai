// 외쳐.rhai - 외침. [내용] 외쳐. send_shout(ob, msg) → Shout
// Python: cmds/외쳐.py. 외침거부/운기조식/길이/성격·관리자 타입은 efun에서 검사

fn cmd(ob, line) {
    let msg = trim(line);
    if length(msg) == 0 {
        send_line(ob, "☞ 사용법: [내용] 외침(,)");
        return;
    }
    send_shout(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
