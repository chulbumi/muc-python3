// Python cmds/방파별호.py parity.
fn cmd(ob, line) {
    let guild = get_string(ob, "소속");
    if guild == "" { send_line(ob, "☞ 당신은 소속이 없습니다."); return; }
    if get_string(ob, "직위") != "방주" {
        send_line(ob, "☞ 방파의 방주만이 할 수 있습니다.");
        return;
    }
    let words = line_args(line);
    if len(words) != 2 {
        send_line(ob, "☞ 사용법 : [대상] [무림별호] 방파별호");
        return;
    }
    let target_name = words[0];
    let nickname = words[1];
    let own_name = get_string(ob, "이름");
    let target_guild = "";
    let found = target_name == own_name;
    if found { target_guild = guild; }
    let location = get_my_position(ob);
    let online = get_all_online_players();
    if !found {
        for player in online {
            if player["이름"].to_string() == target_name
                    && player["투명상태"] != 1
                    && player["zone"].to_string() == location["zone"].to_string()
                    && player["room"].to_string() == location["room"].to_string() {
                found = true;
                target_guild = player["소속"].to_string();
                break;
            }
        }
    }
    if !found {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    if target_guild != guild {
        send_line(ob, "☞ 당신의 소속이 아닙니다.");
        return;
    }
    if length(nickname) > 10 {
        send_line(ob, "☞ 사용하시려는 별호가 너무 길어요.");
        return;
    }
    if guild_set_nickname(ob, target_name, nickname) != "ok" {
        send_line(ob, "☞ 이곳에 그런 무림인이 없습니다.");
        return;
    }
    let target_display = if own_name == target_name { "자신" } else { target_name };
    send_line(ob, "당신이 \x1b[1m" + target_display
        + "\x1b[0;37m의 방파별호를 『\x1b[1;32m" + nickname
        + "\x1b[0;37m』" + han_uro(nickname) + " 함을 선포합니다.");

    let actor = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name);
    let announcement = actor + " \x1b[1m" + target_display
        + "\x1b[0;37m의 방파별호를 『\x1b[1;32m" + nickname
        + "\x1b[0;37m』" + han_uro(nickname) + " 함을 선포합니다.";
    send_python_guild_group_except(ob, announcement, own_name);
}
fn main(ob, line) { cmd(ob, line); }
