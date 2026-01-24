// lib/std/item.rhai - Standard Item Library
//
// 아이템 동작을 정의합니다.
// 장착 가능한 장비, 소모품, 기타 아이템 등을 지원합니다.

/// 아이템 생성 시 호출
fn create() {
    print("[ITEM] create() called");
    // TODO: data/item/*.json에서 아이템 데이터 로드
}

/// 아이템 사용
///
/// 사용 시 효과를 적용합니다.
fn use_item(user) {
    print("[ITEM] use_item() by " + get_string(user, "이름"));

    let item_type = get_string(this, "종류");

    if item_type == "음식" {
        // TODO: HP/MP 회복 등
        consume();
    } else if item_type == "장비" {
        equip(user);
    } else if item_type == "약초" {
        // TODO: 제조 시 재료로 사용
    }
}

/// 아이템 장착
fn equip(user) {
    print("[ITEM] equip() by " + get_string(user, "이름"));

    let slot = get_string(this, "장비부위");
    // TODO: 해당 슬롯에 장착
    // TODO: 스탯 보너스 적용
}

/// 아이템 탈착
fn unequip(user) {
    print("[ITEM] unequip() by " + get_string(user, "이름"));

    // TODO: 스탯 보너스 제거
}

/// 아이템 소비 (소모품)
fn consume() {
    print("[ITEM] consume()");

    // TODO: 효과 적용 후 아이템 제거
    // remove(this);
}

/// 아이템 이름 가져오기
fn get_name() {
    get_string(this, "이름")
}

/// 아이템 설명 가져오기
fn get_description() {
    get_string(this, "설명")
}

/// 아이템 종류 확인
fn get_type() {
    get_string(this, "종류")
}

/// 아이템 무게 확인
fn get_weight() {
    get_int(this, "무게")
}

/// 아이템 가격 확인
fn get_value() {
    get_int(this, "가격")
}

/// 아이템 타입 확인
fn is_item() {
    true
}

/// 무기인지 확인
fn is_weapon() {
    get_string(this, "종류") == "무기"
}

/// 방어구인지 확인
fn is_armor() {
    let kind = get_string(this, "종류");
    kind == "방패" || kind == "투구" || kind == "갑옷"
        || kind == "신발" || kind == "장갑" || kind == "허리띠"
}
