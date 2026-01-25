// 방파말.rhai - 방파말 명령어 (같은 방파/소속 접속자에게)
// Python: cmds/방파말.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 방파말");
        return;
    }
    let r = send_broadcast_to_guild(ob, line);
    if r == "no_guild" { send_line(ob, "☞ 소속된 방파가 없어요."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
