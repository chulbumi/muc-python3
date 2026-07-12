// 오브젝트저장.rhai - 오브젝트 저장
// Python: cmds/오브젝트저장.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    send_line(ob, save_object_python(ob, line));
}

fn main(ob, line) {
    cmd(ob, line);
}
