// 기연정리.rhai - Python: cmds/기연정리.py
// [기연이름] → ONEITEM.index, owner 파싱. len==1: Player.load/3일/objs 제거는 미지원. len==3: who,where 출력. 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [기연이름] 기연정리");
        return;
    }
    let idx = oneitem_get_index_by_name(trim(line));
    if idx == "" {
        send_line(ob, "☞ 그런 아이템은 없습니다.!");
        return;
    }
    let owner = oneitem_get(idx);
    let words = split(owner, " ");
    if len(words) == 1 {
        send_line(ob, "☞ 오프라인 사용자 인벤·3일·제거는 아직 지원하지 않습니다.");
        return;
    }
    if len(words) == 3 {
        let who = words[0];
        let where = words[1];
        send_line(ob, who + " " + where);
        return;
    }
    send_line(ob, "아무도 소지하고 있지 않습니다.!");
}

fn main(ob, line) {
    cmd(ob, line);
}
