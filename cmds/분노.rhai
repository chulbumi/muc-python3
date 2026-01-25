// 분노.rhai - 분노. Python: cmds/분노.py
// ob.act==ACT_FIGHT, ob.target, ob['분노']>=100, getGuardNum(호위), makeFightScript.

fn cmd(ob, line) {
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
