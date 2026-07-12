// Python 외침 계열의 사용자 색상 태그 변환.
fn chat_user_ansi(text, enabled) {
    let value = text;
    let tags = [
        ["{밝}", "\x1b[1m"], ["{어}", "\x1b[0m"],
        ["{검}", "\x1b[30m"], ["{빨}", "\x1b[31m"],
        ["{초}", "\x1b[32m"], ["{노}", "\x1b[33m"],
        ["{파}", "\x1b[34m"], ["{자}", "\x1b[35m"],
        ["{하}", "\x1b[36m"], ["{흰}", "\x1b[37m"],
        ["{배검}", "\x1b[40m"], ["{배빨}", "\x1b[41m"],
        ["{배초}", "\x1b[42m"], ["{배노}", "\x1b[43m"],
        ["{배파}", "\x1b[44m"], ["{배자}", "\x1b[45m"],
        ["{배하}", "\x1b[46m"], ["{배흰}", "\x1b[47m"]
    ];
    for tag in tags {
        value = return_replace(value, tag[0], if enabled { tag[1] } else { "" });
    }
    value
}

// Python Player.sendGroup(line, prompt=True). The command owns all visible
// bytes; Rust only supplies online state and routes send_to_user requests.
fn send_python_guild_group_except(ob, line, excluded) {
    let guild = get_string(ob, "소속");
    let own_name = get_string(ob, "이름");
    let position = get_string(ob, "직위");
    let custom_title = guild_get(guild, position + "명칭");
    let title = if custom_title == "" { position } else { custom_title };
    let group_line = "\x1b[1m《\x1b[36m" + title + "\x1b[37mː\x1b[36m"
        + own_name + "\x1b[37m》\x1b[0;37m " + line;
    for player in get_all_online_players() {
        if player["소속"].to_string() != guild { continue; }
        let config = player["설정상태"].to_string();
        if config_text_is_enabled(config, "방파말거부") { continue; }
        let name = player["이름"].to_string();
        if name == excluded { continue; }
        if name == own_name {
            send_line(ob, group_line);
        } else {
            let prompt = if config_text_is_enabled(config, "엘피출력") {
                ""
            } else {
                "\r\n\x1b[0;37;40m[ " + player["현재체력"].to_string() + "/"
                    + player["최고체력"].to_string() + ", "
                    + player["현재내공"].to_string() + "/"
                    + player["최고내공"].to_string() + " ] "
            };
            send_to_user(name, "\r\n" + group_line + prompt);
        }
    }
}

fn send_python_guild_group(ob, line) {
    send_python_guild_group_except(ob, line, "");
}
