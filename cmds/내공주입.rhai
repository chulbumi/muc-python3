// 내공주입.rhai - 내공주입. Python: cmds/내공주입.py
// 호위(ob.objs 중 종류=='호위') 체력 회복, ob['내공'] 소모. ob.objs/getItem/호위 전용 efun 없음.

fn cmd(ob, line) {
    send_line(ob, "☞ 호위를 거느리지 않고 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
