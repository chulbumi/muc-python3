// Python cmds/몹찾기.py parity. find_mobs returns ordered template data only.

fn cmd(ob, line) {
    if get_int(ob, "관리자등급") < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 운영자 명령: [몹이름]|[몹종류] 몹찾기\r\n ex) 산적 몹찾기 or 6 몹찾기");
        return;
    }
    let results = find_mobs(line);
    for mob in results {
        send_line(ob, "\x1b[33m" + mob["name"].to_string() + "\x1b[37m("
            + mob["index"].to_string() + ") : " + mob["location"].to_string());
    }
    if len(results) == 0 {
        send_line(ob, "☞ 찾으시는 몹이 없습니다.");
    }
}

fn main(ob, line) { cmd(ob, line); }
