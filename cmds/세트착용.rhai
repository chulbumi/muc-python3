// 세트착용.rhai - 장비 세트 착용
// Python: cmds/세트착용.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 명령: 세트착용 [세트이름]");
        send_line(ob, "   (저장해둔 장비 세트를 한번에 착용)");
        return;
    }
    send_line(ob, "☞ 장비 세트 착용 기능은 현재 구현 중입니다.");
    send_line(ob, "   (세트이름: '" + line + "')");
}

fn main(ob, line) {
    cmd(ob, line);
}
