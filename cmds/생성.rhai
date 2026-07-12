// Python `cmds/생성.py`. Rust owns item cloning and ONEITEM state only.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if line == "" { send_line(ob, "사용법: [아이템 이름] [갯수] 생성"); return; }
    let args = line_args(line);
    let count = if len(args) == 1 { 1 } else { to_int(args[1]) };
    let result = admin_create_items(ob, args[0], count);
    if result["status"] == "missing" { send_line(ob, "* 생성 실패!!!"); return; }
    if result["status"] == "one_exists" {
        send_line(ob, "[단일아이템] " + result["name"] + han_iga(result["particle_source"]) + " 이미 생성되어 있습니다.");
        return;
    }
    send_line(ob, "\x1b[1;32m* [" + result["name"] + "] 생성 되었습니다.\x1b[0;37m");
}

fn main(ob, line) { cmd(ob, line); }
