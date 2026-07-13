// 체인지.rhai - 장비 변경
// Python: cmds/체인지.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 체인지");
        return;
    }
    let target = resolve_room_player_name(ob, line);
    if target == "" {
        send_line(ob, "여기에 없나봐요^^");
        return;
    }
    request_change_player(ob, target);
}

fn main(ob, line) {
    cmd(ob, line);
}
