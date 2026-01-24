// 쉬어.rhai - 쉬기 명령어
// Python: cmds/쉬어.py

fn cmd(ob, line) {
    // ob.env.checkAttr('쉼금지'), ob.act 설정 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
