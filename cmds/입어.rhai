// 입어.rhai - 입어 명령어
// Python: cmds/입어.py

fn cmd(ob, line) {
    if !require_arg(ob, line, "☞ 사용법: [아이템 이름] 입어") { return; }
    // findObjInven, inUse, 장착/방어력 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
