// lib/std/room.rhai - Standard Room Library
//
// 방(맵) 동작을 정의합니다.
/// 플레이어가 있는 장소를 나타냅니다.

/// 방 생성 시 호출
fn create() {
    print("[ROOM] create() called");
    // TODO: data/map/*.json에서 맵 데이터 로드
}

/// 초기화 - 누군가 방에 들어왔을 때
fn init() {
    print("[ROOM] init() called");
    // TODO: 입장 메시지, 이벤트 트리거 등
}

/// 주기적 리셋
fn reset() {
    print("[ROOM] reset() called");
    // TODO: 아이템 리스폰, 이벤트 초기화 등
}

/// 방 설명 가져오기
fn get_description() {
    get_string(this, "설명")
}

/// 방 이름 가져오기
fn get_name() {
    get_string(this, "이름")
}

/// 출구 정보 가져오기
fn get_exits() {
    // TODO: 방의 출구 정보 반환
    // data/config/mappath.json 참조
    get_attr(this, "출구")
}

/// 방에 있는 플레이어 목록
fn get_players() {
    // TODO: environment(this)에서 플레이어 필터링
    []
}

/// 방에 있는 몹 목록
fn get_mobs() {
    // TODO: environment(this)에서 몹 필터링
    []
}

/// 방에 있는 아이템 목록
fn get_items() {
    // TODO: environment(this)에서 아이템 필터링
    []
}

/// 방 메시지 전송 (방에 있는 모두에게)
fn send_room(msg) {
    let players = get_players();
    // TODO: 각 플레이어에게 메시지 전송
    print("[ROOM] broadcast: " + msg);
}

/// 방 타입 확인
fn is_room() {
    true
}

/// 존(Zone) 이름 가져오기
fn get_zone() {
    get_string(this, "존")
}
