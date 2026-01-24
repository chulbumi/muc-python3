// 넣어.rhai - 보관함에 물건 넣기
// Python: cmds/넣어.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 넣어");
        return;
    }
    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 넣어");
        return;
    }
    // ob.env.findObjName, is_box, box.addMoney/objs 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
