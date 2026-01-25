// 청소.rhai - 청소. Python: cmds/청소.py
// gc.collect. 관리자 1000 (원래 2000, 코드에 1000).

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
    } else {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
