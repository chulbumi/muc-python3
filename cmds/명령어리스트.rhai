// 명령어리스트.rhai - 명령어리스트. Python: cmds/명령어리스트.py
// ob.cmdList 순회, level==1000인 명령만.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        let msg = "";
        let c = 0;
        for name in get_python_commands_at_level(1000) {
            msg += pad_start(name.to_string(), 20, " ");
            c += 1;
            if c % 3 == 0 { msg += "\r\n"; }
        }
        send_line(ob, msg);
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
