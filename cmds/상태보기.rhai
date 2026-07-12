// Python cmds/상태보기.py. Rust supplies state; Rhai owns all presentation.
fn status_value(v, fallback) { if v == "" { fallback } else { v } }

fn render_player_status(ob, s) {
    let personality = status_value(s["성격"], "--------");
    let sex = status_value(s["성별"], "--");
    let guild = status_value(s["소속"], "--------");
    let title = status_value(s["직위"], "--------");
    let spouse = status_value(s["배우자"], "--------");
    send_line(ob, "┏━━━━━━━━━━━━━━━━━━━━━━━━━┑");
    send_line(ob, "│\x1b[0m\x1b[44m\x1b[1m\x1b[37m ▷▶▷▶▷ " + pad_start(s["name"], 10, " ") + "의 현재 상태     ◁◀◁◀◁ \x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "┝━━━━━━━━━━━━┯━━━━━━━━━━━━┥");
    send_line(ob, "│ [레  벨]       [" + pad_start(s["level"].to_string(), 5, " ") + "] │ [나  이]          " + pad_start(s["age"].to_string(), 4, " ") + " │");
    send_line(ob, "│ [체  력] " + pad_start(s["hp"].to_string() + "/" + s["max_hp"].to_string(), 13, " ") + " │ [성  격]      " + pad_start(personality, 8, " ") + " │");
    send_line(ob, "│ [  힘  ]  " + pad_start(s["attack"].to_string(), 4, " ") + " + " + pad_start(s["strength"].to_string(), 5, " ") + " │ [성  별]            " + pad_start(sex, 2, " ") + " │");
    send_line(ob, "│ [맷  집] " + pad_start(s["armor"].to_string(), 5, " ") + " + " + pad_start(s["arm"].to_string(), 5, " ") + " │ [소  속]      " + pad_start(guild, 8, " ") + " │");
    send_line(ob, "│ [민  첩]  " + pad_start(s["dex"].to_string(), 12, " ") + " │ [직  위]      " + pad_start(title, 8, " ") + " │");
    send_line(ob, "│ [내  공]  " + pad_start(s["mp"].to_string() + "/" + s["max_mp"].to_string(), 12, " ") + " │ [배우자]      " + pad_start(spouse, 8, " ") + " │");
    send_line(ob, "│ [현  경]  " + pad_start(s["current_exp"].to_string(), 12, " ") + " │ [소지품]  " + pad_start(s["weight"].to_string() + "/" + (s["strength"] * 10).to_string(), 12, " ") + " │");
    send_line(ob, "│ [목  경]  " + pad_start(s["total_exp"].to_string(), 12, " ") + " │ [분  노]             0 │");
    send_line(ob, "│ [命  中] " + pad_start(s["hit"].to_string(), 15, " ") + " │ [回  避] " + pad_start(s["miss"].to_string(), 15, " ") + " │");
    send_line(ob, "│ [必  殺] " + pad_start(s["critical"].to_string(), 15, " ") + " │ [  運  ] " + pad_start(s["luck"].to_string(), 15, " ") + " │");
    send_line(ob, "├────────────┴────────────┤");
    send_line(ob, "│\x1b[0m\x1b[47m\x1b[30m [은  전]                    " + pad_start(s["silver"].to_string(), 20, " ") + " \x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "┕━━━━━━━━━━━━━━━━━━━━━━━━━┙");
    send_line(ob, "★ " + post_position_once(s["name"] + s["hp_script"]));
    if s["insurance"] == 0 { send_line(ob, "★ \x1b[1m" + s["name"] + "\x1b[0;37m의 표국보험은 효력이 없습니다."); }
    else { send_line(ob, "★ \x1b[1m" + s["name"] + "\x1b[0;37m" + han_iga(s["name"]) + " " + s["insurance"].to_string() + "번의 표국보험 혜택을 받으실 수 있습니다."); }
    send_line(ob, "★ " + post_position_once(s["name"] + s["mp_script"]));
    if s["feature"] != 0 { send_line(ob, "★ \x1b[1m" + s["name"] + "\x1b[0;37m" + han_eun(s["name"]) + " " + s["feature"].to_string() + "개의 여유 특성치를 보유하고 있습니다."); }
}

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if line == "" { send_line(ob, "☞ 운영자 명령: [대상] 상태보기"); return; }
    let target = first_arg(line);
    if target == get_string(ob, "이름") {
        let s = admin_self_status(ob); s["name"] = s["이름"];
        render_player_status(ob, s); return;
    }
    for s in get_room_players_admin(ob) {
        if s["name"] != target { continue; }
        s["insurance"] = admin_insurance_count(s["level"], s["insurance_premium"]);
        // Python accidentally calls get_hp_script(ob), not get_hp_script(obj).
        s["hp_script"] = get_hp_status_script(ob);
        render_player_status(ob, s); return;
    }
    let selected = find_attack_target(ob, target);
    if !selected.is_unit() && selected["kind"] == "mob" {
        for s in get_room_mobs_admin(ob) {
            if s["instance_id"] != selected["instance_id"] { continue; }
            s["hp_script"] = get_hp_status_script(ob);
            send_line(ob, "Index : " + s["index"]);
            render_player_status(ob, s); return;
        }
    }
    send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
}
fn main(ob, line) { cmd(ob, line); }
