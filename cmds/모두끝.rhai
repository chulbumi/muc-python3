// 모두끝.rhai - 모두끝. Python: cmds/모두끝.py
// Client.players 순회, do_command('끝'). 관리자 2000. Client/do_command efun 없음.

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
