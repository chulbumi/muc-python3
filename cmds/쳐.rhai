// Python cmds/쳐.py의 실제 명령 경로.
// 대상/상태 전이는 efun이 담당하고 모든 문구·ANSI는 이 파일에 둔다.

fn attack_actor_name(ob) {
    let name = get_string(ob, "이름");
    "\x1b[1m" + name + "\x1b[0;37m" + han_iga(name)
}

fn attack_mob_start(name, script) {
    "\x1b[33m" + name + "\x1b[37m" + han_iga(name) + " " + script
}

fn attack_skill_script(line, attacker, target) {
    let text = return_replace(line, "[공]", attacker);
    text = return_replace(text, "[방]", target);
    text = return_replace(text, "[무]", "\x1b[36m주먹\x1b[37m");
    text = post_position_once(text);
    text = post_position_once(text);
    post_position_once(text)
}

fn send_attack_room(ob, messages) {
    if len(messages) == 0 {
        return;
    }
    let own_name = get_string(ob, "이름");
    let text = join(messages, "\r\n");
    for player_name in get_room_players(ob) {
        if player_name != own_name {
            send_to_user(player_name, text);
        }
    }
}

fn attack_prompt(player) {
    if !player["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + player["hp"].to_string() + "/"
        + player["max_hp"].to_string() + ", " + player["mp"].to_string()
        + "/" + player["max_mp"].to_string() + " ] "
}

fn send_attack_prompts(ob, excluded_name) {
    let own_name = get_string(ob, "이름");
    for player in get_cast_room_players(ob) {
        if player["name"] != own_name && player["name"] != excluded_name {
            let prompt = attack_prompt(player);
            if prompt != "" { send_raw_to_user(player["name"], prompt); }
        }
    }
}

fn attack_prompt_for(ob, player_name) {
    for player in get_cast_room_players(ob) {
        if player["name"] == player_name { return attack_prompt(player); }
    }
    ""
}

fn cmd(ob, line) {
    if length(line) == 0 {
        send_line(ob, "☞ 사용법: [대상] 공격");
        return;
    }
    if combat_room_has_attr(ob, "전투금지") {
        send_line(ob, "☞ 이곳에선 모든 전투가 금지되어 있어요. ^^");
        return;
    }
    if contains(line, "시체") {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }

    let mob = find_attack_target(ob, line);
    if mob == () {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }
    if mob["kind"] == "player" {
        if combat_room_has_attr(ob, "사용자전투금지") {
            send_line(ob, "☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네");
            return;
        }
        if mob["name"] == get_string(ob, "이름") || mob["act"] > 1 {
            send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
            return;
        }
        if mob["current"] {
            send_line(ob, "☞ 이미 공격중이에요. ^_^");
            return;
        }
        if combat_target_count(ob) != 0 {
            send_line(ob, "☞ 현재의 비무에 신경을 집중하세요. @_@");
            return;
        }
        let result = start_pvp_attack(ob, mob["name"]);
        if result["status"] == "busy" {
            send_line(ob, "☞ 상대는 이미 다른 비무에 집중하고 있어요.");
            return;
        }
        if result["status"] != "ok" {
            send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
            return;
        }
        let actor = attack_actor_name(ob);
        let target_name = mob["name"];
        if result["caster_was_stand"] {
            let weapon = get_cast_weapon(ob);
            send_line(ob, "당신이 " + weapon["fight_start"]);
        }
        let target_text = actor + " 당신을 공격하기 시작합니다.";
        send_raw_to_user(target_name, "\r\n" + target_text + "\r\n"
            + attack_prompt_for(ob, target_name));
        for player in get_cast_room_players(ob) {
            let player_name = player["name"];
            if player_name != get_string(ob, "이름") && player_name != target_name {
                send_raw_to_user(player_name, "\r\n" + actor + " " + target_name
                    + han_eul(target_name) + " 공격하기 시작합니다.\r\n"
                    + attack_prompt(player));
            }
        }
        return;
    }
    if mob["kind"] != "mob" {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }
    if mob["mob_type"] != 1 || mob["act"] > 1 {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }
    if mob["current"] {
        send_line(ob, "☞ 이미 공격중이에요. ^_^");
        return;
    }
    if combat_target_count(ob) != 0 {
        send_line(ob, "☞ 현재의 비무에 신경을 집중하세요. @_@");
        return;
    }

    let result = start_attack(ob, mob["id"], mob["room_index"]);
    if result["status"] != "ok" {
        // Python의 setFight 내부 즉시 자동무공/몹 무공과 연속 전투 상태가
        // 아직 완전히 표현되지 않는 분기는 성공한 것처럼 출력하지 않는다.
        // 원본에 없는 오류 문구도 만들지 않는다.
        return;
    }

    // Python setFight(mode=False) starts configured automatic martial arts
    // before the ordinary player/mob combat-start lines.
    finish_entry_auto_skill(ob);

    let room_lines = [];
    if result["caster_was_stand"] {
        let weapon = get_cast_weapon(ob);
        send_line(ob, "당신이 " + weapon["fight_start"]);
        room_lines.push(attack_actor_name(ob) + " " + weapon["fight_start"]);
    }
    if result["target_was_stand"] {
        let target_start = attack_mob_start(result["target_name"], result["target_start"]);
        send_line(ob, target_start);
        room_lines.push(target_start);
    }
    send_attack_room(ob, room_lines);
    // Python Player.setFight() follows its noPrompt start messages with
    // promptRoom(), regardless of 타인전투출력거부.
    send_attack_prompts(ob, "");

    // Python의 덩달이 몹 시작 문구는 공격자에게만 보낸다.
    for linked in result["linked"] {
        send_line(ob, attack_mob_start(linked["name"], linked["script"]));
    }
    for skill in result["immediate_skills"] {
        let mob_a = "\x1b[33m" + skill["mob"] + "\x1b[37m";
        let text = attack_skill_script(skill["script"], mob_a, "당신");
        if skill["kind"] == "defense" && skill["amount"] > 0 {
            send_line(ob, text + " (\x1b[1;31m+ " + skill["amount"].to_string() + "\x1b[0;37m)");
        } else {
            send_line(ob, text);
        }
        let room_text = attack_skill_script(skill["script"], mob_a, attack_actor_name(ob));
        for player in get_cast_room_players(ob) {
            if player["name"] != get_string(ob, "이름") && !player["reject_fight"] {
                send_raw_to_user(player["name"], "\r\n" + room_text + "\r\n"
                    + attack_prompt(player));
            }
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
