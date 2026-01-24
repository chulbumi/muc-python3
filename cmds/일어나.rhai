// 일어나.rhai - 일어나기 명령어
// Python: cmds/일어나.py

fn cmd(ob, line) {
    // ob.act 확인/설정, send_room 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
