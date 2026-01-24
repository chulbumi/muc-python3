// 정렬.rhai - 정렬. Python: cmds/정렬.py
// [보관함] [특성치] findObjName, is_box, obj.objs.sort. 은전 10만.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [보관함] [특성치] 정렬");
        return;
    }
    let w = split(line, " ");
    if len(w) < 2 {
        send_line(ob, "☞ 사용법: [보관함] [특성치] 정렬");
        return;
    }
    send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네 (findObjName/is_box 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
