// 소환.rhai - 대상 소환
// Python: cmds/소환.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 소환 [대상이름]");
        return;
    }
    send_line(ob, "☞ '" + line + "'님을 소환하는 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
