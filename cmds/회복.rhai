// 회복.rhai - 회복. Python: cmds/회복.py
// 관리자 1000. ob['체력']=getMaxHp(), ob['내공']=getMaxMp(). set_value(ob, "hp", max_hp), set_value(ob, "mp", max_mp).

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        set_value(ob, "hp", get_int(ob, "max_hp"));
        set_value(ob, "mp", get_int(ob, "max_mp"));
        send_line(ob, "* 회복되었습니다.");
    } else {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
