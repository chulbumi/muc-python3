// Python cmds/시전.py의 실제(주석 밖) 실행 경로를 이관한다.
// 출력 문구, ANSI, 전투 스크립트 치환은 이 hot-reload 가능한 파일에만 둔다.

fn array_has(values, wanted) {
    for value in values {
        if value.to_string() == wanted {
            return true;
        }
    }
    false
}

fn value_lines(value) {
    if value == () {
        return "";
    }
    if type_of(value) == "array" {
        let lines = [];
        for line in value {
            lines.push(line.to_string());
        }
        return join(lines, "\r\n");
    }
    value.to_string()
}

fn skill_attrs(skill) {
    let value = skill["속성"];
    if value == () || value == "" {
        return [];
    }
    if type_of(value) == "array" {
        return value;
    }
    [value]
}

fn has_skill_attr(skill, prefix) {
    for value in skill_attrs(skill) {
        if starts_with(value.to_string(), prefix) {
            return true;
        }
    }
    false
}

fn post_three(line) {
    let result = post_position_once(line);
    result = post_position_once(result);
    result = post_position_once(result);
    result
}

fn actor_name_a(ob) {
    "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m"
}

fn target_name_a(target) {
    if target["kind"] == "player" {
        return "\x1b[1m" + target["name"] + "\x1b[0;37m";
    }
    "\x1b[33m" + target["name"] + "\x1b[37m"
}

fn weapon_name_a(ob) {
    let weapon = get_cast_weapon(ob);
    if weapon["fist"] {
        return "\x1b[36m주먹\x1b[37m";
    }
    if weapon["ansi"] != "" {
        return weapon["ansi"] + weapon["name"] + "\x1b[0;37m";
    }
    "\x1b[0;36m" + weapon["name"] + "\x1b[37m"
}

// Python Body.makeFightScript: 목록은 CRLF로 합친 뒤 세 출력에 같은 순서로
// [공]/[방]/[무]를 치환하고 postPosition1을 정확히 세 번 적용한다.
fn make_fight_script(ob, script_value, target) {
    let line = value_lines(script_value);
    let target_a = target_name_a(target);
    let weapon_a = weapon_name_a(ob);

    let self_line = cast_replace(line, "[공]", "당신");
    self_line = cast_replace(self_line, "[방]", target_a);
    self_line = cast_replace(self_line, "[무]", weapon_a);
    self_line = post_three(self_line);

    let target_line = cast_replace(line, "[공]", actor_name_a(ob));
    target_line = cast_replace(target_line, "[방]", "당신");
    target_line = cast_replace(target_line, "[무]", weapon_a);
    target_line = post_three(target_line);

    let room_line = cast_replace(line, "[공]", actor_name_a(ob));
    room_line = cast_replace(room_line, "[방]", target_a);
    room_line = cast_replace(room_line, "[무]", weapon_a);
    room_line = post_three(room_line);

    [self_line, target_line, room_line]
}

// sendRoom/sendFightScriptRoom은 전 접속자가 아니라 현재 방 사용자만 순회한다.
fn send_cast_room(ob, message, excluded_name, reject_filter) {
    let my_name = get_string(ob, "이름");
    for player in get_cast_room_players(ob) {
        let name = player["name"];
        if name == my_name || name == excluded_name {
            continue;
        }
        if reject_filter && player["reject_fight"] {
            continue;
        }
        send_to_user(name, message);
    }
}

fn target_is_available(ob, target) {
    if target == () || (target["kind"] != "player" && target["kind"] != "mob") {
        return false;
    }
    if target["act"] == 2 {
        return false;
    }
    if target["name"] != "똥파리"
            && len(target["targets"]) != 0
            && !array_has(target["targets"], get_string(ob, "이름")) {
        return false;
    }
    true
}

fn find_learned_skill(ob, query) {
    let learned = get_skill_list(ob);
    for name in learned {
        if name.to_string() == query {
            return name.to_string();
        }
    }
    for name in learned {
        let skill_name = name.to_string();
        if starts_with(skill_name, query) {
            return skill_name;
        }
    }
    ""
}

fn print_skill_up(ob, result) {
    if result["level_up"] {
        send_line(ob, "\x1b[1m당신이 무공을 펼치기위한 진기집성이 수월해 지는것을 느낍니다.\x1b[0m\x1b[40m\x1b[37m");
    }
    if result["achievement_up"] {
        send_line(ob, "!!!!!!!!!!!!!");
    }
}

