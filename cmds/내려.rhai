// 내려.rhai - 내려. Python: cmds/내려.py
// [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] — 특성치 1 하락, ob[line+'특성치']--, ob['특성치']++, ob[line]-- 등.

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려");
        return;
    }
    let hay = " 힘 민첩성 맷집 명중 회피 필살 운 내공 체력 ";
    if !contains(hay, " " + line + " ") {
        send_line(ob, "☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려");
        return;
    }
    send_line(ob, "☞ [특성치 내려] Body 특성치 연동 예정.");
}

fn main(ob, line) {
    cmd(ob, line);
}
