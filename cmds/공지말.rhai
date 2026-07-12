// 공지말.rhai - 공지말 명령어 (관리자 전원 공지)
// Python: cmds/공지말.py

fn cmd(ob, line) {
    if length(line) == 0 {
        send_line(ob, "☞ 운영자 명령: [내용] 공지말");
        return;
    }
    let padded = if length(line) >= 68 { line } else { line + repeat(" ", 68 - length(line)) };
    let buf = "───────────────────────────────────────\r\n"
        + "\x1b[7m☞ 공지 : " + padded + "\x1b[0m\r\n"
        + "───────────────────────────────────────";
    send_notice(ob, buf);
}

fn main(ob, line) {
    cmd(ob, line);
}
