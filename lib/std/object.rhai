// lib/std/object.rhai - Standard Object Library
//
// 기본 오브젝트 동작을 정의합니다.
// Driver의 Object struct와 상호작용합니다.

/// Object 생성 시 호출
///
/// Driver에서 새 Object를 생성할 때 호출됩니다.
/// data/config/*.json 에서 속성을 로드합니다.
fn create() {
    // 기본 구현: 아무것도 하지 않음
    // 필요하다면 각 mudlib에서 override
}

/// 초기화 - Object가 다른 Object를 만났을 때
///
/// 예: 플레이어가 방에 들어왔을 때 방의 init()이 호출됨
fn init() {
    // 기본 구현: 아무것도 하지 않음
}

/// 주기적 리셋 - 일정 주기로 호출됨
///
/// 몹 재생성, 아이템 리스폰 등에 사용
fn reset() {
    // 기본 구현: 아무것도 하지 않음
}

/// 이동/소멸 전에 호출
///
/// 반환값: true면 이동 허용, false면 소멸
fn move_or_destruct(destination) {
    true  // 기본: 이동 허용
}

/// 소멸 전 정리
fn clean_up() {
    // 기본 구현: 소멸하지 않음
    false
}

/// 이름 가져오기 (한국어 조사 포함)
fn get_name() {
    get_string(this, "이름")
}

/// 이름을 조사와 함께 반환
fn get_name_with_particle() {
    let name = get_name();
    let particle = han_iga(name);
    name + particle
}

/// 설명 가져오기
fn get_description() {
    get_string(this, "설명")
}

/// 환경 설정 확인
fn get_environment() {
    // Driver efun 필요
    environment(this)
}

/// 인벤토리 목록
fn get_inventory() {
    // Driver efun 필요
    all_inventory(this)
}

/// 속성 설정
fn set_attr(key, value) {
    // Driver efun 호출
    set_attr(this, key, value)
}

/// 속성 가져오기
fn get_attr(key) {
    // Driver efun 호출
    get_attr(this, key)
}

/// 정수 속성
fn get_int_attr(key) {
    // Driver efun 호출
    get_int(this, key)
}

/// 문자열 속성
fn get_str_attr(key) {
    // Driver efun 호출
    get_string(this, key)
}

/// 반응 이름 확인
fn responds_to(name) {
    let names = get_str_attr("반응이름");
    let my_name = get_name();

    if name == my_name {
        return true;
    }

    if names != "" {
        // names는 배열일 수 있음
        // TODO: 배열 처리 필요
    }

    false
}
