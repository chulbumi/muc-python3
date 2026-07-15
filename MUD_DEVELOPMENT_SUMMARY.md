# Rust MUD 개발 진행 상황 요약

## 프로젝트 개요

Python으로 개발 중이던 MUD (Multi-User Dungeon) 게임을 Rust로 재작성 중입니다.
- JSON 파일로부터 게임 데이터를 로딩합니다 (.map, .mob, .item 파일은 사용하지 않음)
- 서버 포트: 9999 (비교용 Python 서버: 9900)
- 로그인: 아이디 "멍멍멍", 비밀번호 "멍멍멍"
- 한글 UTF-8 인코딩 (EUC-KR은 현재 미지원)

## 완료된 기능

### 1. 모듈 구조 (src/world/)

#### mob.rs - 몬스터 로딩 및 스폰 시스템
```rust
// 주요 구조
pub struct RawMobData {
    pub name: String,        // "이름"
    pub zone: String,        // "존이름"
    pub desc1: String,       // "설명1" - 방에 표시될 설명
    pub locations: Vec<i64>, // "위치" - 스폰되는 방 번호들
    // ... 기타 필드
}

// 주요 함수 (맵 기준 on-demand)
// - 몹은 방 입장 시 맵의 `몹` 목록만 로드 (존 전체 preload 제거)
MobCache::spawn_mobs_for_room(zone, room, mob_ids)  // 맵의 몹 ID 기준 스폰, 필요 시 로드
MobCache::get_mobs_in_room(zone, room)  // 방의 활성화된 몹 반환
```

#### item.rs - 아이템 로딩 시스템
```rust
pub struct RawItemData {
    pub name: String,
    pub zone: String,
    pub item_type: String,
    // ...
}
```

#### room.rs - 방 로딩 및 캐싱
```rust
pub struct Room {
    pub display_name: String,  // "이름"
    pub zone: String,          // "존이름"
    pub description: Vec<String>,  // "설명"
    pub exits: HashMap<Direction, Exit>,  // "출구"
    pub mob_ids: Vec<String>,  // "몹" — 이 방에 스폰할 몹 ID (입장 시에만 로드)
}

pub struct RoomCache {
    rooms: HashMap<String, Arc<RwLock<Room>>>,  // "존:방" 키로 캐싱
    data_dir: PathBuf,  // "data/map"
}
```

#### mod.rs - WorldState 통합 관리
```rust
pub struct WorldState {
    pub player_positions: HashMap<String, PlayerPosition>,
    pub room_cache: RoomCache,
    pub mob_cache: MobCache,
    pub item_cache: ItemCache,
}

// 전역 접근
pub fn get_world_state() -> &'static RwLock<WorldState>

// 플레이어 위치 이동
world.move_player(player_name, direction) -> Result<(String, i64), String>
```

### 2. 네트워크 (src/network/)

#### mod.rs - 텍스트 처리
- 클라이언트 입출력은 **UTF-8** 기준 (EUC-KR 미지원)

#### client.rs - 로그인 및 게임 명령 처리
- `complete_login_and_enter_game()`: 로그인 완료 후 게임 입장
- `show_room_to_player()`: WorldState 사용하여 방 표시
- `handle_movement()`: WorldState 사용하여 이동 처리

### 3. 커맨드 (src/command/commands/)

#### movement.rs - 이동 명령
```rust
// 한국어 방향 명령 처리
"북" | "ㅂ" => Direction::North
"남" | "ㄴ" => Direction::South
"동" | "ㄷ" => Direction::East
"서" | "ㅅ" => Direction::West
"위" | "ㅇ" => Direction::Up
"아래" | "ㅁ" => Direction::Down
```

## 데이터 파일 구조

