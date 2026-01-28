// 무공전수2.rhai - 무공 전수 (다른 대상에게 전수)
// Python: cmds/무공전수2.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 명령: 무공전수2 [대상] [무공이름]");
        return;
    }
    send_line(ob, "☞ 무공 전수 기능은 현재 구현 중입니다.");
    send_line(ob, "   (다른 대상에게 무공을 전수)");
}

fn main(ob, line) {
    cmd(ob, line);
}
