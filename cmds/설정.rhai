// Python `cmds/설정.py`: 설정 목록 표시 및 단일 설정 토글.

fn config_names() {
    [
        "자동습득", "비교거부", "접촉거부", "동행거부", "전음거부",
        "외침거부", "방파말거부", "간략설명", "엘피출력", "나침반제거",
        "운영자안시거부", "사용자안시거부", "입출입메세지거부",
        "타인전투출력거부", "자동무공시전", "순위거부", "수련모드",
        "잡담시간보기", "자동채널입장"
    ]
}

fn cmd(ob, line) {
    let configs = config_names();
    if line == "" {
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        send_line(ob, "\x1b[47m\x1b[30m◁               설      정      상      태               ▷\x1b[40m\x1b[37m");
        send_line(ob, "──────────────────────────────");
        let row = "";
        let column = 0;
        for config in configs {
            row += "\x1b[1m\x1b[40m\x1b[32m·\x1b[0m\x1b[40m\x1b[37m "
                + fill_space_euc_kr(17, config);
            if get_user_config(ob, config) == "1" {
                row += "[\x1b[1m설  정\x1b[0;37m]    ";
            } else {
                row += "[비설정]    ";
            }
            column += 1;
            if column % 2 == 0 {
                send_line(ob, row);
                row = "";
            }
        }
        if row != "" { send_line(ob, row); }
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        return;
    }
    if !configs.contains(line) {
        send_line(ob, "☞ 그런 설정은 없어요. ^^");
        return;
    }
    let enabled = toggle_user_config_python(ob, line);
    let state = if enabled { "\x1b[1m[설정]\x1b[0;37m" } else { "\x1b[1m[비설정]\x1b[0;37m" };
    send_line(ob, "☞ " + line + han_eul(line) + " " + state + " 하였습니다.");
}

fn main(ob, line) { cmd(ob, line); }
