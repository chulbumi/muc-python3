// 누가주나.rhai - 특정 아이템을 가진 몹 검색
// Python: cmds/누가주나.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 운영자 명령: [아이템인덱스] 누가주나");
        return;
    }
    for holder in get_mob_item_holders(line) {
        send_line(ob, holder["name"] + " : " + holder["index"]);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
