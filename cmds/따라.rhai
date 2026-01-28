// 따라.rhai - 자동 따라가기
// Python: cmds/따라.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 명령: 따라 [대상이름]");
        send_line(ob, "   (대상이 이동할 때 자동으로 따라감)");
        return;
    }
    send_line(ob, "☞ 자동 따라가기 기능은 현재 구현 중입니다.");
    send_line(ob, "   (대상: '" + line + "')");
}

fn main(ob, line) {
    cmd(ob, line);
}
