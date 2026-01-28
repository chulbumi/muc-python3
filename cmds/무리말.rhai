// 무리말.rhai - 무리/파티 채팅
// Python: cmds/무리말.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 무리말");
        return;
    }
    send_line(ob, "☞ 무리 채팅 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
