// 분해.rhai - 아이템 분해
// Python: cmds/분해.py

fn cmd(ob, line) {
    if line != "모두" { send_line(ob, "☞ 사용법: [모두] 분해"); return; }
    let count = decompose_all_items(ob);
    if count < 0 { send_line(ob, "☞ 상인이 없어요. ^_^"); return; }
    if count == 0 { send_line(ob, "☞ 그런 아이템이 없어요."); return; }
    send_line(ob, "당신이 아이템을 분해합니다.");
    send_line(ob, "☞ 강철조각 " + count + "개를 얻었습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
