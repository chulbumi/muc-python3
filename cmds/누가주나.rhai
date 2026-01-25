// 누가주나.rhai - 누가주나. Python: cmds/누가주나.py
// [아이템인덱스] → Mob.Mobs 순회, mob['아이템']/['사용아이템']에 해당 인덱스 있는 몹 출력. 관리자 1000.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 운영자 명령: [아이템인덱스] 누가주나");
            return;
        }
        send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
    } else {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
