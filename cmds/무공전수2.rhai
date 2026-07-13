// Python cmds/무공전수2.py - 관리자 대상에게 문자열 무공 전수

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let words = python_split_once(trim(line));
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [무공이름] 무공전수");
        return;
    }
    let target = resolve_room_player_name(ob, words[0]);
    if target == "" {
        let view = get_mugong_view(ob, words[0]);
        if view["ok"] { target = view["name"]; }
    }
    let skill = words[1];
    if target == "" {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }
    let result = teach_skill_to_target(ob, target, skill, true);
    if result == "ok" {
        send_line(ob, "☞ 무공이 전수되었습니다.");
    } else {
        send_line(ob, "☞ 그런 대상이 없어요!");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
