// 리젠.rhai - 리젠. Python: cmds/리젠.py
// line=='': ob.env.objs 중 is_mob, ACT_DEATH/ACT_REGEN → doRegen. [이름]: findObjName, 시체만 doRegen. env.objs/doRegen efun 없음.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 리젠될 몹이 없어요!!");
            return;
        }
        send_line(ob, "☞ 시체만 가능합니다. *^_^* (env.objs/doRegen 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
