// 올려.rhai - 올려. Python: cmds/올려.py
// [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] ob['특성치']--, ob[line]++, ob[line+'특성치']++ 등.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 올려");
        return;
    }
    let hay = " 힘 민첩성 맷집 명중 회피 필살 운 내공 체력 ";
    if !contains(hay, " " + line + " ") {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 올려");
        return;
    }
    send_line(ob, "☞ 더이상 올릴 수 있는 여유 특성치가 없습니다. (Body 특성치 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
