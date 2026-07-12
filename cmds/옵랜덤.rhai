// 옵랜덤.rhai - 아이템 옵션 랜덤
// Python: cmds/옵랜덤.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 옵랜덤");
        return;
    }
    let result = apply_item_magic(ob, line, get_int(ob, "레벨"));
    if result == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); }
    else { send_line(ob, "☞ 설정되었습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
