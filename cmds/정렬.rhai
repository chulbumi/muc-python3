// 정렬.rhai - Python: cmds/정렬.py
// [보관함] [특성치] → 사용법, 특성치 화이트리스트, 은전 10만 검사. findObjName/is_box/보관함.objs.sort 미지원.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [보관함] [특성치] 정렬");
        return;
    }
    let words = split(line, " ");
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [보관함] [특성치] 정렬");
        return;
    }
    let k = trim(words[1]);
    if k != "힘" && k != "민첩성" && k != "맷집" && k != "명중" && k != "회피" && k != "필살" && k != "운" && k != "방어력" && k != "체력" && k != "내공" && k != "이름" {
        send_line(ob, "☞ 힘|민첩성|맷집|명중|회피|필살|운|방어력|체력|내공|이름 만 가능합니다.");
        return;
    }
    if get_int(ob, "은전") < 100000 {
        send_line(ob, "☞ 은전이 부족해요.");
        return;
    }
    // findObjName(line)→보관함, is_box, obj.objs.sort(특성치), set_value(ob,"은전",은전-100000) (미지원)
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
