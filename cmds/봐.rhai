// 봐.rhai - 보기/조사 명령어

fn view_box_data(ob, box, detailed) {
    let ref = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
    send_line(ob, ref);
    let title = "◁ " + box["주인"].to_string() + "의 " + box["이름"].to_string() + " ▷";
    send_line(ob, "\x1b[1m\x1b[44m\x1b[37m" + fill_space_euc_kr(79, title)
        + "\x1b[0m\x1b[40m\x1b[37m");
    send_line(ob, "───────────────────────────────────────");
    let items = box["items"];
    if len(items) == 0 {
        send_line(ob, "☞ 아무것도 없습니다.");
    } else if detailed {
        let message = "";
        let column = 0;
        let i = 0;
        while i < len(items) {
            let item = items[i];
            let name = if item["oneitem"] {
                "\x1b[1;36m" + strip_ansi(item["name"].to_string()) + "\x1b[0;37m"
            } else {
                item["name"].to_string()
            };
            let option = item["option"].to_string();
            let cell = "[" + fill_space_front_euc_kr(4, (i + 1).to_string()) + "] " + name
                + if option == "" { " " } else { " " + option };
            let padded = fill_space_euc_kr(38, cell);
            if column == 1 && euc_kr_len(padded) > 38 {
                message += "\r\n";
            }
            message += padded;
            if euc_kr_len(cell) > 38 {
                message += "\r\n";
                column = 0;
            } else {
                column += 1;
            }
            if column == 2 {
                message += "\r\n";
                column = 0;
            }
            i += 1;
        }
        if ends_with(message, "\r\n") {
            message = substring(message, 0, len(message) - 2);
        }
        send_line(ob, message);
    } else {
        let names = [];
        let counts = [];
        for item in items {
            let name = item["name"].to_string();
            let found = -1;
            let i = 0;
            while i < len(names) {
                if names[i] == name { found = i; break; }
                i += 1;
            }
            if found < 0 { names.push(name); counts.push(1); }
            else { counts[found] += 1; }
        }
        let message = "";
        let column = 0;
        let i = 0;
        while i < len(names) {
            let count = counts[i];
            let ansi_count = if len(names[i]) != len(strip_ansi(names[i])) {
                "\x1b[0;36m"
            } else {
                ""
            };
            let cell = "\x1b[1;36m·\x1b[0;36m" + names[i]
                + if count == 1 { "\x1b[0;37m" }
                  else { " " + ansi_count + count.to_string() + "개\x1b[0;37m" };
            let padded = fill_space_euc_kr(22, cell);
            if column == 1 && euc_kr_len(padded) > 22 {
                message += "\r\n";
            }
            message += padded;
            if euc_kr_len(cell) > 22 {
                message += "\r\n";
                column = 0;
            } else {
                column += 1;
            }
            if column == 3 {
                message += "\r\n";
                column = 0;
            }
            i += 1;
        }
        if len(names) % 3 == 0 && len(message) >= 2 {
            message = substring(message, 0, len(message) - 2);
        }
        send_line(ob, message);
    }
    send_line(ob, "───────────────────────────────────────");
    let count = len(items);
    let capacity = box["보관수량"].to_string();
    let maximum = box["보관최대수량"].to_string();
    let footer = if box["보관수량"] == box["보관최대수량"] {
        "◆ 수량 (" + count.to_string() + "/" + capacity + ")"
    } else {
        "◆ 수량 (" + count.to_string() + "/" + capacity + ")  ◆ 최대수량 ("
            + maximum + ")  ◆ 확장에 필요한 은전 (" + box["은전"].to_string()
            + "/" + box["보관증가은전"].to_string() + ")"
    };
    send_line(ob, "\x1b[0m\x1b[47m\x1b[30m" + fill_space_euc_kr(79, footer)
        + "\x1b[0m\x1b[40m\x1b[37m");
    send_line(ob, ref);
}

fn view_fixture_data(ob, fixture) {
    let ref = "━━━━━━━━━━━━━━━━━━━━━";
    send_line(ob, ref);
    let name_line = fill_space_euc_kr(42, "◆ 이름 ▷ " + fixture["name"].to_string());
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m" + name_line
        + "\x1b[0m\x1b[37m\x1b[40m");
    let type_line = fill_space_euc_kr(42, "◆ 종류 ▷ " + fixture["종류"].to_string());
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m" + type_line
        + "\x1b[0m\x1b[37m\x1b[40m");
    send_line(ob, "─────────────────────");
    let desc = fixture["설명"];
    if type_of(desc) == "array" {
        if len(desc) == 0 && fixture["설명1"].to_string() != "" {
            send_line(ob, fixture["설명1"].to_string());
        } else {
            for line in desc { send_line(ob, line.to_string()); }
        }
    } else if desc.to_string() != "" {
        for line in split(desc.to_string(), "\r\n") { send_line(ob, line); }
    }
    send_line(ob, ref);
}

fn main(ob, line) {
    if line.trim() == "" {
        // 이동 직후와 같은 room_view 렌더러를 사용해야 간략설명과
        // 나침반제거 설정이 `봐`에도 동일하게 반영된다.
        emit_room_view(ob);
        return;
    }

    let words = line_args(line);
    if line == "호위" {
        guard_view_command(ob, "");
        return;
    }
    if len(words) > 1 && words[1] == "호위" {
        guard_view_command(ob, words[0]);
        return;
    }

    let box = find_view_box(ob, line);
    if box["found"] {
        if get_int(ob, "관리자등급") >= 1000 {
            send_line(ob, "Index : " + box["인덱스"].to_string());
        }
        let detailed = line == "무기고" || line == "화초장" || line == "한옥장" || line == "진열장";
        view_box_data(ob, box, detailed);
        return;
    }

    let r = find_target(ob, line);
    if r["err"] == "no_position" {
        return;
    }
    if r["err"] == "not_found" {
        send_line(ob, "☞ 당신의 안광으로는 그런것을 볼수 없다네");
        return;
    }

    let lines = r["lines"];
    if r["fixture_target"] {
        view_fixture_data(ob, r["fixture"]);
        return;
    }
    for l in lines {
        send_line(ob, l);
    }

    if get_int(ob, "관리자등급") >= 1000 {
        for player in get_room_players_admin(ob) {
            if player["name"].to_string() == line && len(player["targets"]) > 0 {
                let i = 0;
                while i < len(player["targets"]) {
                    send_line(ob, " [" + fill_space_front_euc_kr(2, (i + 1).to_string())
                        + " 목표] " + player["targets"][i].to_string() + " ");
                    i += 1;
                }
                send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            }
        }
    }

    let to_target = r["to_target"];
    let target_name = to_target["name"];
    let target_msg = to_target["msg"];
    if target_name != "" {
        let payload = "\r\n" + target_msg + "\r\n";
        for player in get_room_view_players(ob) {
            if player["name"] == target_name {
                if player["show_prompt"] {
                    payload += "\r\n\x1b[0;37;40m[ " + player["hp"].to_string() + "/"
                        + player["max_hp"].to_string() + ", " + player["mp"].to_string()
                        + "/" + player["max_mp"].to_string() + " ] ";
                }
                break;
            }
        }
        send_raw_to_player(target_name, payload);
    }
}
