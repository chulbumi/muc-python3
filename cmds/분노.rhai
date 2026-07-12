// Python `cmds/분노.py`. Rust owns random/state mutations; Rhai owns text.

fn anger_script(line, attacker, target, weapon) {
    let text = return_replace(line, "[공]", attacker);
    text = return_replace(text, "[방]", target);
    text = return_replace(text, "[무]", weapon);
    text = post_position_once(text);
    text = post_position_once(text);
    post_position_once(text)
}

fn cmd(ob, line) {
    if get_int(ob, "act") != 1 || combat_target_count(ob) == 0 {
        send_line(ob, "☞ 지금은 \x1b[1m\x1b[31m살겁\x1b[0m\x1b[37m\x1b[40m을 일으키기에 부적합한 상황 이라네");
        return;
    }
    if get_int(ob, "분노") < 100 {
        send_line(ob, "☞ 당신은 아직 \x1b[1;40;31m분노\x1b[0;40;37m를 표출할 수 없습니다.");
        return;
    }
    if anger_guard_count(ob) < 1 {
        set_anger(ob, get_int(ob, "분노") - 100);
        send_line(ob, "☞ 당신이 \x1b[1;40;31m분노\x1b[0;40;37m를 스스로 다스립니다.");
        return;
    }

    let target = if line == "" { anger_primary_target(ob) } else { find_attack_target(ob, line) };
    if target.is_unit() || (target.contains("found") && !target["found"]) {
        send_line(ob, "☞ 공격할 그런 대상이 없습니다.");
        return;
    }
    if !target["current"] {
        send_line(ob, "☞ 현재의 비무에 신경을 집중하세요. @_@");
        return;
    }

    let result = run_guard_anger(ob, target["id"], target["room_index"]);
    if result["status"] != "ok" {
        send_line(ob, "☞ 공격할 그런 대상이 없습니다.");
        return;
    }
    let own = "\x1b[1m" + get_string(ob, "이름") + "\x1b[0;37m";
    let victim = "\x1b[33m" + result["target"] + "\x1b[37m";
    let room_lines = [];
    let first = true;
    for event in result["events"] {
        let guard = if event["guard_ansi"] != "" {
            event["guard_ansi"] + event["guard"] + "\x1b[0;37m"
        } else {
            "\x1b[0;36m" + event["guard"] + "\x1b[37m"
        };
        if first {
            let use_self = anger_script(event["use_script"], "당신", victim, guard);
            let use_room = anger_script(event["use_script"], own, victim, guard);
            send_line(ob, use_self + "\r\n");
            room_lines.push(use_room + "\r\n");
            first = false;
        }
        let script = if event["hit"] { event["attack_script"] } else { event["fail_script"] };
        let self_text = anger_script(script, "당신", victim, guard);
        let room_text = anger_script(script, own, victim, guard);
        if event["hit"] {
            send_line(ob, self_text + " \x1b[1;36m" + event["damage"].to_string() + "\x1b[0;37m");
        } else {
            send_line(ob, self_text);
        }
        room_lines.push(room_text + "\r\n");
    }
    if len(room_lines) > 0 { send_room(ob, join(room_lines, "")); }
}

fn main(ob, line) { cmd(ob, line); }
