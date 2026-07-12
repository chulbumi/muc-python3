// 입문신청.rhai - 방파 입문 신청
// Python: cmds/입문신청.py

fn cmd(ob, line) {
    if get_string(ob, "소속") != "" {
        send_line(ob, "☞ 방파에 입문 신청을 할 수 없습니다.");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법 : [방주이름] 입문신청");
        return;
    }
    let target = line;
    let present = false;
    for name in get_room_players(ob) { if name == target { present = true; break; } }
    if !present || target == get_string(ob, "이름") {
        send_line(ob, if target == get_string(ob, "이름") { "☞ 자기 자신입니다." } else { "☞ 이곳에 그런 무림인이 없습니다." });
        return;
    }
    let position = get_room_player_attr(ob, target, "직위");
    if position.is_unit() || (position != "방주" && position != "부방주") {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }
    let target_personality = get_room_player_attr(ob, target, "성격").to_string();
    let mine = get_string(ob, "성격");
    let existing = get_string(ob, "기존성격");
    let allowed = if target_personality == "기인" {
        mine == "기인" || mine == "선인" || mine == existing
    } else if target_personality == "선인" {
        mine != ""
    } else { mine == "기인" || mine == "선인" || target_personality == mine };
    if !allowed { send_line(ob, "☞ 방파에 입문 신청을 할 수 없습니다."); return; }
    let prior = get_room_player_attr(ob, target, "입문신청자");
    if prior.to_string().contains(get_string(ob, "이름")) {
        send_line(ob, "☞ 이미 입문 신청을 하였습니다.");
        return;
    }
    if !set_room_player_attr(ob, target, "입문신청자", get_string(ob, "이름")) {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    send_line(ob, "당신이 " + target + "의 방파에 입문을 신청합니다.");
    send_to_player(target, "\r\n" + han_iga(get_string(ob, "이름")) + " 당신의 방파에 입문을 신청합니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
