// 말.rhai - 말하기 명령어
// Python: cmds/말.py

// ANSI 색상 코드 변환 함수
fn ANSI(msg, conv) {
    let buf = msg;

    if conv {
        buf = buf.replace("{밝}", "\x1b[1m");
        buf = buf.replace("{어}", "\x1b[0m");
        buf = buf.replace("{검}", "\x1b[30m");
        buf = buf.replace("{빨}", "\x1b[31m");
        buf = buf.replace("{초}", "\x1b[32m");
        buf = buf.replace("{노}", "\x1b[33m");
        buf = buf.replace("{파}", "\x1b[34m");
        buf = buf.replace("{자}", "\x1b[35m");
        buf = buf.replace("{하}", "\x1b[36m");
        buf = buf.replace("{흰}", "\x1b[37m");
        buf = buf.replace("{배검}", "\x1b[40m");
        buf = buf.replace("{배빨}", "\x1b[41m");
        buf = buf.replace("{배초}", "\x1b[42m");
        buf = buf.replace("{배노}", "\x1b[43m");
        buf = buf.replace("{배파}", "\x1b[44m");
        buf = buf.replace("{배자}", "\x1b[45m");
        buf = buf.replace("{배하}", "\x1b[46m");
        buf = buf.replace("{배흰}", "\x1b[47m");
    } else {
        buf = buf.replace("{밝}", "");
        buf = buf.replace("{어}", "");
        buf = buf.replace("{검}", "");
        buf = buf.replace("{빨}", "");
        buf = buf.replace("{초}", "");
        buf = buf.replace("{노}", "");
        buf = buf.replace("{파}", "");
        buf = buf.replace("{자}", "");
        buf = buf.replace("{하}", "");
        buf = buf.replace("{흰}", "");
        buf = buf.replace("{배검}", "");
        buf = buf.replace("{배빨}", "");
        buf = buf.replace("{배초}", "");
        buf = buf.replace("{배노}", "");
        buf = buf.replace("{배파}", "");
        buf = buf.replace("{배자}", "");
        buf = buf.replace("{배하}", "");
        buf = buf.replace("{배흰}", "");
    }

    buf;
}

// 메인 명령어 함수
fn cmd(ob, line) {
    if !require_arg(ob, line) { send_line(ob, "\r\nSay What???"); return; }
    let m1 = ANSI(line, true) + "\x1b[0;40;37m";
    let m2 = ANSI(line, false);
    let name = ob_name(ob);
    let particle = ob_iga(ob);

    send_line(ob, "당신이 말합니다 : '" + m1 + "'");
    let room_msg = name + particle + " 말합니다 : '" + m1 + "'";
    send_room(ob, room_msg);
}

// 엔트리 포인트
fn main(ob, line) {
    cmd(ob, line);
}
