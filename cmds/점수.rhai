// Score command - matching Python output
// Using global helper functions to avoid parsing issues

// 전역 헬퍼 함수
fn pad15(text) {
    let len = length(text);
    if len >= 15 { text } else { text + repeat(" ", 15 - len) }
}

fn cmd(ob, line) {
    let level = get_int(ob, "레벨");
    let age = get_int(ob, "나이");
    let hp = get_int(ob, "체력");
    let max_hp = get_int(ob, "최고체력");
    let mp = get_int(ob, "내공");
    let max_mp = get_int(ob, "최고내공");
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
    let total_exp = get_int(ob, "목표경험치");
    let anger = get_int(ob, "분노");
    let money = get_int(ob, "은전");
    let gold = get_int(ob, "금전");

    // Build lines step by step
    send_line(ob, "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑");
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ \x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥");

    let lv_s = pad_start(int_to_str(level), 6, " ");
    let age_s = pad_start(int_to_str(age), 3, " ");
    send_line(ob, "│ [레  벨]       [" + lv_s + "] │ [나  이]             " + age_s + " │");

    let hp_s = pad15(to_string(hp) + "/" + to_string(max_hp));
    let line1 = "│ [체  력] " + hp_s + " │ [성  격] " + pad15(pdisp) + " │";
    send_line(ob, line1);

    let att_s = pad_start(int_to_str(0), 5, " ") + " + " + pad_start(int_to_str(str_val), 6, " ");
    let line2 = "│ [  힘  ] " + att_s + " │ [성  별] " + pad15(gender) + " │";
    send_line(ob, line2);

    let arm_s = pad_start(int_to_str(0), 6, " ") + " + " + pad_start(int_to_str(armor), 6, " ");
    send_line(ob, "│ [맷  집] " + arm_s + " │ [소  속] " + pad15(gdisp) + " │");

    send_line(ob, "│ [민  첩] " + pad_start(int_to_str(dex), 15, " ") + " │ [직  위] " + pad15(posdisp) + " │");

    send_line(ob, "│ [命  中] " + pad_start(int_to_str(hit), 15, " ") + " │ [回  避] " + pad_start(int_to_str(miss), 15, " ") + " │");

    send_line(ob, "│ [必  殺] " + pad_start(int_to_str(critical), 15, " ") + " │ [  運  ] " + pad_start(int_to_str(luck), 15, " ") + " │");

    let mp_s = pad15(to_string(mp) + "/" + to_string(max_mp));
    send_line(ob, "│ [내  공] " + mp_s + " │ [배우자] " + pad15(sdisp) + " │");

    let w_s = pad15(to_string(weight) + "/" + to_string(max_weight));
    send_line(ob, "│ [현  경] " + pad_start(int_to_str(exp), 15, " ") + " │ [소지품] " + w_s + " │");

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

    let hp_pct = if max_hp > 0 { (hp * 100) / max_hp } else { 0 };
    if hp_pct >= 90 {
        send_line(ob, "★ 당신은 아주 활력이 넘칩니다.");
    } else if hp_pct >= 70 {
        send_line(ob, "★ 당신은 약간 피로를 느낍니다.");
    } else if hp_pct >= 40 {
        send_line(ob, "★ 당신은 상처를 입었습니다.");
    } else if hp_pct >= 20 {
        send_line(ob, "★ 당신은 상처가 깊습니다.");
    } else {
        send_line(ob, "★ 당신은 죽어가고 있습니다!");
    }

    send_line(ob, "★ 당신의 표국보험은 효력이 없습니다.");
    send_line(ob, "★ 당신은 \x1b[1;32m소주천\x1b[0;37;40m을 하고 있습니다.");

    let special = get_int(ob, "특성치");
    if special > 0 {
        send_line(ob, "★ 당신은 " + to_string(special) + "개의 여유 특성치를 보유하고 있습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
