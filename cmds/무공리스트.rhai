// 무공리스트.rhai - 무공리스트. Python: cmds/무공리스트.py
// MUGONG.attr, 관리자 1000.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        send_line(ob, "☞ (MUGONG.attr 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
