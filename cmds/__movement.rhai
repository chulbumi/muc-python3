// Python Player.parse_command one-word exit branch.
// This is a private hot-reload handler, not a player command.


fn movement_observer_prompt(player_name) {
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

fn send_room_players_except(players, own_name, excluded, message) {
    for player_name in players {
        if player_name != "" && player_name != own_name && !has_exit(excluded, player_name) {
            send_raw_to_user(player_name, "\r\n" + message + "\r\n"
                + movement_observer_prompt(player_name));
        }
    }
}

fn send_room_players(players, own_name, message) {
    send_room_players_except(players, own_name, [], message);
}

fn actor_name(ob) {
    let name = get_string(ob, "이름");
    "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name)
}

fn run_move(ob, command) {
    let old_room_players = get_room_players(ob);
    let transition = move_through_exit(ob, command);
    let status = transition["status"];
    if status == "not_exit" {
        internal_not_handled();
        return;
    }
    if status == "direction_missing" {
        send_line(ob, "☞ 그쪽 방향으로는 가실 수 없습니다.");
        return;
    }
    if status == "move_where" {
        send_line(ob, "Move where?");
        return;
    }
    if status == "not_movable" {
        send_line(ob, "☞ 지금 이동하기에는 좋은 상황이 아니네요. ^_^");
        return;
    }
    if status == "pressure" {
        send_line(ob, "강한 무형의 기운이 당신을 압박합니다.");
        return;
    }
    if status == "room_full" {
        send_line(ob, "☞ 알 수 없는 무형의 기운이 당신을 가로막습니다. ^_^");
        return;
    }
    if status == "evil_forbidden" {
        send_line(ob, "☞ 사파는 출입할 수 없는 곳이라네!");
        return;
    }
    if status == "good_forbidden" {
        send_line(ob, "☞ 정파는 출입할 수 없는 곳이라네!");
        return;
    }
    if status == "guild_forbidden" {
        send_line(ob, "☞ 그곳은 타 방파의 지역이므로 출입하실 수 없습니다.");
        return;
    }
    if status != "ok" {
        // A Python follow-up branch not represented by the Rust state model
        // was detected before mutation. Do not invent a success/error line.
        return;
    }

    let name = get_string(ob, "이름");
    let name_a = "\x1b[1m" + name + "\x1b[0;37m";
    let actor = actor_name(ob);
    let visible = get_int(ob, "투명상태") != 1;
    let move = transition["move"];
    let followers = transition["followers"];
    if visible {
        let exit_script = transition["exit_script"];
        if exit_script != "" {
            let to_self = post_position_once(return_replace(exit_script, "[공]", "당신"));
            let to_room = post_position_once(return_replace(exit_script, "[공]", name_a));
            send_line(ob, "\r\n" + to_self);
            send_room_players_except(old_room_players, name, followers, "\r\n" + to_room);
        } else if transition["hidden"] {
            send_room_players_except(old_room_players, name, followers, actor + " 갑자기 어디론가 사라집니다.");
        } else {
            send_room_players_except(old_room_players, name, followers, actor + " " + move + "쪽으로 갔습니다.");
        }
    }

    send_entry_room_updates(transition);
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

    if visible {
        let entry_script = transition["entry_script"];
        if entry_script != "" {
            let to_room = post_position_once(return_replace(entry_script, "[공]", name_a));
            send_room_players(get_room_players(ob), name, to_room);
        } else {
            send_room_players(get_room_players(ob), name, actor + " 왔습니다.");
        }
    }

    if transition["has_hazard"] {
        let damage = transition["hazard_damage"];
        let message = transition["hazard_message"];
        let to_self = post_position_once(return_replace(message, "[공]", "당신"));
        let to_room = post_position_once(return_replace(message, "[공]", name_a));
        let output = "\r\n" + to_self;
        if get_user_config(ob, "엘피출력") != "1" {
            let vitals = return_vitals(ob);
            output = "\x1b[0;37;40m[ "
                + return_int_string(vitals["hp"]) + "/"
                + return_int_string(vitals["max_hp"]) + ", "
                + return_int_string(vitals["mp"]) + "/"
                + return_int_string(vitals["max_mp"]) + " ] \r\n"
                + to_self;
        }
        send_line(ob, output);
        send_room_players(get_room_players(ob), name, to_room);
        if !apply_movement_hazard(ob, damage) {
            finish_python_death(ob);
            return;
        }
    }

    finish_entry_aggression(ob);

    // Python Player.enterRoom arms a one-second auto move only after entry
    // events, entry notices, hazards and aggressive-mob handling.  The
    // network layer consumes this state after the command/prompt completes.
    let auto_move = transition["auto_move"];
    if auto_move != "" {
        set_temp(ob, "_movement_room_auto_move", auto_move);
    }
}

fn main(ob, cmdline) {
    let args = movement_internal_args(cmdline);
    if args.len() < 2 {
        internal_not_handled();
        return;
    }
    if args[0] == "__limit" {
        if movement_command_limited(ob, args[1]) {
            send_line(ob, "이곳에서 그 명령을 사용할 수 없습니다.");
        } else {
            internal_not_handled();
        }
        return;
    }
    if args[0] == "__move" {
        run_move(ob, args[1]);
        // Python moveNext pops the route before do_command and emits this
        // after that command returns, even when the movement itself fails.
        if take_after_fight_route_finished(ob) {
            send_line(ob, "☞ 더 이상 이동 할 경로가 없습니다.");
        }
        return;
    }
    if args[0] == "__route_end" {
        if take_after_fight_route_finished(ob) {
            send_line(ob, "☞ 더 이상 이동 할 경로가 없습니다.");
        } else {
            internal_not_handled();
        }
        return;
    }
    if args[0] == "__room_object" {
        let lines = get_room_object_lines(ob, args[1]);
        if lines.len() == 0 {
            internal_not_handled();
        } else {
            send_line(ob, join(lines, "\r\n"));
        }
        return;
    }
    internal_not_handled();
}
