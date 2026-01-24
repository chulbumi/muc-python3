// 기연리스트.rhai - Python: cmds/기연리스트.py
// msg='[단일아이템인덱스]\\r\\n'; for index in Item.Items: item=Item.Items[index]; if item.isOneItem(): msg += '#%s\\r\\n:%s\\r\\n\\r\\n'. sendLine(msg). Item.Items/isOneItem 연동 예정.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        return;
    }
    send_line(ob, "[단일아이템인덱스]\n(Item.Items/isOneItem 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
