// 무공전수.rhai - 대상에게 무공 전수 (관리자 lv 2000)
// Python: cmds/무공전수.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    let words = python_split_once(line);
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [대상] [무공이름] 무공전수");
        return;
    }

    let target_name = resolve_room_player_name(ob, words[0]);
    if target_name == "" {
        let view = get_mugong_view(ob, words[0]);
        if view["ok"] { target_name = view["name"]; }
    }
    if target_name == "" {
        send_line(ob, "☞ 그런 대상이 없어요!");
        return;
    }
    let mugong_name = resolve_skill_definition(words[1]);

    // 무공 데이터 확인
    if mugong_name == "" {
        send_line(ob, "☞ 그런 무공을 습득한 적이 없습니다.");
        return;
    }

    let result = teach_skill_to_target(ob, target_name, mugong_name, false);
    if result == "not_found" {
        send_line(ob, "☞ 그런 대상이 없어요!");
    } else if result == "duplicate" {
        send_line(ob, "☞ 이미 무공을 익히셨는걸요");
    } else if result == "ok" {
        send_line(ob, "☞ 무공이 전수되었습니다.");
    } else {
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
