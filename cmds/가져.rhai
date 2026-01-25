// 가져.rhai - 바닥에서 아이템 줍기 (Python: cmds/가져.py)
fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [아이템 이름] 주워"); return; }
    let args = line_args(line);
    let count = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    let count = if count < 1 { 1 } else { if count > 100 { 50 } else { count } };
    let name = args[0];
    let picked = item_get(ob, name, count);
    if picked == 0 {
        send_line(ob, "☞ 강호에 그런 물건은 존재하지 않는다네");
        return;
    }
    send_line(ob, format_item_action_self(name, "집어서 품속에 갈무리 합니다.", picked));
}

fn main(ob, line) {
    cmd(ob, line);
}
