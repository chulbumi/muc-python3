// 리부팅.rhai - 리부팅. Python: cmds/리부팅.py
// reactor.stop(), updateZones. 서버 종료/존갱신은 Rust에서만.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        send_line(ob, "☞ 운영자 전용. 서버 재시작은 관리자 콘솔에서 하세요.");
    } else {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
