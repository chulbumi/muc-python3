// 리젠.rhai - Python: cmds/리젠.py
// 인자 없음: ob.env.objs에서 is_mob이고 act==ACT_DEATH|ACT_REGEN인 것 doRegen. c>0이면 "리젠되었습니다", 0이면 "리젠될 몹이 없어요!!"
// [이름]: findObjName→몹, is_item/is_player면 "무슨 말인지", act!=ACT_DEATH면 "시체만 가능", else doRegen. env.objs/findObjName/doRegen 연동 예정.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        return;
    }
    if line == "" || is_empty(line) {
        // ob.env.objs 순회, is_mob, act in (ACT_DEATH,ACT_REGEN) → doRegen (연동 예정). c==0이면 아래 메시지.
        send_line(ob, "☞ 리젠될 몹이 없어요!!");
        return;
    }
    // ob.env.findObjName(line): None|is_item|is_player → "무슨 말인지"; act!=ACT_DEATH → "시체만 가능"; else doRegen (연동 예정)
    send_line(ob, "☞ 시체만 가능합니다. *^_^* (env.objs/findObjName/doRegen 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
