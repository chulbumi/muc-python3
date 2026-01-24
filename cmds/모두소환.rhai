// 모두소환.rhai - 모두소환. Python: cmds/모두소환.py
// Client.players, enterRoom(ob.env, '소환','소환'). 관리자 2000.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        send_line(ob, "☞ (Client.players/enterRoom 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
