// 모두소환.rhai - 모든 플레이어 소환
// Python: cmds/모두소환.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let pos = get_my_position(ob);
    let zone = pos["zone"];
    let room = pos["room"];
    let players = get_all_online_players();
    let i = 0;
    while i < len(players) {
        let target = players[i];
        if target != get_string(ob, "이름") {
            let result = warp_player(ob, target, zone, room);
            if result != "" { send_line(ob, "☞ " + target + " 소환에 실패하였습니다."); }
        }
        i = i + 1;
    }
    send_line(ob, "☞ 같은 방의 플레이어는 제외하고 모두 소환했습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
