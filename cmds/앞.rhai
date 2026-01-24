// 앞.rhai - 앞으로 이동 (관리자 명령)
// Python: cmds/앞.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { return; }
    if !require_arg(ob, line, "☞ 운영자 명령: [대상] 앞") { return; }
    // ob.channel.players, enterRoom 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
