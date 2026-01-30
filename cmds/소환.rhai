// 소환.rhai - 대상 소환
// Python: cmds/소환.py
// Admin level 2000 (efun requirement)

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: [대상] 소환");
        return;
    }

    let result = summon_player(ob, line);
    if result == "" {
        send_line(ob, "☞ '" + line + "'님을 소환했습니다.");
    } else if result == "대상을 찾을 수 없습니다." {
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^");
    } else if result == "이미 같은 위치에 있습니다." {
        send_line(ob, "☞ 같은 곳이에요. ^^");
    } else {
        send_line(ob, "☞ " + result);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
