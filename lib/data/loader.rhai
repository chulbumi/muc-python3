// lib/data/loader.rhai - Data Loader Library
//
// JSON 데이터 로딩을 위한 래퍼 함수들을 제공합니다.
/// 글로벌 데이터 캐시를 사용하여 data/config/*.json 데이터에 접근합니다.

/// 스킬 데이터 로드 (글로벌 캐시 사용)
fn load_skill(name) {
    let data = get_skill(name);

    if data == () {
        print("[LOADER] Skill not found: " + name);
        return ();
    }

    data
}

/// 무공 스킬 데이터 로드
fn get_mugong_info(name) {
    load_skill(name)
}

/// 스킬 공격 메시지 가져오기
fn get_skill_attacks(name) {
    let skill = load_skill(name);
    if skill == () {
        return [];
    }
    skill["공격"]
}

/// 스킬 스크립트 메시지 가져오기
fn get_skill_script_msg(name) {
    let skill = load_skill(name);
    if skill == () {
        return "";
    }
    skill["무공스크립"]
}

/// 스킬 속성 가져오기
fn get_skill_attributes(name) {
    let skill = load_skill(name);
    if skill == () {
        return [];
    }
    skill["속성"]
}

/// 스킬 실패 메시지 가져오기
fn get_skill_fail(name) {
    let skill = load_skill(name);
    if skill == () {
        return "";
    }
    skill["실패"]
}

/// 스킬 종류 가져오기
fn get_skill_kind(name) {
    let skill = load_skill(name);
    if skill == () {
        return "";
    }
    skill["종류"]
}

/// 무림 설정 로드
fn load_murim_info() {
    get_global("murim")
}

/// 메인 설정 가져오기
fn get_main_setting(key) {
    let main_config = get_murim_config("메인설정");
    if main_config == () {
        return "";
    }
    main_config[key]
}

/// 맵 경로 가져오기
fn get_zone_directory(zone) {
    get_map_path(zone)
}

/// 모든 스킬 이름 목록
fn list_all_skills() {
    get_global_keys("skill")
}

/// 스킬 존재 확인
fn has_skill(name) {
    has_global_key("skill", name)
}

/// 감정 표현 로드
fn load_emotion(name) {
    get_global_key("emotion", name)
}

/// 도움말 로드
fn load_help(topic) {
    get_global_key("help", topic)
}

/// 별명 로드
fn load_nickname(name) {
    get_global_key("nickname", name)
}

/// 설정 값 가져오기 (여러 파일에서 검색)
fn get_config_value(file, key) {
    get_global_key(file, key)
}

/// 글로벌 데이터 상태 확인
fn check_data_status() {
    let files = list_globals();
    let count = len(files);
    print("[DATA] " + count + " config files loaded");
    count
}

/// 특정 설정 리로드
fn refresh_config(file) {
    reload_global(file)
}

/// 모든 설정 리로드
fn refresh_all_configs() {
    reload_all_globals()
}
