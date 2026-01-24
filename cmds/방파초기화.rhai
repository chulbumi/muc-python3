// 방파초기화.rhai - 방파초기화. Python: cmds/방파초기화.py
// [방파이름] or '', GUILD, getRoom, room.attr. 관리자 2000.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 사용법: [방파이름] 방파초기화");
            return;
        }
        send_line(ob, "* 방파가 초기화되었습니다. (GUILD/room 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
