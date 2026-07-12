// 봐.rhai - 보기/조사 명령어

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

    let r = find_target(ob, line);
    if r["err"] == "no_position" {
        send_line(ob, "위치 정보가 없습니다.");
        return;
    }
    if r["err"] == "not_found" {
        send_line(ob, "\x1b[1;31m☞ 당신의 안광으로는 그런것을 볼수 없다네\x1b[0;37m");
        return;
    }

    let lines = r["lines"];
    for l in lines {
        send_line(ob, l);
    }

    let to_target = r["to_target"];
    let target_name = to_target["name"];
    let target_msg = to_target["msg"];
    if target_name != "" {
        send_to_player(target_name, target_msg);
    }
}
