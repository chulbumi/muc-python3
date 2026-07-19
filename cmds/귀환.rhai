// 귀환.rhai - 귀환지로 이동
// Python: cmds/귀환.py, objs/player.py enterRoom(..., "귀환", "귀환")

fn return_observer_prompt(player_name) {
    for player in get_all_online_players() {
        if player["이름"] == player_name {
            if !player["show_prompt"] { return ""; }
            return "\r\n\x1b[0;37;40m[ " + player["현재체력"].to_string() + "/"
                + player["현재최고체력"].to_string() + ", "
                + player["현재내공"].to_string() + "/"
                + player["현재최고내공"].to_string() + " ] ";
        }
    }
    ""
}

fn cmd(ob, line) {
    // 이동 전 방 대상은 이동을 적용하기 전에만 알 수 있다.
    let old_room_players = get_room_players(ob);
    let transition = return_home(ob);
    let result = transition["status"];

    if result == "return_forbidden" {
        send_line(ob, "☞ 이곳에선 귀환하실 수 없어요. ^^");
        return;
    }
    if result == "not_movable" {
        send_line(ob, "☞ 지금은 귀환할 상황이 아니에요. ^^");
        return;
    }
    if result == "missing_room" {
        send_line(ob, "귀환지맵이 없습니다. 관리자에게 연락하세요.");
        return;
    }
    if result == "same_room" {
        send_line(ob, "☞ 같은 자리에요. ^^");
        return;
    }
    if result == "pressure" {
        send_line(ob, "강한 무형의 기운이 당신을 압박합니다.");
        return;
    }
    if result == "room_full" {
        send_line(ob, "☞ 알 수 없는 무형의 기운이 당신을 가로막습니다. ^_^");
        return;
    }
    if result == "evil_forbidden" {
        send_line(ob, "☞ 사파는 출입할 수 없는 곳이라네!");
        return;
    }
    if result == "good_forbidden" {
        send_line(ob, "☞ 정파는 출입할 수 없는 곳이라네!");
        return;
    }
    if result == "guild_forbidden" {
        send_line(ob, "☞ 그곳은 타 방파의 지역이므로 출입하실 수 없습니다.");
        return;
    }
    if result != "ok" {
        // Python의 후속 동작을 현재 엔진이 정확히 재현할 수 없는 방은
        // return_home이 이동 전에 중단한다. 원본에 없는 오류 문구는 만들지 않는다.
        return;
    }

    let name = get_body_string(ob, "이름");
    let name_a = "\x1b[1m" + name + "\x1b[0;37m";
    let actor = name_a + han_iga(name);
    let visible = get_int(ob, "투명상태") != 1;
    if visible {
        let exit_script = transition["exit_script"];
        if exit_script != "" {
            let to_self = post_position_once(return_replace(exit_script, "[공]", "당신"));
            let to_room = post_position_once(return_replace(exit_script, "[공]", name_a));
            send_line(ob, "\r\n" + to_self);
            for player_name in old_room_players {
                if player_name != "" && player_name != name {
                    // Python exitRoom의 사용자 이동스크립은 sendRoom에 이미
                    // CRLF가 붙은 문자열을 넘겨 방 수신자 앞에 두 줄을 둔다.
                    send_raw_to_user(player_name, "\r\n\r\n" + to_room + "\r\n"
                        + return_observer_prompt(player_name));
                }
            }
        } else {
            send_line(ob, "당신이 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'");
            for player_name in old_room_players {
                if player_name != "" && player_name != name {
                    send_raw_to_user(player_name, "\r\n" + actor
                        + " 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'\r\n"
                        + return_observer_prompt(player_name));
                }
            }
        }
    }

    send_entry_room_updates(transition);
    // Python enterRoom()은 이동 방식과 관계없이 같은 viewMapData()를
    // 호출한다. 일반 이동/봐와 공통 렌더러를 사용해야 간략설명과
    // 나침반제거 설정도 귀환 직후 동일하게 적용된다.
    emit_room_view(ob);

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
            for player_name in new_room_players {
                if player_name != "" && player_name != name {
                    send_raw_to_user(player_name, "\r\n" + to_room + "\r\n"
                        + return_observer_prompt(player_name));
                }
            }
        } else {
            for player_name in new_room_players {
                if player_name != "" && player_name != name {
                    send_raw_to_user(player_name, "\r\n" + actor
                        + " 하늘에서 사뿐히 내려 앉습니다. '척~~~'\r\n"
                        + return_observer_prompt(player_name));
                }
            }
        }
    }

    let damage = transition["hazard_damage"];
    if transition["has_hazard"] {
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
        for player_name in get_room_players(ob) {
            if player_name != "" && player_name != name {
                send_raw_to_user(player_name, "\r\n" + to_room + "\r\n"
                    + return_observer_prompt(player_name));
            }
        }
        if !apply_return_hazard(ob, damage) {
            finish_python_death(ob);
            return;
        }
    }
    finish_entry_aggression(ob);
}

fn main(ob, line) {
    cmd(ob, line);
}
