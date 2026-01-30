// 능력치/점수/상태 명령
fn main(ob, line) {
    let name = get_string(ob, "이름");
    let level = get_int(ob, "레벨");
    let age = get_int(ob, "나이");
    let hp = get_int(ob, "체력");
    let max_hp = get_int(ob, "최고체력");
    let mp = get_int(ob, "내공");
    let max_mp = get_int(ob, "최고내공");
    let str_val = get_int(ob, "힘");
    let con = get_int(ob, "맷집");
    let dex = get_int(ob, "민첩성");
    let exp = get_int(ob, "현재경험치");
    let money = get_int(ob, "은전");

    let lv_s = pad_start(int_to_str(level), 5, " ");
    let age_s = pad_start(int_to_str(age), 4, " ");
    let hp_s = pad_start(int_to_str(hp), 6, " ");
    let max_hp_s = pad_start(int_to_str(max_hp), 6, " ");
    let mp_s = pad_start(int_to_str(mp), 6, " ");
    let max_mp_s = pad_start(int_to_str(max_mp), 6, " ");
    let money_s = pad_start(int_to_str(money), 20, " ");
    let name_pad = if length(name) >= 10 { name } else { fill_space(10, name) };

    send_line(ob, "┏━━━━━━━━━━━━━━━━━━━━━━━━━┑");
    send_line(ob, "│[44m[1m[37m ▷▶▷▶▷ " + name_pad + "의 현재 상태     ◁◀◁◀◁ [0m[40m[37m│");
    send_line(ob, "┝━━━━━━━━━━┯━━━━━━━━━━━━┥");
    send_line(ob, "│ [레  벨]       [" + lv_s + "] │ [나  이]          " + age_s + " │");
    send_line(ob, "│ [체  력]     " + hp_s + "/" + max_hp_s + " │ [내  공]     " + mp_s + "/" + max_mp_s + " │");
    send_line(ob, "│ [  힘  ]       " + pad_start(int_to_str(str_val), 6, " ") + " │ [맷  집]       " + pad_start(int_to_str(con), 6, " ") + " │");
    send_line(ob, "│ [민  첩]       " + pad_start(int_to_str(dex), 6, " ") + " │ [현  경]       " + pad_start(int_to_str(exp), 6, " ") + " │");
    send_line(ob, "├────────────┴────────────┤");
    send_line(ob, "│[0m[47m[30m [은  전]                    " + money_s + " [0m[40m[37m│");
    send_line(ob, "┕━━━━━━━━━━━━━━━━━━━━━━━━━┙");
}
