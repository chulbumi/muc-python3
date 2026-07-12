// 속성.rhai - 아이템 속성 확인
// Python: cmds/속성.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let target = line;
    let result = get_obj_attrs(ob, target);
    if !result["found"] {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }
    let msg = "";
    for item in result["attrs"] {
        msg += "#" + item["key"] + "\r\n" + item["value"] + "\r\n\r\n";
    }
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
