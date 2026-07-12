// 몹생성.rhai - 몹 생성 (관리자)
// Python: cmds/몹생성.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if is_empty(line) {
        send_line(ob, "사용법: [몹 이름] 생성");
        return;
    }
    let result = spawn_mob_definition(line);
    if !result["ok"] {
        send_line(ob, "* 생성 실패!!!");
        return;
    }
    send_line(ob, "\x1b[1;32m* [" + result["이름"] + "] 생성 되었습니다.\x1b[0;37m");
}

fn main(ob, line) {
    cmd(ob, line);
}
