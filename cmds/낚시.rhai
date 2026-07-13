// 낚시.rhai - 낚시. Python: cmds/낚시.py
// reactor.callLater(3, down); down: "엄청난것이 걸린것 같다", callLater(3, down1); down1: "젠장! 낚시줄이 끊어졌다"
// Python처럼 첫 콜백이 실행된 시점에 두 번째 3초 콜백을 예약한다.

fn cmd(ob, line) {
    send_line(ob, "낚시바늘에 미끼를 끼우고 낚시대를 드리웁니다.");
    call_out(get_connection_token(ob), "fishing_2", 3);
}
fn fishing_2(ob) {
    set_value(ob, "cooltime", 0);
    send_line(ob, "낚시줄에 엄청난것이 걸린것 같다...");
    call_out(get_connection_token(ob), "fishing_3", 3);
}
fn fishing_3(ob) {
    set_value(ob, "cooltime", 0);
    send_line(ob, "젠장! 낚시줄이 끊어졌다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
