// 올숙리스트.rhai - 올숙리스트. Python: cmds/올숙리스트.py
// Item.Items, 사용자!=0. 관리자 1000.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        send_line(ob, "[올숙아이템인덱스] (Item.Items 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
