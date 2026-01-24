// 저장.rhai - 저장 명령어
// Python: cmds/저장.py

fn cmd(ob, line) {
    let res = body_save(ob);
    if res {
        send_line(ob, "* 저장 되었습니다.");
    } else {
        send_line(ob, "* 저장 실패!!!");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
