// 청소.rhai - 청소. Python: cmds/청소.py
// gc.collect. 관리자 1000 (원래 2000, 코드에 1000).

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        send_line(ob, "* 청소중입니다. (gc 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
