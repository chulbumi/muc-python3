// 회복.rhai - 회복. Python: cmds/회복.py
// 관리자 1000. ob['체력']=getMaxHp(), ob['내공']=getMaxMp().

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        set_value(ob, "체력", get_int(ob, "max_hp"));
        set_value(ob, "내공", get_int(ob, "max_mp"));
        send_line(ob, "* 회복되었습니다.");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
