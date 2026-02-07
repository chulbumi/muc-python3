// 버려.rhai - 아이템 버리기 (Python: cmds/버려.py)
fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [아이템 이름] 버려"); return; }
    if len(trim(line)) >= 2 && substring(trim(line), 0, 2) == "은전" {
        send_line(ob, "☞ 은전은 버릴 수 없어요. ^^");
        return;
    }
    let t = trim(line);
    if t == "모두" || t == "전부" {
        send_line(ob, "☞ 모두/전부 버리기는 아직 지원하지 않습니다.");
        return;
    }
    let args = line_args(line);
    let count = if len(args) >= 2 { to_int(args[1]) } else { 1 };
    let count = if count < 1 { 1 } else { if count > 50 { 50 } else { count } };
    let parsed = parse_order_name(args[0]);
    let order = parsed[0];
    let name = parsed[1];
    let drop_count = if order != 1 { 1 } else { count };
    let dropped = item_drop(ob, name, order, drop_count);
    if dropped == 0 {
        send_line(ob, "☞ 그런 아이템이 소지품에 없어요.");
        return;
    }
    send_line(ob, format_item_action_self(name, "버립니다.", dropped));
}

fn main(ob, line) {
    cmd(ob, line);
}
