// 모두저장.rhai - 모든 플레이어 저장
// Python: cmds/모두저장.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    request_save_all(ob);
}

fn main(ob, line) {
    cmd(ob, line);
}
