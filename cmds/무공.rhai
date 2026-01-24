// 무공.rhai - 무공 목록 명령어
// Python: cmds/무공.py

fn cmd(ob, line) {
    // ob.env.findObjName, target.skillList, 스킬 정보 등 서버 연동 필요
    send_line(ob, "☞ 아직 준비 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
