// 모두끝.rhai - 모든 플레이어 종료
// Python: cmds/모두끝.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let self_name = get_attr(ob, "이름");
    for player in get_all_online_players() {
        let target = player["이름"];
        if target != self_name {
            admin_force_command(ob, target, "끝");
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
