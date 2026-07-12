// 기연정리리.rhai - Python: cmds/기연정리리.py
// 기연정리와 유사. len==3(보관): who,where 출력 후 oneitem_destroy. len==1: Player/objs 제거는 미지원. 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
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
    let who = "";
    let where = "";
    if len(words) == 1 {
        who = words[0];
    } else if len(words) == 3 {
        who = words[0];
        where = words[1];
    } else {
        send_line(ob, "아무도 소지하고 있지 않습니다.!");
        return;
    }
    for online in get_connected_player_names() {
        if online == who { send_line(ob, "사용자가 접속중입니다.!" ); return; }
    }
    if where == "" {
        let result = cleanup_offline_oneitem(ob, who, idx);
        if result == "ok" {
            send_line(ob, who + "의 " + trim(line) + han_eul(trim(line)) + " 정리하였습니다.");
            oneitem_destroy(idx);
            return;
        } else if result != "not_found" {
            send_line(ob, result);
            return;
        }
    }
    send_line(ob, who + " " + where);
    oneitem_destroy(idx);
}

fn main(ob, line) {
    cmd(ob, line);
}
