// 먹어.rhai - 먹어 명령어
// Python: cmds/먹어.py

fn cmd(ob, line) {
    if !require_arg(ob, line, "☞ 사용법: [아이템 이름] 먹어") { return; }
    // ob.act, findObjInven, 종류('먹는것'), 체력/내공 회복 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
