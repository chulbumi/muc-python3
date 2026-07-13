// Python cmds/도망.py와 Player.enterRoom(..., mode="도망")의 출력 경로.
// 방/전투 상태와 난수 선택은 efun, 사용자 문구·ANSI는 이 파일이 담당한다.

fn flee_failed(ob) {
    send_line(ob, "☞ 도망 갈려다 잡혔어요. '흑흑~~ T_T'");
}

fn flee_actor_name(ob) {
    let name = get_string(ob, "이름");
    "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name)
}

fn flee_prompt(player_name) {
    for player in get_all_online_players() {
        if player["이름"] == player_name {
            if !player["show_prompt"] {
                return "";
            }
            return "\r\n\x1b[0;37;40m[ " + player["현재체력"].to_string() + "/"
                + player["현재최고체력"].to_string() + ", "
                + player["현재내공"].to_string() + "/"
                + player["현재최고내공"].to_string() + " ] ";
        }
    }
    ""
}

fn send_flee_players(players, own_name, message, with_prompt) {
    for player_name in players {
        if player_name != "" && player_name != own_name {
            let prompt = if with_prompt { flee_prompt(player_name) } else { "" };
            send_raw_to_user(player_name, "\r\n" + message + "\r\n" + prompt);
        }
    }
}

fn cool(ob) {
    reset_runaway(ob);
}

fn cmd(ob, line) {
    if get_int(ob, "act") != 1 {
        send_line(ob, "☞ 무림인은 아무때나 도망가는것이 아니라네");
        return;
    }
    if combat_room_has_attr(ob, "도망금지") {
        flee_failed(ob);
        return;
    }
    if runaway_cooling_down(ob) {
        flee_failed(ob);
        return;
    }

    // Python은 이후 어느 분기에서 실패해도 먼저 1초 재시도 제한을 건다.
    begin_runaway(ob);
    let old_room_players = get_room_players(ob);
    let transition = attempt_flee(ob);
    let status = transition["status"];
    if status == "failed" {
        flee_failed(ob);
        return;
    }
    if status == "pressure" {
        send_line(ob, "강한 무형의 기운이 당신을 압박합니다.");
        flee_failed(ob);
        return;
    }
    if status == "room_full" {
        send_line(ob, "☞ 알 수 없는 무형의 기운이 당신을 가로막습니다. ^_^");
        flee_failed(ob);
        return;
    }
    if status == "evil_forbidden" {
        send_line(ob, "☞ 사파는 출입할 수 없는 곳이라네!");
        flee_failed(ob);
        return;
    }
    if status == "good_forbidden" {
        send_line(ob, "☞ 정파는 출입할 수 없는 곳이라네!");
        flee_failed(ob);
        return;
    }
    if status == "guild_forbidden" {
        send_line(ob, "☞ 그곳은 타 방파의 지역이므로 출입하실 수 없습니다.");
        flee_failed(ob);
        return;
    }
    if status != "ok" {
        // enterRoom 후속 동작을 현재 모델이 정확히 재현할 수 없는 방은
        // 이동 전에 중단한다. Python에 없는 성공/실패 문구는 만들지 않는다.
        return;
    }

    let name = get_string(ob, "이름");
    let name_a = "\x1b[1m" + name + "\x1b[0;37m";
    let actor = flee_actor_name(ob);
    let visible = get_int(ob, "투명상태") != 1;
    if visible {
        let exit_script = transition["exit_script"];
        if exit_script != "" {
            let to_self = post_position_once(return_replace(exit_script, "[공]", "당신"));
            let to_room = post_position_once(return_replace(exit_script, "[공]", name_a));
            send_line(ob, "\r\n" + to_self);
            // exitRoom은 이미 CRLF가 붙은 문자열을 sendRoom에 넘긴다.
            send_flee_players(old_room_players, name, "\r\n" + to_room, true);
        } else {
            send_line(ob, "당신이 신형을 비틀거리며 간신히 도망갑니다. '살리도~~'");
            send_flee_players(
                old_room_players,
                name,
                actor + " 신형을 비틀거리며 간신히 도망갑니다. '살리도~~'",
                true
            );
        }
    }

    send_entry_room_updates(transition);
    let room = view_map_data(ob);
    if room["ok"] {
        send_line(ob, room["text"]);
    }

    if transition["has_entry_events"] {
        let events = run_return_entry_events(ob);
        for event_line in events["lines"] {
            send_line(ob, event_line);
        }
        if events["status"] != "ok" {
            return;
        }
    }

    let new_room_players = get_room_players(ob);
    if visible {
        let entry_script = transition["entry_script"];
        if entry_script != "" {
            let to_room = post_position_once(return_replace(entry_script, "[공]", name_a));
            send_flee_players(new_room_players, name, to_room, true);
        } else {
            send_flee_players(
                new_room_players,
                name,
                actor + " 신형을 비틀거리며 간신히 도망옵니다. '헉헉~~' ",
                true
            );
        }
    }

    if transition["has_hazard"] {
        let damage = transition["hazard_damage"];
        let hazard_message = transition["hazard_message"];
        let to_self = post_position_once(return_replace(hazard_message, "[공]", "당신"));
        let to_room = post_position_once(return_replace(hazard_message, "[공]", name_a));
        let hazard_output = "\r\n" + to_self;
        if get_user_config(ob, "엘피출력") != "1" {
            let vitals = return_vitals(ob);
            hazard_output = "\x1b[0;37;40m[ "
                + return_int_string(vitals["hp"]) + "/"
                + return_int_string(vitals["max_hp"]) + ", "
                + return_int_string(vitals["mp"]) + "/"
                + return_int_string(vitals["max_mp"]) + " ] \r\n"
                + to_self;
        }
        send_line(ob, hazard_output);
        send_flee_players(get_room_players(ob), name, to_room, true);
        if !apply_return_hazard(ob, damage) {
            finish_python_death(ob);
            return;
        }
    }
    let auto_move = transition["auto_move"];
    if auto_move != "" {
        set_temp(ob, "_movement_room_auto_move", auto_move);
    }
    finish_entry_aggression(ob);
}

fn main(ob, line) {
    cmd(ob, line);
}
