// 모두저장.rhai - 모두저장. Python: cmds/모두저장.py
// Client.players, ply.save(). 관리자 2000.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        send_line(ob, "☞ (Client.players/save 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
