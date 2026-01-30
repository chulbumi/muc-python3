// 직위임명.rhai - 방파 직위 임명
// Python: cmds/직위임명.py
// Uses: guild_promote(ob, member_name, position)

fn cmd(ob, line) {
    let my_position = get_attr(ob, "직위");
    if my_position != "방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 사용법 : [대상] [방주|부방주|장로|제자] 직위임명");
        return;
    }

    let args = line_args(line);
    if len(args) < 2 {
        send_line(ob, "☞ 사용법 : [대상] [방주|부방주|장로|제자] 직위임명");
        return;
    }

    let target = args[0];
    let new_position = args[1];

    // 유효한 직위 확인
    let valid_positions = ["방주", "부방주", "장로", "제자"];
    let is_valid = false;
    let i = 0;
    while i < len(valid_positions) {
        if valid_positions[i] == new_position {
            is_valid = true;
            break;
        }
        i = i + 1;
    }

    if !is_valid {
        send_line(ob, "☞ 사용법 : [대상] [방주|부방주|장로|제자] 직위임명");
        return;
    }

    let my_name = get_attr(ob, "이름");
    if my_name == target {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }

    // 현재 방에 있는 플레이어 목록에서 대상 찾기
    let players = get_room_players(ob);
    let target_found = false;
    let j = 0;
    while j < len(players) {
        if players[j] == target {
            target_found = true;
            break;
        }
        j = j + 1;
    }

    if !target_found {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }

    // guild_promote efun 호출
    let result = guild_promote(ob, target, new_position);
    if result != "" {
        send_line(ob, "☞ " + result);
        return;
    }

    // 성공 메시지
    send_line(ob, "당신이 " + target + "을 " + new_position + " 직위를 임명합니다.");
    // 방파 전체에게 전송 (send_broadcast_to_guild 사용)
    let msg = get_attr(ob, "이름") + " 님이 " + target + "을 " + new_position + " 직위를 임명합니다.";
    send_broadcast_to_guild(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
