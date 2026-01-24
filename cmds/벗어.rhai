// 벗어.rhai - 장비 해제 명령어
// Python: cmds/벗어.py

fn cmd(ob, line) {
    if !require_arg(ob, line, "☞ 사용법: [아이템 이름] 벗어  또는  [모두/전부] 벗어") { return; }
    // ob.objs, inUse, armor, attpower, option 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
