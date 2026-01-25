// 꺼내.rhai - 보관함에서 물건 꺼내기
// Python: cmds/꺼내.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 꺼내");
        return;
    }
    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [보관함이름] [물품] 꺼내");
        return;
    }
    // ob.env.findObjName, box.objs, ob.insert, 무게/갯수 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
