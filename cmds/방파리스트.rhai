// 방파리스트.rhai - 방파 리스트
// Python: cmds/방파리스트.py

fn cmd(ob, line) {
    // GUILD.attr, 관리자 등급 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
