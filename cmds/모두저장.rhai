// 모두저장.rhai - 모든 플레이어 저장
// Python: cmds/모두저장.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    send_line(ob, "☞ 모든 플레이어 저장 기능은 현재 구현 중입니다.");
    send_line(ob, "   (Client.players 접근 및 ply.save() 호출 필요)");
}

fn main(ob, line) {
    cmd(ob, line);
}
