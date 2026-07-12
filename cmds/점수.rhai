// Score command - matching Python output
// Using global helper functions to avoid parsing issues

// 전역 헬퍼 함수
fn pad15(text) {
    let len = length(text);
    if len >= 15 { text } else { text + repeat(" ", 15 - len) }
}

fn pad15_front(text) {
    fill_space_front_euc_kr(15, text)
}

fn cmd(ob, line) {
    send_line(ob, "☞ " + get_self_desc(ob));
    let level = get_int(ob, "레벨");
    let age = get_int(ob, "나이");
    let hp = get_int(ob, "체력");
    let max_hp = get_max_hp(ob);
    let mp = get_int(ob, "내공");
    let max_mp = get_max_mp(ob);
    let str_val = get_int(ob, "힘");

    let personality = get_string(ob, "성격");
    let pdisp = if personality == "" { "----------" } else { personality };
    let gender = get_string(ob, "성별");
    let armor = get_int(ob, "맷집");
    let guild = get_string(ob, "소속");
    let gdisp = if guild == "" { "----------" } else { guild };
    let position = get_string(ob, "직위");
    let posdisp = if position == "" { "----------" } else { position };
    let dex = get_int(ob, "민첩성");
    let hit = get_int(ob, "명중");
    let miss = get_int(ob, "회피");
    let critical = get_int(ob, "필살");
    let luck = get_int(ob, "운");
    let spouse = get_string(ob, "배우자");
    let sdisp = if spouse == "" { "----------" } else { spouse };
    let exp = get_int(ob, "현재경험치");
    let weight = get_int(ob, "소지품무게");
    let max_weight = str_val * 10;
    let total_exp = get_total_exp(ob);
    let anger = get_int(ob, "분노");
    let money = get_int(ob, "은전");
    let gold = get_int(ob, "금전");

    // Build lines step by step
    send_line(ob, "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑");
    send_line(ob, "│\x1b[0m\x1b[44m\x1b[1m\x1b[37m ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ \x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥");

    let lv_s = pad_start(int_to_str(level), 6, " ");
    let age_s = pad_start(int_to_str(age), 3, " ");
    send_line(ob, "│ [레  벨]        [" + lv_s + "] │ [나  이]             " + age_s + " │");

    let hp_s = pad15(to_string(hp) + "/" + to_string(max_hp));
    let line1 = "│ [체  력] " + pad15_front(to_string(hp) + "/" + to_string(max_hp)) + " │ [성  격] " + pad15_front(pdisp) + " │";
    send_line(ob, line1);

    let att_s = pad_start(int_to_str(get_att_power(ob)), 6, " ") + " + " + pad_start(int_to_str(str_val), 6, " ");
    let line2 = "│ [  힘  ] " + att_s + " │ [성  별] " + pad15_front(gender) + " │";
    send_line(ob, line2);

    let arm_s = pad_start(int_to_str(get_armor(ob)), 6, " ") + " + " + pad_start(int_to_str(get_arm(ob)), 6, " ");
    send_line(ob, "│ [맷  집] " + arm_s + " │ [소  속] " + pad15_front(gdisp) + " │");

    send_line(ob, "│ [민  첩] " + pad_start(int_to_str(dex), 15, " ") + " │ [직  위] " + pad15_front(posdisp) + " │");

    send_line(ob, "│ [命  中] " + pad_start(int_to_str(hit), 15, " ") + " │ [回  避] " + pad_start(int_to_str(miss), 15, " ") + " │");

    send_line(ob, "│ [必  殺] " + pad_start(int_to_str(critical), 15, " ") + " │ [  運  ] " + pad_start(int_to_str(luck), 15, " ") + " │");

    let mp_s = pad15(to_string(mp) + "/" + to_string(max_mp));
    send_line(ob, "│ [내  공] " + pad15_front(to_string(mp) + "/" + to_string(max_mp)) + " │ [배우자] " + pad15_front(sdisp) + " │");

    let w_s = pad15(to_string(weight) + "/" + to_string(max_weight));
    send_line(ob, "│ [현  경] " + pad_start(int_to_str(exp), 15, " ") + " │ [소지품] " + pad15_front(to_string(weight) + "/" + to_string(max_weight)) + " │");

    let anger_s = if anger >= 100 { "\x1b[1;31m" + to_string(anger) + "\x1b[0;37m" } else { to_string(anger) };
    let anger_pad = "             " + pad_start(anger_s, 3, " ");
    send_line(ob, "│ [목  경] " + pad_start(int_to_str(total_exp), 15, " ") + " │ [분  노]" + anger_pad + " │");

    send_line(ob, "├─────────────┴─────────────┤");

    let money_s = pad_start(int_to_str(money), 40, " ");
    send_line(ob, "│\x1b[0m\x1b[47m\x1b[30m [은  전]    " + money_s + " \x1b[0m\x1b[40m\x1b[37m│");

    if gold > 0 {
        let gold_s = pad_start(int_to_str(gold), 40, " ");
        send_line(ob, "│\x1b[0m\x1b[43m\x1b[30m [금  전]    " + gold_s + " \x1b[0m\x1b[40m\x1b[37m│");
    }

    send_line(ob, "┕━━━━━━━━━━━━━━━━━━━━━━━━━━━┙");

    if guild != "" {
        let title = if position == "" { "" } else { position };
        let nickname = get_string(ob, "방파별호");
        let nickname_text = if nickname == "" { "" } else { "(" + nickname + ")" };
        send_line(ob, "★ 당신" + han_un("당신") + " \x1b[1m【" + guild + "】\x1b[0m 문파의 \x1b[1m" + title + nickname_text + "\x1b[0m 입니다.");
    }

    let python_max_hp = min(get_persisted_max_hp(ob), get_int(ob, "최대체력"));
    let hp_script = if hp >= python_max_hp {
        "(은/는) 아주 활력이 넘칩니다."
    } else {
        get_hp_status_script(ob)
    };
    send_line(ob, "★ " + post_position_once("당신" + hp_script));

    let insurance = get_insurance_view(ob);
    let insurance_count = get_int(insurance, "count");
    if insurance_count == 0 {
        send_line(ob, "★ 당신의 표국보험은 효력이 없습니다.");
    } else {
        send_line(ob, "★ 당신은 " + int_to_str(insurance_count) + "번의 표국보험 혜택을 받으실 수 있습니다.");
    }
    send_line(ob, "★ " + post_position_once("당신" + get_mp_status_script(ob)));

    let special = get_int(ob, "특성치");
    if special > 0 {
        send_line(ob, "★ 당신은 " + to_string(special) + "개의 여유 특성치를 보유하고 있습니다.");
    }
    send_line(ob, "");
}

fn main(ob, line) {
    cmd(ob, line);
}
