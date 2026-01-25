// 상태보기.rhai - 상태보기 명령어 (관리자)
// Python: cmds/상태보기.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { send_line(ob, "☞ 관리자만 사용할 수 있어요."); return; }
    let target = trim(line);
    let me = get_string(ob, "이름");
    if is_empty(target) || target == "나" || target == me {
        let lv = get_body_int(ob, "레벨");
        let hp = get_int(ob, "hp");
        let max_hp = get_int(ob, "max_hp");
        let mp = get_int(ob, "mp");
        let max_mp = get_int(ob, "max_mp");
        let age = get_body_int(ob, "나이");
        let per = get_string(ob, "성격");
        let armor = get_armor(ob);
        let att = get_att_power(ob);
        send_line(ob, "◆ 레벨 " + int_to_str(lv) + " | 나이 " + int_to_str(age) + " | " + per);
        send_line(ob, "◆ 체력 " + int_to_str(hp) + "/" + int_to_str(max_hp) + " | 내력 " + int_to_str(mp) + "/" + int_to_str(max_mp));
        send_line(ob, "◆ 방어 " + int_to_str(armor) + " | 공격 " + int_to_str(att));
        let eq = get_equipped(ob);
        if len(eq) > 0 {
            send_line(ob, "◆ 장비: " + join(eq, ", "));
        }
        return;
    }
    send_line(ob, "☞ 다른 플레이어의 상태는 현재 조회할 수 없습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
