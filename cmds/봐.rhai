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
            message += fill_space_euc_kr(38, cell);
            column += 1;
            if column == 2 && i + 1 < len(items) {
                message += "\r\n";
                column = 0;
            }
            i += 1;
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
        let i = 0;
        while i < len(names) {
            let count = counts[i];
            let cell = "\x1b[1;36m·\x1b[0;36m" + names[i]
                + if count == 1 { "\x1b[0;37m" }
                  else { " " + count.to_string() + "개\x1b[0;37m" };
            message += fill_space_euc_kr(22, cell);
            if (i + 1) % 3 == 0 && i + 1 < len(names) { message += "\r\n"; }
            i += 1;
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

fn main(ob, line) {
    if line.trim() == "" {
        let r = view_map_data(ob);
        if r["ok"] {
            for row in split(r["text"], "\r\n") {
                send_line(ob, row);
            }
        } else {
            if r["err"] == "no_position" {
                send_line(ob, "\x1b[1;31m위치 정보가 없습니다.\x1b[0;37m");
            } else if r["err"] == "room_error" {
                send_line(ob, "\x1b[1;31m방 정보를 읽을 수 없습니다.\x1b[0;37m");
            } else if r["err"] == "unknown_room" {
                send_line(ob, "\x1b[1;37m[" + r["zone"] + ":" + int_to_str(r["room"]) + "]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n");
            }
        }
        return;
    }

    let box = find_view_box(ob, line);
    if box["found"] {
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
        send_to_player(target_name, target_msg);
    }
}
