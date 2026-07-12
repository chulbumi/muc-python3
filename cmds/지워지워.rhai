// 지워지워.rhai - 기록 삭제
// Python: cmds/지워지워.py

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [아이템 이름] 먹어");
        return;
    }
    let result = clear_item_magic(ob, line);
    if result == "no_item" { send_line(ob, "☞ 그런 아이템이 소지품에 없어요."); }
    else if result == "no_option" { }
    else { send_line(ob, result); }
}

fn main(ob, line) {
    cmd(ob, line);
}
