// Python cmds/모두소환.py parity. Each ACTIVE Player executes its own
// target-side enterRoom request; already co-located players report separately.
fn cmd(ob, line) {
    if get_int(ob, "관리자등급") < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    for player in get_all_online_players() {
        let result = summon_player(ob, player["이름"].to_string());
        if result == "이미 같은 위치에 있습니다." {
            send_line(ob, "☞ 같은 곳이에요. ^^");
        } else if result != "" {
            send_line(ob, "☞ 공간이동에 실패하였습니다.");
        }
    }
}
fn main(ob, line) { cmd(ob, line); }
