// Python `cmds/리젠.py`. Rust returns state; Rhai owns output/prompt text.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let result = regen_room_mobs(ob, line);
    if result["status"] == "empty" {
        send_line(ob, "☞ 리젠될 몹이 없어요!!");
        return;
    }
    if result["status"] == "unknown" {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if result["status"] == "not_corpse" {
        send_line(ob, "☞ 시체만 가능합니다. *^_^*");
        return;
    }
    let first = true;
    let room_raw = "";
    for message in result["messages"] {
        send_line(ob, if first { "\r\n" + message } else { message });
        first = false;
        room_raw += "\r\n" + message;
    }
    let context = get_party_context();
    for player_name in get_room_players(ob) {
        if player_name == get_string(ob, "이름") { continue; }
        let raw = room_raw + "\r\n";
        for person in context["room_players"] {
            if person["name"] == player_name && person["show_prompt"] {
                raw += "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/"
                    + person["max_hp"].to_string() + ", " + person["mp"].to_string()
                    + "/" + person["max_mp"].to_string() + " ] ";
                break;
            }
        }
        send_raw_to_user(player_name, raw);
    }
    // Python prints this confirmation only for the all-corpses branch.
    if line == "" { send_line(ob, "\r\n☞ 리젠되었습니다."); }
}

fn main(ob, line) { cmd(ob, line); }
