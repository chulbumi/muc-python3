// 몹제작.rhai - 사용자 정의 몹 생성
// Python: cmds/몹제작.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 몹제작 [이름] [체력] [공격력]");
        return;
    }
    send_line(ob, "☞ 사용자 정의 몹 생성 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
