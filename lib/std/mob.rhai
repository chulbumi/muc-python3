// lib/std/mob.rhai - Standard Mob Library
//
// 몹(NPC) 동작을 정의합니다.
// Body를 상속받아 몹 특정 기능을 추가합니다.

/// 몹 생성 시 호출
fn create() {
    print("[MOB] create() called");
    // TODO: data/mob/*.json에서 데이터 로드
}

/// 주기적 리셋
///
/// 몹 재생성, 아이템 리스폰 등에 사용
fn reset() {
    print("[MOB] reset() called");

    // TODO: 죽은 몹 리스폰, 아이템 재생성 등
}

/// 주기적 호출 (heart_beat)
///
/// AI 행동: 이동, 공격, 대화 등
fn heart_beat() {
    // TODO: AI 행동 패턴 구현
    // - 공격 대상이 있으면 공격
    // - 랜덤 이동
    // - 대화 출력
}

/// 공격받았을 때 호출
fn on_attacked(attacker) {
    print("[MOB] on_attacked() by " + get_name(attacker));

    // TODO: 반격, 도망, 동료 호출 등
}

/// 죽었을 때 호출
fn on_die(killer) {
    print("[MOB] on_die() by " + get_name(killer));

    // TODO: 경험치 보상, 아이템 드롭 등
}

/// 대화 시작
fn start_talk(player) {
    print("[MOB] start_talk() with " + get_name(player));

    // TODO: 대화 시스템 연동
}

/// 이름 가져오기
fn get_name(mob) {
    get_string(mob, "이름")
}

/// 몹 타입 확인
fn is_mob() {
    true
}
