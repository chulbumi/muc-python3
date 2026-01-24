// 기연정리.rhai - 기연정리. Python: cmds/기연정리.py
// [기연이름] ONEITEM, Player.load, 3일 체크, objs 에서 제거 등.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 사용법: [기연이름] 기연정리");
            return;
        }
        send_line(ob, "☞ 기연 정리: ONEITEM/Player 연동 후 지원 예정.");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