fn cast_combat(ob, skill_name, skill, target) {
    if cast_has_current_skill(ob) {
        send_line(ob, "\x1b[1;37m당신의 내가진기가 흩어지며 기의 순환이 멈추어 버립니다.\x1b[0;37m");
        return;
    }
    if get_int(ob, "act") == 1 && !target["current"] {
        send_line(ob, "☞ 현재의 비무에 신경을 집중하세요. @_@");
        return;
    }
    if target["kind"] == "player" && cast_room_has_attr(ob, "사용자전투금지") {
        send_line(ob, "☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네");
        return;
    }
    // Python 원본은 사용자 전투 지원 여부와 무관하게 다음 분기에서 막는다.
    if target["kind"] == "player" {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }
    if !target["current"] && target["mob_type"] != 1 {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }

    let result = cast_apply_combat(ob, skill_name, target["id"]);
    if result["status"] == "mp" || result["status"] == "hp" {
        send_line(ob, "\x1b[1;37m당신이 내공진기를 끌어 모으지만 기가 흩어져 버립니다.\x1b[0;37m");
        return;
    }
    if result["status"] == "busy" {
        send_line(ob, "\x1b[1;37m당신의 내가진기가 흩어지며 기의 순환이 멈추어 버립니다.\x1b[0;37m");
        return;
    }
    if result["status"] != "ok" {
        send_line(ob, "☞ 그런 상대가 없습니다.");
        return;
    }

    let scripts = make_fight_script(ob, skill["무공스크립"], target);
    send_line(ob, scripts[0]);
    // Python: 시전 직전 Stand면 sendRoom, 이미 Fight면 sendRoomFightScript.
    send_cast_room(ob, scripts[2], "", !result["caster_was_stand"]);

    if result["new_target"] {
        let weapon = get_cast_weapon(ob);
        if result["caster_was_stand"] {
            let own_start = "당신이 " + weapon["fight_start"];
            let room_start = actor_name_a(ob) + han_iga(get_string(ob, "이름"))
                + " " + weapon["fight_start"];
            send_line(ob, own_start);
            send_cast_room(ob, room_start, "", false);
        }
        if result["target_was_stand"] {
            let mob_start = "\x1b[33m" + result["target_name"] + "\x1b[37m"
                + han_iga(result["target_name"]) + " " + result["target_start"];
            send_line(ob, mob_start);
            send_cast_room(ob, mob_start, "", false);
        }
        // Python의 덩달이 전투 몹 분기는 시전자에게만 시작 문구를 보낸다.
        for linked in result["linked"] {
            let linked_start = "\x1b[33m" + linked["name"] + "\x1b[37m"
                + han_iga(linked["name"]) + " " + linked["script"];
            send_line(ob, linked_start);
        }
    }
}

fn cast_noncombat(ob, skill_name, skill, target) {
    if has_skill_attr(skill, "자신금지") && target["kind"] == "player"
            && target["id"] == get_string(ob, "이름") {
        send_line(ob, "☞ 자신에게 사용할 수 없는 무공입니다. ^^");
        return;
    }
    if has_skill_attr(skill, "타인금지")
            && (target["kind"] != "player" || target["id"] != get_string(ob, "이름")) {
        send_line(ob, "☞ 자신만 사용할 수 있는 무공입니다. ^^");
        return;
    }

    let result = cast_apply_noncombat(ob, skill_name, target["kind"], target["id"]);
    if result["status"] == "duplicate" || result["status"] == "mp" {
        send_line(ob, "\x1b[1m당신이 내공진기를 끌어 모으지만 기가 흩어져 버립니다.\x1b[0;37m");
        return;
    }
    if result["status"] == "hp" {
        send_line(ob, "\x1b[1m당신의 내공진기가 흩어지며 기의 순환이 멈추어 버립니다.\x1b[0;37m");
        return;
    }
    if result["status"] != "ok" {
        send_line(ob, "☞ 그런 상대가 없습니다.");
        return;
    }

    print_skill_up(ob, result);
    let scripts = make_fight_script(ob, skill["무공스크립"], target);
    // Python은 실패 스크립트도 같은 시점에 계산하지만 출력하거나 상태에 쓰지 않는다.
    let unused_failure = make_fight_script(ob, skill["실패"], target);
    if result["plus"] > 0 {
        send_line(ob, scripts[0] + " (\x1b[1;36m+ " + result["plus"].to_string() + "\x1b[0;37m)");
    } else {
        send_line(ob, scripts[0]);
    }

    if target["kind"] == "player" && target["id"] != get_string(ob, "이름") {
        send_to_user(target["id"], scripts[1]);
        send_cast_room(ob, scripts[2], target["id"], true);
    } else {
        send_cast_room(ob, scripts[2], "", true);
    }
}

fn cmd(ob, line) {
    if get_int(ob, "act") == 4 {
        send_line(ob, "☞ 운기조식중엔 무공을 사용할 수 없습니다.");
        return;
    }
    if length(line) == 0 {
        send_line(ob, "☞ 사용법: [대상|무공이름] 시전");
        return;
    }

    let parsed = parse_cast_line(line);
    let target = ();
    if parsed["explicit"] {
        target = find_cast_target(ob, parsed["target"]);
        if !target_is_available(ob, target) {
            send_line(ob, "☞ 그런 상대가 없습니다.");
            return;
        }
    } else if get_int(ob, "act") == 1 {
        target = get_cast_current_target(ob);
    }

    let skill_name = find_learned_skill(ob, parsed["skill"]);
    if skill_name == "" {
        send_line(ob, "☞ 그런 무공을 습득한 적이 없습니다.");
        return;
    }
    let skill = get_skill_data(skill_name);
    if skill == () || skill == "" {
        send_line(ob, "☞ 아직 사용할 수 없는 무공입니다.");
        return;
    }

    if skill["종류"] == "전투" {
        if !parsed["explicit"] && get_int(ob, "act") == 0 {
            send_line(ob, "☞ 무공을 펼칠 수 있는 상대가 필요합니다.");
            return;
        }
        if target == () {
            send_line(ob, "☞ 그런 상대가 없습니다.");
            return;
        }
        cast_combat(ob, skill_name, skill, target);
    } else {
        if !parsed["explicit"] {
            target = get_cast_self_target(ob);
        }
        cast_noncombat(ob, skill_name, skill, target);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
