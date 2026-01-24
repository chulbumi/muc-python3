// 채널누구.rhai - 채널누구. Python: cmds/채널누구.py
// ob.adultCH → get_all_online_players() 동일 형식. 투명상태 제외, 소속 mode 없음.

fn cmd(ob, line) {
    let lines = [];
    let line_buf = "";
    let cnt = 0;
    let arr = get_all_online_players();
    for c in arr {
        if get_int(c, "투명상태") == 1 {
            continue;
        }
        let nick = c["무림별호"];
        let buf = "";
        if nick == "" || nick == () {
            buf = "[\x1b[0;37m무명객\x1b[0;37m]";
        } else {
            let bright = if (c["레벨초기화"] != "" && c["레벨초기화"] != ()) { 0 } else { 1 };
            let b = if bright == 1 { "1" } else { "0" };
            if c["성격"] == "정파" {
                buf = "[\x1b[" + b + ";32m" + nick + "\x1b[0;37m]";
            } else if c["성격"] == "기인" {
                buf = "[\x1b[" + b + ";33m" + nick + "\x1b[0;37m]";
            } else if c["성격"] == "선인" {
                buf = "[\x1b[" + b + ";36m" + nick + "\x1b[0;37m]";
            } else {
                buf = "<\x1b[" + b + ";31m" + nick + "\x1b[0;37m>";
            }
        }
        let name = c["이름"];
        line_buf = line_buf + "  " + fill_space(12, buf) + fill_space(10, name);
        cnt = cnt + 1;
        if cnt % 3 == 0 {
            lines.push(line_buf);
            line_buf = "";
        }
    }
    if line_buf != "" {
        lines.push(line_buf);
    }
    let msg = join(lines, "\r\n");
    send_line(ob, "┌─────────────────────────────────────┐");
    send_line(ob, "│\x1b[7m ◁     무       림       크       래       프       트      １-１      ▷ \x1b[0;37m│");
    send_line(ob, "└─────────────────────────────────────┘");
    send_line(ob, msg);
    send_line(ob, " ──────────────────────────────────────");
    let cnt_s = int_to_str(cnt);
    send_line(ob, " ★ 총 " + cnt_s + "명의 무림인이 활동하고 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
