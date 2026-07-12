// 정렬.rhai - 보관함 아이템 정렬
// Python: cmds/정렬.py

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
    let result = box_sort_python(ob, line);
    if result == "usage" { send_line(ob, "☞ 사용법: [보관함] [특성치] 정렬"); return; }
    if result == "invalid" { send_line(ob, "☞ 힘|민첩성|맷집|명중|회피|필살|운|방어력|체력|내공|이름 만 가능합니다."); return; }
    if result == "money" { send_line(ob, "☞ 은전이 부족해요."); return; }
    if result != "ok" { send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네"); return; }
    send_line(ob, "☞ 정렬되었습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
