// 무림별호.rhai - 무림별호 설정 및 개인방 생성
// Python: cmds/무림별호.py

fn cmd(ob, line) {
    let nickname = trim(line);
    let args = line_args(nickname);
    if length(nickname) == 0 || len(args) > 1 {
        send_line(ob, "☞ 사용법: [별호이름] 무림별호");
        return;
    }

    if get_body_string(ob, "무림별호") != "" {
        send_line(ob, "☞ 이미 별호를 만들었어요. ^^");
        return;
    }
    if get_user_event(ob, "무림별호설정") == "" {
        send_line(ob, "☞ 아직은 무림별호를 받을 수 없어요. ^^");
        return;
    }
    if length(nickname) < 3 {
        send_line(ob, "☞ 사용하시려는 별호가 너무 짧아요.");
        return;
    }
    if length(nickname) > 10 {
        send_line(ob, "☞ 사용하시려는 별호가 너무 길어요.");
        return;
    }

    if nickname_exists(nickname) {
        send_line(ob, "☞ 다른 무림인이 사용중인 별호입니다. ^^");
        return;
    }

    // 현재 Rhai 이벤트는 `무림별호=정파|사파`, 레거시는
    // `무림별호 정파|사파` 키를 사용하므로 두 형식을 모두 읽는다.
    let route = get_user_event(ob, "무림별호");
    let is_evil = route == "사파" || get_user_event(ob, "무림별호 사파") != "";
    let faction = if is_evil { "사파" } else { "정파" };

    let name = get_body_string(ob, "이름");
    set_value(ob, "무림별호", nickname);
    set_value(ob, "성격", faction);
    // Python은 NICKNAME.save() 성공 여부를 명령 출력으로 바꾸지 않는다.
    // 동시 설정 경쟁으로 이 시점에 선점된 경우만 중복으로 처리한다.
    if nickname_reserve(nickname, name) == "exists" {
        set_value(ob, "무림별호", "");
        set_value(ob, "성격", "");
        send_line(ob, "☞ 다른 무림인이 사용중인 별호입니다. ^^");
        return;
    }

    del_user_event(ob, "무림별호설정");
    del_user_event(ob, "무림별호");
    del_user_event(ob, "무림별호 사파");
    del_user_event(ob, "무림별호 정파");

    let faction_prefix = if is_evil {
        "\x1b[1m☞ [\x1b[31m사파\x1b[37m] "
    } else {
        "\x1b[1m☞ [\x1b[32m정파\x1b[37m] "
    };
    let actor = "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name);
    let announce = "\x1b[1m" + faction_prefix + actor
        + " \x1b[1m자신의 별호를 『\x1b[33m" + nickname + "\x1b[37m』"
        + han_uro(nickname) + " 칭하기 시작합니다.\x1b[0;37m";

    for target_name in get_online_names() {
        if target_name != "" && target_name != name {
            send_to_user(target_name, announce);
        }
    }
    // Python은 자기 자신에게만 한 줄을 더 비운다.
    send_line(ob, announce + "\r\n");

    // makeHome -> 귀환지맵 설정 -> save -> enterRoom('귀환') 순서를 유지한다.
    let home_error = make_home(ob);
    set_value(ob, "귀환지맵", "사용자맵:" + name);
    body_save(ob);
    if home_error != "" {
        send_line(ob, "☞ 사용자맵 생성에 실패하였습니다.");
        return;
    }

    // Python enterRoom은 전투/휴식 중이면 귀환 이동을 거부한다.
    let act = get_int(ob, "act");
    if act == 1 || act == 4 {
        send_line(ob, "☞ 지금 이동하기에는 좋은 상황이 아니네요. ^_^");
        return;
    }

    let old_room_players = get_room_players(ob);
    send_line(ob, "당신이 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'");
    for target_name in old_room_players {
        if target_name != "" && target_name != name {
            send_to_user(target_name, actor + " 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'");
        }
    }

    let move_error = set_my_position(ob, "사용자맵", name);
    if move_error != "" && move_error != "same_place" {
        send_line(ob, "☞ 사용자맵 생성에 실패하였습니다.");
        return;
    }

    let room = view_map_data(ob);
    if room["ok"] {
        send_line(ob, room["text"]);
    } else {
        send_line(ob, "☞ 사용자맵 생성에 실패하였습니다.");
        return;
    }

    for target_name in get_room_players(ob) {
        if target_name != "" && target_name != name {
            send_to_user(target_name, actor + " 하늘에서 사뿐히 내려 앉습니다. '척~~~'");
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
