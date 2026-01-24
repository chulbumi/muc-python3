// 정리.rhai - 정리. Python: cmds/정리.py
// [사용자명] channel.players, do_command('끝'), loseConnection. 관리자 1000.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 사용법: [사용자명] 정리");
            return;
        }
        send_line(ob, "☞ (channel.players/loseConnection 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
