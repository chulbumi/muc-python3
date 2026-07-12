// Python cmds/무공전수2.py - 관리자 대상에게 문자열 무공 전수

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let words = line_args(line);
    if line == "" || len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [무공이름] 무공전수");
        return;
    }
    let target = words[0].to_string();
    let skill = words[1].to_string();
    if get_string(ob, "이름") == target {
        send_line(ob, "☞ 자기 자신에게 무공을 전수할 수 없습니다.");
        return;
    }
    if !room_player_exists(target) {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }
    let result = teach_skill_to_player(ob, target, skill);
    if result == "ok" {
        send_line(ob, "☞ 무공이 전수되었습니다.");
    } else if result == "self" {
        send_line(ob, "☞ 자기 자신에게 무공을 전수할 수 없습니다.");
    } else {
        send_line(ob, "☞ 그런 대상이 없어요!");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
