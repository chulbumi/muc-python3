// 비교.rhai - 비교 명령어 (아이템 정보 비교)
// Python: cmds/비교.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [아이템1] [아이템2] 비교");
        return;
    }
    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [아이템1] [아이템2] 비교");
        return;
    }
    let w1 = trim(words[0]);
    let w2 = trim(words[1]);
    let d1 = get_item_data(w1);
    let d2 = get_item_data(w2);
    if type_of(d1) != "map" {
        send_line(ob, "☞ '" + w1 + "' 아이템 정보를 찾을 수 없습니다.");
        return;
    }
    if type_of(d2) != "map" {
        send_line(ob, "☞ '" + w2 + "' 아이템 정보를 찾을 수 없습니다.");
        return;
    }
    let n1 = d1["이름"];
    let n2 = d2["이름"];
    let a1 = d1["공격력"];
    let a2 = d2["공격력"];
    let d1v = d1["방어력"];
    let d2v = d2["방어력"];
    send_line(ob, "■ " + w1 + " : " + n1 + " (공격 " + a1 + " / 방어 " + d1v + ")");
    send_line(ob, "■ " + w2 + " : " + n2 + " (공격 " + a2 + " / 방어 " + d2v + ")");
}

fn main(ob, line) {
    cmd(ob, line);
}
