// Private heartbeat combat renderer. Rust supplies state-only event maps.

fn combat_tick_replace(line, attacker, target, weapon) {
    let text = return_replace(line, "[공]", attacker);
    text = return_replace(text, "[방]", target);
    text = return_replace(text, "[무]", weapon);
    text = post_position_once(text);
    text = post_position_once(text);
    post_position_once(text)
}

fn one_fight_script(key, fallback) {
    let scripts = get_fight_scripts(key);
    if scripts.len == 0 { return fallback; }
    scripts[random(0, scripts.len - 1)]
}

fn cmd(ob, line) {
    let own_name = get_string(ob, "이름");
    let own_a = "\x1b[1m" + own_name + "\x1b[0;37m";
    let training = get_user_config(ob, "수련모드") == "1";
    for event in take_combat_presentation_events(ob) {
        if event["kind"] == "combat_prompt" {
            let vitals = return_vitals(ob);
            if training {
                send_line(ob, "\r\n\x1b[0;37;40m[ " + vitals["hp"].to_string() + "/" + vitals["max_hp"].to_string()
                    + ", " + vitals["mp"].to_string() + "/" + vitals["max_mp"].to_string() + " ] \r");
            } else if get_user_config(ob, "엘피출력") != "1" {
                send_line(ob, "\r\n\x1b[0;37;40m[ " + vitals["hp"].to_string() + "/" + vitals["max_hp"].to_string()
                    + ", " + vitals["mp"].to_string() + "/" + vitals["max_mp"].to_string() + " ] ");
            }
            continue;
        }
        if event["kind"] == "age_up" {
            let own_name = get_string(ob, "이름");
            let room_message = "\x1b[1m" + own_name + "의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돑니다.\x1b[0;37m";
            send_line(ob, "\r\n\x1b[1m당신의 단전에 회오리가 몰아치며 몸주위에 하얀 진기가 맴돑니다.\x1b[0;37m");
            for player_name in get_room_players(ob) {
                if player_name != own_name { send_to_user(player_name, room_message); }
            }
            continue;
        }
        if event["kind"] == "nickname_invitation" {
            send_line(ob, "\r\n" + join(get_murim_config_list("무림별호이벤트호출"), "\r\n"));
            continue;
        }
        if event["kind"] == "defense_expired" {
            let own_name = get_string(ob, "이름");
            let actor = "\x1b[1m" + own_name + "\x1b[0;37m";
            let own_lines = [];
            let room_line = "";
            for script in event["scripts"] {
                let own_text = combat_tick_replace(script, "당신", "", "");
                own_lines.push(own_text);
                room_line = combat_tick_replace(script, actor, "", "");
            }
            if own_lines.len > 0 { send_line(ob, "\r\n" + join(own_lines, "\r\n")); }
            if room_line != "" {
                for player_name in get_room_players(ob) {
                    if player_name != own_name { send_to_user(player_name, "\r\n" + room_line); }
                }
            }
            continue;
        }
        if event["kind"] == "strength_up" || event["kind"] == "dexterity_up" || event["kind"] == "mastery_up" {
            let key = if event["kind"] == "strength_up" {
                "힘증가스크립"
            } else if event["kind"] == "dexterity_up" {
                "민첩성증가스크립"
            } else {
                "숙련도증가스크립"
            };
            for message in get_murim_config_list(key) {
                send_line(ob, message);
            }
            continue;
        }
        if event["kind"] == "anger_100" {
            let self_message = "당신이 갑자기 \x1b[1;40;31m괴성\x1b[0;40;37m을 지르며 \x1b[1;40;31m난동\x1b[0;40;37m을 부립니다. '끄오오오오오~~'";
            let room_message = own_a + " 갑자기 \x1b[1;40;31m괴성\x1b[0;40;37m을 지르며 \x1b[1;40;31m난동\x1b[0;40;37m을 부립니다. '끄오오오오오~~'";
            send_line(ob, self_message);
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name && !player["reject_fight"] { send_to_user(player["name"], room_message); }
            }
            continue;
        }
        if event["kind"] == "item_skill_learned" {
            let message = "\r\n\x1b[1m\x1b[40m\x1b[37m당신이 『\x1b[1m\x1b[40m\x1b[32m" + event["skill"] + "\x1b[1m\x1b[40m\x1b[37m』의 무공 구결을 깨우치기 시작합니다. 'ΔΨΞλΟ~'\x1b[0m\x1b[40m\x1b[37m\r\n";
            send_line(ob, message);
            let room_message = "\x1b[1m\x1b[40m\x1b[37m" + own_a + " 『\x1b[1m\x1b[40m\x1b[32m" + event["skill"] + "\x1b[1m\x1b[40m\x1b[37m』의 무공 구결을 깨우치기 시작합니다. 'ΔΨΞλΟ~'\x1b[0m\x1b[40m\x1b[37m";
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name { send_to_user(player["name"], room_message); }
            }
            continue;
        }
        if event["kind"] == "skill_level_up" {
            send_line(ob, "\x1b[1m당신이 무공을 펼치기위한 진기집성이 수월해 지는것을 느낍니다.\x1b[0m\x1b[40m\x1b[37m");
            continue;
        }
        if event["kind"] == "armor_up" {
            for message in get_murim_config_list("맷집증가스크립") { send_line(ob, message); }
            continue;
        }
        if event["kind"] == "combat_recovery" {
            if training { continue; }
            let message = combat_tick_replace(event["script"], "당신", "당신", "")
                + " (\x1b[1;32m+ " + event["amount"].to_string() + "\x1b[0;37m)";
            send_line(ob, message);
            continue;
        }
        if event["kind"] == "vision_learned" {
            let message = "\x1b[1m당신이 『\x1b[32m" + event["skill"] + "\x1b[37m』의 무공 구결을 깨우치기 시작합니다. 'ΔΨΞλΟ~'\x1b[0;37m\r\n";
            send_line(ob, message);
            let room_message = "\x1b[1m" + own_a + han_iga(own_name) + " 『\x1b[32m" + event["skill"] + "\x1b[37m』의 무공 구결을 깨우치기 시작합니다. 'ΔΨΞλΟ~'\x1b[0;37m";
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name { send_to_user(player["name"], room_message); }
            }
            continue;
        }
        if event["kind"] == "player_death" {
            send_line(ob, "\r\n\x1b[1;37m당신이 쓰러집니다. '쿠웅~~ 철퍼덕~~'\x1b[0;37m");
            continue;
        }
        if event["kind"] == "death_progress" {
            let step = event["step"];
            if step == 0 { send_line(ob, "\r\n기혈이 거꾸로 돌며 정신이 혼미해 집니다."); }
            else if step == 1 { send_line(ob, "\r\n누군가가 당신 주위를 어슬렁 거립니다."); }
            else if step == 2 { send_line(ob, "\r\n웅성 웅성 거리는 소리가 귓전에 맴돌며 점점 멀어져 갑니다."); }
            else if step == 4 { send_line(ob, "\r\n코끝을 찌르는 향냄새에 정신을 차려보니 장의사 내부다."); }
            else if step == 5 { send_line(ob, "\r\n장의사가 말합니다. \"앗~~ 죽은줄 알았는데 다시 깨어나는군~\""); }
            else if step == 6 { send_line(ob, "\r\n장의사가 말합니다. \"하나뿐인 목숨 무모하게 행동하지 말고 조심해서 행동하게나.\""); }
            else if step == 7 { send_line(ob, "\r\n당신이 떠지지 않는 눈을 힘겹게 뜨며 주위를 살펴 봅니다."); }
            else if step == 8 {
                if event["insured_items"] == 0 {
                    send_line(ob, "\r\n장의사가 말합니다. \"쯧쯧... 표국에서 보험을 들지 않았군...\"");
                    send_line(ob, "                   \"무리한 무공수련은 화를 자초한다네.\"");
                } else {
                    send_line(ob, "\r\n장의사가 말합니다. \"자네가 가지고 다니던 물건은 표국에서 회수 해왔으니\"");
                    send_line(ob, "                   \"잃어버린 것이 없는지 확인해 보게나..\"");
                    send_line(ob, "                   \"표국무사가 그러는데 보험료가 나갔다더군...\"");
                }
            } else if step == 9 {
                send_line(ob, "\r\n당신이 자세를 편안히 하며 운기조식에 들어갑니다.");
                send_line(ob, "\x1b[1;36m당신의 기혈이 타동하기 시작합니다.\x1b[0;37m");
                let own_name = get_string(ob, "이름");
                let room_message = "\x1b[1m" + own_name + "\x1b[0;37m" + han_iga(own_name)
                    + " 자세를 편안히 하며 운기조식에 들어갑니다.";
                for player_name in get_room_players(ob) {
                    if player_name != own_name { send_to_user(player_name, room_message); }
                }
            }
            if step != 3 && get_user_config(ob, "엘피출력") != "1" {
                let vitals = return_vitals(ob);
                send_line(ob, "\x1b[0;37;40m[ " + vitals["hp"].to_string() + "/"
                    + vitals["max_hp"].to_string() + ", " + vitals["mp"].to_string()
                    + "/" + vitals["max_mp"].to_string() + " ] ");
            }
            continue;
        }
        if event["kind"] == "death_room_transition" {
            let player_name = event["player"];
            let actor = "\x1b[1m" + player_name + "\x1b[0;37m";
            for recipient in event["old_players"] {
                if recipient != player_name {
                    send_to_user(recipient, "\x1b[1m장의사\x1b[0;37m가 " + actor
                        + han_eul(player_name) + " 데려갑니다.");
                }
            }
            let expired = event["expired_items"];
            while expired.len > 0 {
                let item_name = expired[0];
                let count = 0;
                let remaining = [];
                for candidate in expired {
                    if candidate == item_name { count += 1; }
                    else { remaining.push(candidate); }
                }
                expired = remaining;
                let message = if count == 1 {
                    item_name + han_iga(item_name) + " 먼지가 되어 사라집니다."
                } else {
                    item_name + " " + count + "개가 먼지가 되어 사라집니다."
                };
                for recipient in event["new_players"] {
                    if recipient != player_name { send_to_user(recipient, "\r\n" + message); }
                }
            }
            for message in event["update_messages"] {
                for recipient in event["new_players"] {
                    if recipient != player_name { send_to_user(recipient, "\r\n" + message); }
                }
            }
            emit_room_view(ob);
            for recipient in event["new_players"] {
                if recipient != player_name {
                    send_to_user(recipient, actor + han_iga(player_name) + " 손수레에 실려옵니다.");
                }
            }
            if get_user_config(ob, "엘피출력") != "1" {
                let vitals = return_vitals(ob);
                send_line(ob, "\x1b[0;37;40m[ " + vitals["hp"].to_string() + "/"
                    + vitals["max_hp"].to_string() + ", " + vitals["mp"].to_string()
                    + "/" + vitals["max_mp"].to_string() + " ] ");
            }
            continue;
        }
        if event["kind"] == "room_corpse_update" {
            send_line(ob, corpse_update_text(event));
            continue;
        }
        if event["kind"] == "mob_death" {
            let death = event["script"];
            if death == "" {
                death = event["mob"] + han_iga(event["mob"]) + " 쓰러집니다. '쿠웅~~ 철퍼덕~~'";
            }
            let death = "\r\n\x1b[1;37m" + death + "\x1b[0;37m";
            send_line(ob, death);
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name && !player["reject_fight"] {
                    send_to_user(player["name"], death);
                }
            }
            continue;
        }
        if event["kind"] == "reward" {
            let mob_name = "\x1b[33m" + event["mob"] + "\x1b[37m";
            if event["bonus_exp"] != 0 || event["bonus_gold"] != 0 {
                send_line(ob, "\r\n당신이 " + event["exp"].to_string() + "(+" + event["bonus_exp"].to_string() + ")의 경험치를 얻습니다.");
                send_line(ob, "당신이 " + mob_name + "에게 은전 " + event["gold"].to_string() + "(+" + event["bonus_gold"].to_string() + ")개를 획득합니다.");
            } else {
                send_line(ob, "\r\n당신이 " + event["exp"].to_string() + "의 경험치를 얻습니다.");
                send_line(ob, "당신이 " + mob_name + "에게 은전 " + event["gold"].to_string() + "개를 획득합니다.");
            }
            continue;
        }
        if event["kind"] == "level_up" {
            for message in get_murim_config_list("레벨증가스크립") { send_line(ob, message); }
            send_line(ob, "☞ 체력 상승 ▷ " + event["hp_increase"].to_string() + "〔" + event["max_hp"].to_string() + "〕, 맷집 상승 ▷ 1〔" + event["armor"].to_string() + "〕");
            continue;
        }
        if event["kind"] == "reward_observer" {
            let actor = "\x1b[1m" + event["player"] + "\x1b[0;37m" + han_iga(event["player"]);
            let message = actor + " 약간의 경험치를 얻습니다.\r\n" + actor + " 몇개의 은전을 획득합니다.";
            for item in event["loot"] {
                message += "\r\n" + actor + " " + item + han_eul(strip_ansi(item)) + " 전리품으로 획득합니다.";
            }
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name { send_to_user(player["name"], message); }
            }
            continue;
        }
        if event["kind"] == "loot" {
            send_line(ob, "당신이 " + event["item"] + han_eul(strip_ansi(event["item"])) + " 전리품으로 획득합니다.");
            continue;
        }
        if event["kind"] == "combat_error" {
            if event["code"] == "already_dead" { send_line(ob, "☞ 이미 죽은 몬스터입니다."); }
            continue;
        }
        if event["kind"] == "player_attack" || event["kind"] == "player_miss" {
            if training { continue; }
            let suffix = if event["kind"] == "player_miss" { "전투실패스크립" } else { "전투스크립" };
            let line = one_fight_script(event["weapon_type"] + suffix, "[공](이/가) [방](을/를) 공격했습니다");
            let mob_a = "\x1b[33m" + event["mob"] + "\x1b[37m";
            let weapon = if event["kind"] == "player_attack" { event["weapon"] } else { "\x1b[36m주먹\x1b[37m" };
            let to_self = combat_tick_replace(line, "당신", mob_a, weapon);
            let to_room = combat_tick_replace(line, own_a, mob_a, weapon);
            if event["kind"] == "player_attack" {
                send_line(ob, to_self + "  \x1b[1;36m" + event["damage"].to_string() + "\x1b[0;37m");
            } else {
                send_line(ob, to_self);
            }
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name && !player["reject_fight"] { send_to_user(player["name"], to_room); }
            }
            continue;
        }
        if event["kind"] == "mob_normal_attack" || event["kind"] == "mob_normal_miss" {
            if training { continue; }
            let suffix = if event["kind"] == "mob_normal_miss" { "전투실패스크립" } else { "전투스크립" };
            let line = one_fight_script(event["weapon_type"] + suffix, "[공](이/가) [방](을/를) 공격했습니다");
            let mob_a = "\x1b[33m" + event["mob"] + "\x1b[37m";
            let weapon = "\x1b[36m주먹\x1b[37m";
            let to_self = combat_tick_replace(line, mob_a, "당신", weapon);
            let to_room = combat_tick_replace(line, mob_a, own_a, weapon);
            if event["kind"] == "mob_normal_attack" {
                send_line(ob, to_self + "  \x1b[1;31m" + event["damage"].to_string() + "\x1b[0;37m");
            } else {
                send_line(ob, to_self);
            }
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name && !player["reject_fight"] { send_to_user(player["name"], to_room); }
            }
            continue;
        }
        if event["kind"] == "player_skill_form" || event["kind"] == "player_skill_attack" || event["kind"] == "player_skill_fail" {
            if training { continue; }
            let mob_a = "\x1b[33m" + event["mob"] + "\x1b[37m";
            let to_self = combat_tick_replace(event["script"], "당신", mob_a, event["weapon"]);
            let to_room = combat_tick_replace(event["script"], own_a, mob_a, event["weapon"]);
            if event["kind"] == "player_skill_attack" {
                send_line(ob, to_self + "  \x1b[1;36m" + event["damage"].to_string() + "\x1b[0;37m");
            } else {
                send_line(ob, to_self);
            }
            for player in get_cast_room_players(ob) {
                if player["name"] != own_name && !player["reject_fight"] { send_to_user(player["name"], to_room); }
            }
            continue;
        }
        let mob_a = "\x1b[33m" + event["mob"] + "\x1b[37m";
        if training && (event["kind"] == "mob_attack" || event["kind"] == "mob_fail" || event["kind"] == "mob_form") { continue; }
        let weapon = "\x1b[36m주먹\x1b[37m";
        let to_self = combat_tick_replace(event["script"], mob_a, "당신", weapon);
        let to_room = combat_tick_replace(event["script"], mob_a, own_a, weapon);
        let damage = event["damage"];
        if event["kind"] == "mob_attack" {
            send_line(ob, to_self + "  \x1b[1;31m" + damage.to_string() + "\x1b[0;37m");
        } else if event["kind"] == "mob_defense" && damage > 0 {
            send_line(ob, to_room + " (\x1b[1;31m+ " + damage.to_string() + "\x1b[0;37m)");
        } else if event["kind"] == "mob_defense" {
            send_line(ob, to_room);
        } else {
            send_line(ob, to_self);
        }
        for player in get_cast_room_players(ob) {
            if player["name"] != own_name && !player["reject_fight"] {
                send_to_user(player["name"], to_room);
            }
        }
    }
}

fn main(ob, line) { cmd(ob, line); }
