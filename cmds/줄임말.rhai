// 줄임말.rhai - Python cmds/줄임말.py의 사용자 줄임말 관리 명령.

fn cmd(ob, line) {
    if is_empty(line) {
        if alias_count(ob) == 0 {
            send_line(ob, "☞ 줄임말이 설정되어 있지 않아요. ^^");
            return;
        }

        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        send_line(ob, "\x1b[47m\x1b[30m◁ 줄임말 ▷                                                                  \x1b[40m\x1b[37m");
        send_line(ob, "───────────────────────────────────────");
        for key in alias_keys(ob) {
            send_line(ob, "[" + key + "] " + alias_get(ob, key));
        }
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        return;
    }

    let words = line_args(line);
    let key = words[0];

    // 인수가 하나면 줄임말 삭제.
    if len(words) == 1 {
        if alias_del(ob, key) {
            send_line(ob, "☞ 줄임말을 제거하였어요. ^^");
        } else {
            send_line(ob, "☞ 줄임말이 설정되어 있지 않아요. ^^");
        }
        return;
    }

    let pair = parse_two_args(trim(line));
    let data = trim(pair[1]);
    let commands = split(data, ";");
    for word in commands {
        if word == key || alias_has(ob, word) {
            send_line(ob, "☞ 중첩된 줄임말은 사용할 수 없어요. ^^");
            return;
        }
    }

    if alias_count(ob) >= 100 {
        send_line(ob, "☞ 줄임말이 너무 많아요. ^^");
        return;
    }

    if alias_set(ob, key, data) {
        send_line(ob, "☞ 줄임말을 설정하였어요. ^^");
    } else {
        send_line(ob, "☞ 이미 설정되어 있는 줄임말입니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
