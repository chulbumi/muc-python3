// 특정방파초기화.rhai - 특정 방파 초기화 (관리자)
// Python: cmds/특정방파초기화.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "초기화할 방파를 입력하세요.");
        return;
    }
    if !guild_has(line) {
        send_line(ob, "* 그런 방파가 없습니다.");
        return;
    }
    if guild_reset(line) {
        send_line(ob, "* 방파가 초기화되었습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
