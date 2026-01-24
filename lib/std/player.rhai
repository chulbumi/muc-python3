// lib/std/player.rhai - Standard Player Library
//
// 플레이어 동작을 정의합니다.
// Body를 상속받아 플레이어 특정 기능을 추가합니다.

/// 플레이어 생성 시 호출
///
/// 새 플레이어 캐릭터 생성 시 호출됩니다.
fn create() {
    // 기본 속성 초기화
    print("[PLAYER] create() called");
}

/// 로그인 시 호출
fn on_login() {
    print("[PLAYER] on_login() called");

    // TODO: 환영 메시지, 초기 위치 설정 등
}

/// 로그아웃 시 호출
fn on_logout() {
    print("[PLAYER] on_logout() called");

    // TODO: 플레이어 데이터 저장
}

/// 주기적 호출 (heart_beat)
///
/// 1초마다 호출됩니다. HP/MP 회복, 상태 처리 등을 수행합니다.
fn heart_beat() {
    // HP/MP 회복
    let hp = get_int(this, "체력");
    let max_hp = get_int(this, "최고체력");

    if hp < max_hp {
        let heal = 1; // 기본 회복량
        let new_hp = hp + heal;
        set_attr(this, "체력", min(new_hp, max_hp));
    }

    // MP 회복
    let mp = get_int(this, "내공");
    let max_mp = get_int(this, "최고내공");

    if mp < max_mp {
        let heal = 1; // 기본 회복량
        let new_mp = mp + heal;
        set_attr(this, "내공", min(new_mp, max_mp));
    }
}

/// 메시지 전송
fn send_message(msg) {
    send_line(this, msg);
}

/// HP 상태 문자열 가져오기
fn get_hp_string() {
    let hp = get_int(this, "체력");
    let max_hp = get_int(this, "최고체력");

    if max_hp <= 0 {
        return "";
    }

    let ratio = (hp * 100) / max_hp;

    if ratio <= 10 {
        return "\x1b[31m━━━━━━━━━━\x1b[37m";  // 빨강 (치명적)
    } else if ratio <= 30 {
        return "\x1b[31m━━━\x1b[37m━━━━━━━\x1b[37m";  // 빨강 (위험)
    } else if ratio <= 50 {
        return "\x1b[33m━━━━\x1b[37m━━━━━━\x1b[37m";  // 노랑 (경고)
    } else if ratio <= 70 {
        return "\x1b[33m━━━━━\x1b[37m━━━━━\x1b[37m";  // 노랑
    } else if ratio <= 90 {
        return "\x1b[32m━━━━━━━\x1b[37m━━━\x1b[37m";  // 초록 (양호)
    } else {
        return "\x1b[32m━━━━━━━━━━\x1b[37m";  // 초록 (풀체력)
    }
}

/// 경험치 추가
///
/// 경험치를 추가하고 레벨업 확인을 합니다.
fn add_exp(amount) {
    let current = get_int(this, "현재경험치");
    let level = get_int(this, "레벨");

    set_attr(this, "현재경험치", current + amount);

    // 레벨업 확인
    let total_needed = get_exp_needed(level);
    if get_int(this, "현재경험치") >= total_needed {
        level_up();
    }
}

/// 레벨업
fn level_up() {
    let level = get_int(this, "레벨");
    set_attr(this, "현재경험치", 0);
    set_attr(this, "레벨", level + 1);

    // HP 증가
    let hp_gain = random(25, 35);
    let max_hp = get_int(this, "최고체력");
    set_attr(this, "최고체력", max_hp + hp_gain);
    set_attr(this, "체력", get_int(this, "최고체력"));

    // 맷집 증가
    let armor = get_int(this, "맷집");
    set_attr(this, "맷집", armor + 1);

    print("[PLAYER] level_up() - Level " + (level + 1));
}

/// 다음 레벨 필요 경험치 계산
fn get_exp_needed(level) {
    let c = (((level * level) / 3) + 30) * (level + 4);
    if c < 1 {
        return 1;
    }
    return min(c, 999999999);
}

/// 최소값 함수
fn min(a, b) {
    if a < b { a } else { b }
}
