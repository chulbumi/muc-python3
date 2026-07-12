// 옵설정.rhai - 아이템 옵션 설정
// Python: cmds/옵설정.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let words = line_args(line);
    if len(words) < 3 {
        send_line(ob, "☞ 사용법: [대상] [키] [값] 옵설정");
        return;
    }
    let result = set_inventory_option(ob, words[0], words[1], to_int(words[2]));
    if result == "ok" { send_line(ob, "☞ 설정되었습니다."); }
    else { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
