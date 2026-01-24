// 낚시.rhai - 낚시. Python: cmds/낚시.py
// reactor.callLater(3, down); down: "엄청난것이 걸린것 같다", callLater(3, down1); down1: "젠장! 낚시줄이 끊어졌다"
// call_out(이름, "낚시_2", 3), call_out(이름, "낚시_3", 6) — 지연 콜백에서는 call_out 미등록이라 cmd에서 한번에 스케줄.

fn cmd(ob, line) {
    send_line(ob, "낚시바늘에 미끼를 끼우고 낚시대를 드리웁니다.");
    call_out(get_string(ob, "이름"), "낚시_2", 3);
    call_out(get_string(ob, "이름"), "낚시_3", 6);
}
fn 낚시_2(ob) {
    send_line(ob, "낚시줄에 엄청난것이 걸린것 같다...");
}
fn 낚시_3(ob) {
    send_line(ob, "젠장! 낚시줄이 끊어졌다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
