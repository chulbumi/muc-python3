// 모두끝.rhai - 모두끝. Python: cmds/모두끝.py
// Client.players 순회, do_command('끝'). 관리자 2000. Client/do_command efun 없음.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        send_line(ob, "☞ (Client.players/do_command 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
