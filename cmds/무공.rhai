// 무공.rhai - 무공. Python: cmds/무공.py
// MUGONG.attr 순회. get_global/스킬 목록 efun 없음.

fn cmd(ob, line) {
    send_line(ob, "☞ 습득한 무공이 없습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
