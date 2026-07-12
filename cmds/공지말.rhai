// 공지말.rhai - 공지말 명령어 (관리자 전원 공지)
// Python: cmds/공지말.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 공지말");
        return;
    }
    if length(line) > 200 {
        send_line(ob, "☞ 너무 깁니다.");
        return;
    }
    send_notice(ob, line);
}

fn main(ob, line) {
    cmd(ob, line);
}
