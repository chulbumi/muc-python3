// 기연리스트.rhai - 기연리스트. Python: cmds/기연리스트.py
// Item.Items, isOneItem() → get_global/데이터 연동 없음.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        send_line(ob, "[단일아이템인덱스] (연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
