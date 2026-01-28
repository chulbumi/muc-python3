// 사용자몹소환.rhai - 사용자 생성 몹 소환
// Python: cmds/사용자몹소환.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 사용자몹소환 [몹이름]");
        return;
    }
    send_line(ob, "☞ 사용자 생성 몹 소환 기능은 현재 구현 중입니다.");
    send_line(ob, "   (몹: '" + line + "')");
}

fn main(ob, line) {
    cmd(ob, line);
}
