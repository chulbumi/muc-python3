// 능력치.rhai - 능력치 명령어 (점수와 동일). Python: cmds/점수.py
// Python 서버의 점수.py 출력과 동일하게 만듭니다

// 전역 헬퍼 함수
fn pad15(text) {
    let len = length(text);
    if len >= 15 { text } else { text + repeat(" ", 15 - len) }
}

fn pad_any(text, target_len) {
    let len = length(text);
    if len >= target_len { text } else { text + repeat(" ", target_len - len) }
}

fn cmd(ob, line) {
    // 데이터 가져오기
    let name = get_string(ob, "이름");

    // Python: ob.get('레벨')
    let level = get_int(ob, "레벨");
    let age = get_int(ob, "나이");

    // Python: ob.getHp(), ob.getMaxHp()
    // 서버는 hp/max_hp 속성 사용
    let hp = get_int(ob, "체력");
    let max_hp = get_int(ob, "최고체력");

    // Python: ob.getMp(), ob.getMaxMp()
    let mp = get_int(ob, "내공");
    let max_mp = get_int(ob, "최고내공");

    // Python: ob.getAttPower() (무기 공격력), ob.getStr() (기본 힘)
    let str_val = get_int(ob, "힘");

    // 성격 (없으면 ----------)
    let 성경_val = get_string(ob, "성격");
    let 성격_display = if 성경_val == "" { "----------" } else { 성경_val };

    // 성별
    let 성별_val = get_string(ob, "성별");

    // Python: ob.getArmor() (방어력), ob.getArm() (기본 맷집)
    let armor = get_int(ob, "맷집");

    // 소속 (없으면 ----------)
    let 소속_val = get_string(ob, "소속");
    let 소속_display = if 소속_val == "" { "----------" } else { 소속_val };

    // 직위
    let 직위_val = get_string(ob, "직위");
    let 직위_display = if 직위_val == "" { "----------" } else { 직위_val };

    // Python: ob.getDex()
    let dex = get_int(ob, "민첩성");

    // Python: ob.getHit(), ob.getMiss()
    let hit = get_int(ob, "명중");
    let miss = get_int(ob, "회피");

    // Python: ob.getCritical(), ob.getCriticalChance()
    let critical = get_int(ob, "필살");
    let luck = get_int(ob, "운");

    // 배우자
    let 배우자_val = get_string(ob, "배우자");
    let 배우자_display = if 배우자_val == "" { "----------" } else { 배우자_val };

    // 현재경험치
    let exp = get_int(ob, "현재경험치");

    // 소지품 무게 / 최대 무게
    let weight = get_int(ob, "소지품무게");
    let max_weight_val = str_val * 10;  // 힘 * 10
    let weight_str = to_string(weight) + "/" + to_string(max_weight_val);

    // 목표경험치
    let total_exp = get_int(ob, "목표경험치");

    // 분노
    let anger_val = get_int(ob, "분노");

    // 은전
    let money = get_int(ob, "은전");

    // 금전
    let gold_val = get_int(ob, "금전");

    // 출력 시작 (Python은 "☞ 당신이 서 있습니다." 메시지 없음)
    send_line(ob, "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑");
    send_line(ob, "\x1b[0m\x1b[44m\x1b[1m\x1b[37m ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ \x1b[0m\x1b[40m\x1b[37m│");
    send_line(ob, "┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥");

    // 레벨, 나이
    let lv_str = pad_start(int_to_str(level), 6, " ");
    send_line(ob, "│ [레  벨]       [" + lv_str + "] │ [나  이]             " + pad_start(int_to_str(age), 3, " ") + " │");

    // 체력, 성격
    let hp_str = to_string(hp) + "/" + to_string(max_hp);
    let hp_padded = pad15(hp_str);
    send_line(ob, "│ [체  력] " + hp_padded + " │ [성  격] " + pad_any(성격_display, 15) + " │");

    // 힘, 성별
    let att_power = 0;  // TODO: 장비 공격력 계산
    let str_total = att_power + str_val;
    let att_str = pad_start(int_to_str(att_power), 5, " ") + " + " + pad_start(int_to_str(str_val), 6, " ");
    send_line(ob, "│ [  힘  ] " + att_str + " │ [성  별] " + pad_any(성별_val, 15) + " │");

    // 맷집, 소속
    let arm_val = 0;  // TODO: 장비 방어력 계산
    let arm_total = arm_val + armor;
    let arm_str = pad_start(int_to_str(arm_val), 6, " ") + " + " + pad_start(int_to_str(armor), 6, " ");
    send_line(ob, "│ [맷  집] " + arm_str + " │ [소  속] " + pad_any(소속_display, 15) + " │");

    // 민첩, 직위
    let dex_str = pad_start(int_to_str(dex), 15, " ");
    send_line(ob, "│ [민  첩] " + dex_str + " │ [직  위] " + pad_any(직위_display, 15) + " │");

    // 명중, 회피
    let hit_str = pad_start(int_to_str(hit), 15, " ");
    let miss_str = pad_start(int_to_str(miss), 15, " ");
    send_line(ob, "│ [命  中] " + hit_str + " │ [回  避] " + miss_str + " │");

    // 필살, 운
    let crit_str = pad_start(int_to_str(critical), 15, " ");
    let luck_str = pad_start(int_to_str(luck), 15, " ");
    send_line(ob, "│ [必  殺] " + crit_str + " │ [  運  ] " + luck_str + " │");

    // 내공, 배우자
    let mp_str = to_string(mp) + "/" + to_string(max_mp);
    let mp_padded = pad15(mp_str);
    send_line(ob, "│ [내  공] " + mp_padded + " │ [배우자] " + pad_any(배우자_display, 15) + " │");

    // 현경, 소지품
    let exp_str = pad_start(int_to_str(exp), 15, " ");
    let weight_padded = pad15(weight_str);
    send_line(ob, "│ [현  경] " + exp_str + " │ [소지품] " + weight_padded + " │");

    // 목경, 분노
    let total_str = pad_start(int_to_str(total_exp), 15, " ");
    let anger_str = if anger_val >= 100 {
        "\x1b[1;31m" + to_string(anger_val) + "\x1b[0;37m"
    } else {
        to_string(anger_val)
    };
    let anger_padded = "             " + pad_start(anger_str, 3, " ");
    send_line(ob, "│ [목  경] " + total_str + " │ [분  노]" + anger_padded + " │");

    // 구분선
    send_line(ob, "├─────────────┴─────────────┤");

    // 은전
    let money_str = pad_start(int_to_str(money), 40, " ");
    send_line(ob, "│\x1b[0m\x1b[47m\x1b[30m [은  전]    " + money_str + " \x1b[0m\x1b[40m\x1b[37m│");

    // 금전 (있으면)
    if gold_val > 0 {
        let gold_str = pad_start(int_to_str(gold_val), 40, " ");
        send_line(ob, "│\x1b[0m\x1b[43m\x1b[30m [금  전]    " + gold_str + " \x1b[0m\x1b[40m\x1b[37m│");
    }

    // 하단 테두리
    send_line(ob, "┕━━━━━━━━━━━━━━━━━━━━━━━━━━━┙");

    // 추가 메시지 (TODO: 구현 필요)
    // HP/MP 상태 메시지
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

    // 표국보험 (TODO: 구현 필요)
    send_line(ob, "★ 당신의 표국보험은 효력이 없습니다.");

    // 운기조식 상태 (TODO: 구현 필요)
    send_line(ob, "★ 당신은 \x1b[1;32m소주천\x1b[0;37;40m을 하고 있습니다.");

    // 특성치
    let 특성_val = get_int(ob, "특성치");
    if 특성_val > 0 {
        send_line(ob, "★ 당신은 " + to_string(특성_val) + "개의 여유 특성치를 보유하고 있습니다.");
    }

    // 방파 정보 (TODO: 구현 필요)
    // if 소속_val != "" {
    //     let g = GUILD[소속_val];
    //     let 직위명칭 = ...;
    //     let 별호 = get_string(ob, "방파별호");
    //     send_line(ob, "★ 당신" + han_un("당신") + " " + 소속_val + " 문파의 " + 직위명칭 + 별호 + " 입니다.");
    // }
}

fn main(ob, line) {
    cmd(ob, line);
}
