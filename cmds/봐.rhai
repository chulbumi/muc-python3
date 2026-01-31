// 봐.rhai - 보기/조사 명령어
// arg 없음: view_map_data(ob) efun — 방 이름·설명·출구·몹·바닥 아이템
// arg 있음: find_target(ob, line) → lines send_line, to_target이면 send_to_player

// 관리자용 mob 상세 정보 표시
fn show_mob_admin_info(ob, mob_data) {
    send_line(ob, "│ [레  벨]               │ [상  태]               │");
    send_line(ob, "│ [체  력]               │ [내  공]               │");
    send_line(ob, "│ [맷  집]               │ [민  첩]               │");
    send_line(ob, "│ [  힘  ]               │ [스  킬]               │");
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 타겟 목록
    if len(mob_data["targets"]) > 0 {
        send_line(ob, "★ 목표 대상");
        let mut msg = "";
        let mut cnt = 1;
        let targets = mob_data["targets"];
        for i in 0..len(targets) {
            let target = targets[i];
            msg += " [" + int_to_str(cnt) + "] " + target + "    ";
            if cnt % 3 == 0 {
                send_line(ob, msg);
                msg = "";
            }
            cnt += 1;
        }
        if msg != "" {
            send_line(ob, msg);
        }
        send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

// 관리자용 player 상세 정보 표시
fn show_player_admin_info(ob, player_data) {
    send_line(ob, "│ [레  벨]               │ [상  태]               │");
    send_line(ob, "│ [체  력]               │ [내  공]               │");
    send_line(ob, "│ [맷  집]               │ [민  첩]               │");
    send_line(ob, "│ [  힘  ]               │ [은  전]               │");
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn main(ob, line) {
    if line.trim() == "" {
        let r = view_map_data(ob);
        if r["ok"] { send_line(ob, r["text"]); }
        else {
            if r["err"] == "no_position" { send_line(ob, "\x1b[1;31m위치 정보가 없습니다.\x1b[0;37m"); }
            else if r["err"] == "room_error" { send_line(ob, "\x1b[1;31m방 정보를 읽을 수 없습니다.\x1b[0;37m"); }
            else if r["err"] == "unknown_room" { send_line(ob, "\x1b[1;37m[" + r["zone"] + ":" + int_to_str(r["room"]) + "]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n"); }
        }

        // 관리자 등급 체크 - 추가 정보 표시
        let admin = get_int(ob, "관리자등급");
        if admin >= 1000 {
            // 몹 상세 정보
            let mobs = get_room_mobs_admin(ob);
            if len(mobs) > 0 {
                send_line(ob, "\r\n\x1b[0;33m[관리자] 몹 상세 정보\x1b[0;37m");
                for mob in mobs {
                    send_line(ob, "───────────────────────────────────────");
                    send_line(ob, "│ 몹: " + mob["name"]);
                    show_mob_admin_info(ob, mob);
                }
            }

            // 플레이어 상세 정보
            let players = get_room_players_admin(ob);
            if len(players) > 0 {
                send_line(ob, "\r\n\x1b[0;33m[관리자] 플레이어 상세 정보\x1b[0;37m");
                for player in players {
                    send_line(ob, "───────────────────────────────────────");
                    send_line(ob, "│ 플레이어: " + player["name"]);
                    show_player_admin_info(ob, player);
                }
            }
        }
        return;
    }

    let r = find_target(ob, line);
    if r["err"] == "no_position" { send_line(ob, "위치 정보가 없습니다."); return; }
    if r["err"] == "not_found" { send_line(ob, "\x1b[1;31m☞ 당신의 안광으로는 그런것을 볼수 없다네\x1b[0;37m"); return; }
    for l in r["lines"] {
        send_line(ob, l);
    }
    let tt = r["to_target"];
    if tt["name"] != "" {
        send_to_player(tt["name"], tt["msg"]);
    }

    // 관리자가 mob을 본 경우 상세 정보 추가 표시
    let admin = get_int(ob, "관리자등급");
    if admin >= 1000 {
        // mob인지 확인 (라인에 mob 정보가 있는지)
        let mobs = get_room_mobs_admin(ob);
        for mob in mobs {
            if mob["name"] == line || mob["name"].starts_with(line) {
                send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                send_line(ob, "\x1b[0;33m[관리자] 몹 상세 정보\x1b[0;37m");
                show_mob_admin_info(ob, mob);
                break;
            }
        }
    }
}
