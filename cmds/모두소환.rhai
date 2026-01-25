// 모두소환.rhai - 모두소환. Python: cmds/모두소환.py
// Client.players, enterRoom(ob.env, '소환','소환'). 관리자 2000.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
    } else {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
