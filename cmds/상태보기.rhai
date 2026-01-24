// 상태보기.rhai - 상태보기 명령어 (관리자)
// Python: cmds/상태보기.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { return; }
    if !require_arg(ob, line, "☞ 운영자 명령: [대상] 상태보기") { return; }
    // ob.env.findObjName, obj 레벨/나이/체력/성격/성별/힘 등 서버 연동 필요
    send_line(ob, "☞ 아직 준비 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
