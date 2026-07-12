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
