// 오브젝트저장.rhai - 오브젝트 저장
// Python: cmds/오브젝트저장.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    send_line(ob, "☞ 오브젝트 저장 기능은 현재 구현 중입니다.");
    send_line(ob, "   (월드 상태의 모든 오브젝트를 파일로 저장)");
}

fn main(ob, line) {
    cmd(ob, line);
}
