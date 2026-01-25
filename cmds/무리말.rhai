// 무리말.rhai - 무리말 명령어 (무리/파티 채널)
// Python: cmds/무리말.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 무리말");
        return;
    }
    send_line(ob, "☞ 무리 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
