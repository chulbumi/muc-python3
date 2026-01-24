// 방어구찾기.rhai - 방어구찾기. Python: cmds/방어구찾기.py
// Item.Items, 종류=='방어구'. 관리자 1000.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        send_line(ob, "☞ 찾으시는 아이템이 없습니다. (Item.Items 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
