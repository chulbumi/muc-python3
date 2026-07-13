// Python cmds/무공제거.py - 관리자 대상의 무공 제거

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let words = python_split_once(trim(line));
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [무공이름] 무공제거");
        return;
    }
    let skill = words[1];
    let target = resolve_room_player_name(ob, words[0]);
    if target == "" {
        let view = get_mugong_view(ob, words[0]);
        if !view["ok"] {
            send_line(ob, "☞ 그런 대상이 없어요!");
            return;
        }
        target = view["name"];
    }
    if remove_active_skill_from_target(ob, target, skill) {
        send_line(ob, "☞ 무공이 제거되었습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
