// lib/data/global.rhai - 글로벌 데이터 접근 라이브러리
//
// data/config/*.json에 있는 데이터를 쉽게 접근할 수 있는 래퍼 함수들입니다.
//
// 사용 예시:
//   let skill_data = get_skill("가의신공");
//   let config = get_murim_config("메인설정");
//   let zone = get_map_path("강소성");
//   let all_skills = get_global_keys("skill");

/// skill.json에서 스킬 데이터 가져오기
///
/// 반환 값: 스킬 데이터 (Map 또는 Unit)
fn get_skill_data(name) {
    get_skill(name)
}

/// 스킬 공격 메시지 가져오기
fn get_skill_attacks(name) {
    let skill = get_skill(name);
    if skill == () {
        return [];
    }
    skill["공격"]
}

/// 스크립트 메시지 가져오기
fn get_skill_script(name) {
    let skill = get_skill(name);
    if skill == () {
        return "";
    }
    skill["무공스크립"]
}

/// 스킬 속성 가져오기
fn get_skill_attrs(name) {
    let skill = get_skill(name);
    if skill == () {
        return [];
    }
    skill["속성"]
}

/// 스킬 종류 가져오기
fn get_skill_type(name) {
    let skill = get_skill(name);
    if skill == () {
        return "";
    }
    skill["종류"]
}

/// 스킬 타격률 가져오기
fn get_skill_damage(name) {
    let skill = get_skill(name);
    if skill == () {
        return 0;
    }
    skill["타격률"]
}

/// murim.json에서 설정 가져오기
fn get_murim(key) {
    get_murim_config(key)
}

/// 메인 설정 가져오기
fn get_main_config(key) {
    get_murim_config("메인설정")[key]
}

/// 맵 경로 가져오기
fn get_zone_path(zone) {
    get_map_path(zone)
}

/// 모든 스킬 이름 목록 가져오기
fn list_skills() {
    get_global_keys("skill")
}

/// 모든 맵(존) 이름 목록 가져오기
fn list_zones() {
    let path_data = get_global("mappath");
    if path_data == () {
        return [];
    }
    let dir_settings = path_data["디렉토리설정"];
    if dir_settings == () {
        return [];
    }

    // Map을 Array로 변환
    let keys = [];
    // TODO: Map의 키들을 배열로 변환
    keys
}

/// 감정 표현 로드
fn get_emotion(name) {
    let emotion = get_global_key("emotion", name);
    if emotion == () {
        return "";
    }
    emotion
}

/// 도움말 로드
fn get_help(topic) {
    let help = get_global_key("help", topic);
    if help == () {
        return "";
    }
    help
}

/// 별명 로드
fn get_nickname(name) {
    let nickname = get_global_key("nickname", name);
    if nickname == () {
        return "";
    }
    nickname
}

/// data 파일들이 로드되었는지 확인
fn check_globals() {
    let files = list_globals();
    let count = len(files);
    print("[GLOBAL] Loaded " + count + " config files");
    count > 0
}

/// 글로벌 데이터 리로드
fn reload_config(file) {
    let result = reload_global(file);
    if result {
        print("[GLOBAL] Reloaded: " + file);
    } else {
        print("[GLOBAL] Failed to reload: " + file);
    }
    result
}

/// 모든 설정 리로드
fn reload_all_configs() {
    let count = reload_all_globals();
    print("[GLOBAL] Reloaded " + count + " config files");
}
