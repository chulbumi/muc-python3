// 상태보기.rhai - 상태보기 명령어 (관리자)
// Python: cmds/상태보기.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if is_empty(line) {
        send_line(ob, "☞ 운영자 명령: [대상] 상태보기");
        return;
    }

    let target = trim(line);
    let me = get_string(ob, "이름");

    if is_empty(target) || target == "나" || target == me {
        // 자기 자신의 상태 표시
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

    // 다른 플레이어의 상태 조회 (get_player_by_name 사용)
    let player = get_player_by_name(target);
    if player.is_unit() {
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^");
        return;
    }

    // 플레이어 데이터 표시
    let name = get_attr(player, "이름");
    let lv = get_attr(player, "레벨");
    let hp = get_attr(player, "hp");
    let max_hp = get_attr(player, "max_hp");
    let silver = get_attr(player, "은전");
    let guild = get_attr(player, "소속");
    let nickname = get_attr(player, "무림별호");

    send_line(ob, "┏━━━━━━━━━━━━━━━━━━━━━━━━━┑");
    send_line(ob, "│ ▷▶▷▶▷ " + name + "의 현재 상태     ◁◀◁◀◁ │");
    send_line(ob, "┝━━━━━━━━━━━━┯━━━━━━━━━━━━┥");
    send_line(ob, "│ [레  벨]       [" + int_to_str(lv) + "] │ [은  전]        " + int_to_str(silver) + " │");
    send_line(ob, "│ [체  력] " + int_to_str(hp) + "/" + int_to_str(max_hp) + " │ [소  속]      " + guild + " │");
    send_line(ob, "┕━━━━━━━━━━━━━━━━━━━━━━━━━┙");
}

fn main(ob, line) {
    cmd(ob, line);
}
