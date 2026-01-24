// 누구.rhai - 무림인 목록 명령어
// Python: cmds/누구.py

fn cmd(ob, line) {
    // ob.channel.players, 플레이어 목록 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
