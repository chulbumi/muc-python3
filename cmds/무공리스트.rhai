// 무공리스트.rhai - 무공리스트. Python: cmds/무공리스트.py
// MUGONG.attr, 관리자 1000.

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
