// Python: cmds/채널누구.py

fn cmd(ob, line) {
    let lines = [];
    let line_buf = "";
    let cnt = 0;

    for member in get_adult_channel_members() {
        if member["투명상태"] == 1 || member["이름"] == "" || !member["active"] {
            continue;
        }

        let nick = member["무림별호"];
        let nick_buf = "";
        if nick == "" || nick == () {
            nick_buf = "[\x1b[0;37m무명객\x1b[0;37m]";
        } else if member["성격"] == "정파" {
            nick_buf = "[\x1b[1;32m" + nick + "\x1b[0;37m]";
        } else if member["성격"] == "기인" {
            nick_buf = "[\x1b[1;33m" + nick + "\x1b[0;37m]";
        } else if member["성격"] == "선인" {
            nick_buf = "[\x1b[1;36m" + nick + "\x1b[0;37m]";
        } else {
            nick_buf = "<\x1b[1;31m" + nick + "\x1b[0;37m>";
        }

        line_buf += "  " + fill_space(26, nick_buf) + " " + fill_space(10, member["이름"]);
        cnt += 1;
        if cnt % 3 == 0 {
            lines.push(line_buf);
            line_buf = "";
        }
    }
    if line_buf != "" {
        lines.push(line_buf);
    }
    let message = join(lines, "\r\n");

    send_line(ob, "┌─────────────────────────────────────┐");
    let title = " ◁     무       림       크       래       프       트      １-１     ▷";
    send_line(ob, "│\x1b[7m" + fill_space(74, title) + "\x1b[0;37m│");
    send_line(ob, "└─────────────────────────────────────┘");
    send_line(ob, message);
    send_line(ob, " ──────────────────────────────────────");
    send_line(ob, " ★ 총 " + int_to_str(cnt) + "명의 무림인이 활동하고 있습니다.\r\n");
}

fn main(ob, line) {
    cmd(ob, line);
}
