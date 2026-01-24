// 분노.rhai - 분노. Python: cmds/분노.py
// ob.act==ACT_FIGHT, ob.target, ob['분노']>=100, getGuardNum(호위), makeFightScript.

fn cmd(ob, line) {
    send_line(ob, "☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네 (act/target/분노/호위 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