```
data/
├── map/           # 방 데이터
│   └── 낙양성/
│       ├── 1.json
│       ├── 5.json
│       └── ...
├── mob/           # 몬스터 데이터
│   └── 낙양성/
│       ├── 1.json        # 무기상 (위치: 48)
│       ├── 1-1.json      # 쇠종 (위치: 4000)
│       └── ...
└── item/          # 아이템 데이터
    └── ...
```

### JSON 예시 (방 데이터) — 맵이 `몹` 보유, 입장 시에만 해당 몹 로드
```json
{
    "맵정보": {
        "이름": "하남성낙양",
        "존이름": "낙양성",
        "설명": ["설명1", "설명2", ...],
        "맵속성": ["사용자전투금지"],
        "출구": ["북 35", "동 2", "남 5", "서 45", "위 4000", "아래 5000"],
        "몹": ["1", "1-1"]
    }
}
```
※ `몹`은 mob의 `위치`를 map에 반영해 둔 값. map2/map3, mob2/mob3 제외.

## 테스트 방법

### 1. 서버 시작
```bash
cargo run --bin murim_server
```

### 2. 접속 테스트 (Python)
```python
import socket

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(('localhost', 9999))

# 로그인 (UTF-8 인코딩, EUC-KR 미지원)
sock.sendall("멍멍멍\r\n".encode('utf-8'))
sock.sendall("멍멍멍\r\n".encode('utf-8'))
sock.sendall(b"1\r\n")  # 게임 입장

# 이동
sock.sendall("남\r\n".encode('utf-8'))  # 남쪽 이동
sock.sendall("봐\r\n".encode('utf-8'))  # 주변 보기
```

### 3. 테스트 스크립트
```bash
# 로그인 및 이동 테스트
python3 test/test_login.py

# 몹 스폰 테스트 (room 4000)
python3 test/test_mob_spawn.py
```

## 현재 작동 방식

1. **로그인 플로우**
   - 클라이언트 접속 → 로고 표시 → 이름 입력 → 비밀번호 입력 → 공지사항 → 게임 입장
   - 게임 입장 시 `PlayerPosition::start()`로 낙양성:1에 위치

2. **방 표시**
   - WorldState에서 플레이어 위치 조회
   - RoomCache에서 방 데이터 가져오기
   - 방 이름, 설명, 출구, 몹 표시

3. **이동 시스템**
   - 방향 입력 → WorldState.move_player() 호출
   - 현재 방의 출구 확인 → 새 위치 설정
   - 새 방에서 몹 스폰 → 방 표시

4. **몹 스폰**
   - 방 입장 시 `spawn_mobs_for_room()` 호출
   - 몹 데이터의 `locations`에 방 번호가 포함된 몹들 스폰
   - 이미 스폰된 몹은 중복 스폰 방지

## 해결한 주요 이슈

### 1. 방 데이터가 로딩되지 않음
**문제**: "알 수 없는 곳입니다." 메시지 표시
**해결**: `WorldState::initialize()`에서 `preload_zone("낙양성")` 호출

### 2. 이동이 작동하지 않음
**문제**: 이동 명령 입력 시 아무 반응 없음
**해결**: `handle_movement()` 함수를 WorldState 기반으로 재작성

### 3. 몹이 표시되지 않음
**문제**: 방에 몹이 있어도 표시되지 않음
**해결**: `spawn_mobs_for_room()`이 이동 시 호출되도록 수정

## 추후 작업 방향

1. **전투 시스템**: 몹과의 전투 구현
2. **아이템 시스템**: 아이템 획득, 장착, 사용
3. **인벤토리**: 인벤토리 관리
4. **스크립트**: Rhai 스크립트와 게임 로직 연동
5. **저장/로드**: 플레이어 데이터 영구 저장

## 서버 상태

- 현재 실행 중: `cargo run --bin murim_server`
- 포트: 9999
- 로그인: 멍멍멍 / 멍멍멍
- 시작 위치: 낙양성:1
- 몹 스폰 확인 위치: 낙양성:4000 (위로 이동)
