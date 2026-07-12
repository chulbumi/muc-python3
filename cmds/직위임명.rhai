// Python cmds/직위임명.py parity. State mutation is delegated to guild_promote;
// every user-visible byte remains here.

fn cmd(ob, line) {
    if get_string(ob, "직위") != "방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }
    let words = line_args(line);
    if line == "" || len(words) < 2
            || (words[1] != "방주" && words[1] != "부방주"
                && words[1] != "장로" && words[1] != "방파인") {
        send_line(ob, "☞ 사용법 : [대상] [방주|부방주|장로|방파인] 직위임명");
        return;
    }

    let target_name = words[0];
    let new_position = words[1];
    let own_name = get_string(ob, "이름");
    if target_name == own_name {
        send_line(ob, "☞ 자기 자신입니다.");
        return;
    }
    let my_location = get_my_position(ob);
    let target = #{};
    let online = get_all_online_players();
    for player in online {
        if player["이름"].to_string() == target_name
                && player["투명상태"] != 1
                && player["zone"].to_string() == my_location["zone"].to_string()
                && player["room"].to_string() == my_location["room"].to_string() {
            target = player;
            break;
        }
    }
    if len(target) == 0 {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    let guild = get_string(ob, "소속");
    if target["소속"].to_string() != guild {
        send_line(ob, "☞ 당신의 소속이 아닙니다.");
        return;
    }
    let old_position = target["직위"].to_string();
    if old_position == new_position {
        send_line(ob, "☞ 같은 직위입니다.");
        return;
    }
    let status = guild_promote(ob, target_name, old_position, new_position);
    if status == "full" {
        send_line(ob, "☞ 같은 직위의 인원이 너무 많습니다.");
        return;
    }
    if status != "ok" {
        send_line(ob, "☞ 직위 변경에 실패했습니다.");
        return;
    }

    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    let object = "\x1b[1m" + target_name + "\x1b[0;37m" + han_eul(target_name);
    let message = actor + " " + object + " \x1b[1m" + new_position
        + "\x1b[0m" + han_uro(new_position) + " 직위를 임명합니다.";
    send_python_guild_group(ob, message);
}

fn main(ob, line) { cmd(ob, line); }
