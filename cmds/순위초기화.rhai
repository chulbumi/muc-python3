// 순위초기화.rhai - 순위초기화. Python: cmds/순위초기화.py
// RANK.attr = {} or RANK.attr[line]=[]. 관리자 2000.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        if line == "" || is_empty(line) {
            send_line(ob, "* 전체가 초기화 되었습니다. (RANK 연동 예정)");
        } else {
            send_line(ob, "* 초기화 되었습니다. (RANK 연동 예정)");
        }
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
