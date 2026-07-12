// 말.rhai - 말하기 명령어
// Python: cmds/말.py

// ANSI 색상 코드 변환 함수
fn ANSI(msg, conv) {
    let buf = msg;

    if conv {
        buf.replace("{밝}", "\x1b[1m");
        buf.replace("{어}", "\x1b[0m");
        buf.replace("{검}", "\x1b[30m");
        buf.replace("{빨}", "\x1b[31m");
        buf.replace("{초}", "\x1b[32m");
        buf.replace("{노}", "\x1b[33m");
        buf.replace("{파}", "\x1b[34m");
        buf.replace("{자}", "\x1b[35m");
        buf.replace("{하}", "\x1b[36m");
        buf.replace("{흰}", "\x1b[37m");
        buf.replace("{배검}", "\x1b[40m");
        buf.replace("{배빨}", "\x1b[41m");
        buf.replace("{배초}", "\x1b[42m");
        buf.replace("{배노}", "\x1b[43m");
        buf.replace("{배파}", "\x1b[44m");
        buf.replace("{배자}", "\x1b[45m");
        buf.replace("{배하}", "\x1b[46m");
        buf.replace("{배흰}", "\x1b[47m");
    } else {
        buf.replace("{밝}", "");
        buf.replace("{어}", "");
        buf.replace("{검}", "");
        buf.replace("{빨}", "");
        buf.replace("{초}", "");
        buf.replace("{노}", "");
        buf.replace("{파}", "");
        buf.replace("{자}", "");
        buf.replace("{하}", "");
        buf.replace("{흰}", "");
        buf.replace("{배검}", "");
        buf.replace("{배빨}", "");
        buf.replace("{배초}", "");
        buf.replace("{배노}", "");
        buf.replace("{배파}", "");
        buf.replace("{배자}", "");
        buf.replace("{배하}", "");
        buf.replace("{배흰}", "");
    }

    buf;
}

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "\r\nSay What???");
        return;
    }

    let m1 = ANSI(line, true) + "\x1b[0;40;37m";
    let m2 = ANSI(line, false);

    send_line(ob, "당신이 말합니다 : '" + m1 + "'");
    let name = get_body_string(ob, "이름");
    let room_msg = "\x1b[33m" + name + "\x1b[37m" + han_iga(name)
        + " 말합니다 : '" + m1 + "'";
    for player_name in get_room_players(ob) {
        if player_name != name {
            send_to_user(player_name, room_msg);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
