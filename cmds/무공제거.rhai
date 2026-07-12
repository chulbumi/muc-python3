// Python cmds/무공제거.py - 관리자 대상의 무공 제거

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let words = line_args(line);
    if line == "" || len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [무공이름] 무공제거");
        return;
    }
    let target = words[0].to_string();
    let skill = words[1].to_string();
    if !room_player_exists(target) && get_string(ob, "이름") != target {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }
    if remove_skill_from_player(ob, target, skill) {
        send_line(ob, "☞ 무공이 제거되었습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
