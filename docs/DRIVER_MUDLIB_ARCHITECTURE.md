# Driver/Mudlib 분리 아키텍처 설계

## 핵심 원칙 (Core Principles)

FluffOS/MudOS 기준 driver/mudlib 분리:
- **Driver**: 메커니즘 (Mechanisms) - "무엇을 할 수 있는가"
- **Mudlib**: 정책 (Policies) - "어떻게 동작하는가"

```
┌─────────────────────────────────────────────────────────────┐
│                    Driver (Rust)                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Object     │  │   Network    │  │    Data      │     │
│  │   Struct     │  │   I/O        │  │    Loader    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│         ↓                  ↓                  ↓             │
│    attr/map          Player            JSON Files          │
│    env/objs          Connection        data/config/*.json │
└─────────────────────────────────────────────────────────────┘
                            │
                    ┌───────▼────────┐
                    │   Rhai Engine  │ ← Script 실행
                    └────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼──────┐   ┌────────▼─────┐   ┌────────▼─────┐
│   cmds/     │   │    lib/      │   │   objs/       │
│  (Commands)  │   │  (Library)   │   │ (Templates)   │
└──────────────┘   └──────────────┘   └───────────────┘
```

## 1. Object Core 구조 (Driver 제공)

### Python objs/object.py → Rust Driver

```python
# Python - objs/object.py
class Object:
    def __init__(self):
        self.attr = {}      # 영구 속성
        self.temp = {}      # 임시 속성
        self.env = None     # 상위 오브젝트
        self.objs = []      # 하위 오브젝트 리스트
```

```rust
// Rust - src/object/base.rs
pub struct Object {
    /// 영구 속성 (데이터 로드에서 초기화)
    pub attr: HashMap<String, Value>,
    /// 임시 속성 (런타임만 저장)
    pub temp: HashMap<String, Value>,
    /// 상위 오브젝트 (Environment) - weak reference
    pub env: Option<Weak<Mutex<Object>>>,
    /// 하위 오브젝트 (Contains) - Arc 참조
    pub objs: Vec<Arc<Mutex<Object>>>,
}

// Driver 제공 efuns:
impl Object {
    // 기본 조작
    pub fn insert(&mut self, obj: Arc<Mutex<Object>>);  // 하위에 추가
    pub fn append(&mut self, obj: Arc<Mutex<Object>>);  // 맨 뒤에 추가
    pub fn remove(&mut self, obj: &Arc<Mutex<Object>>);  // 하위에서 제거

    // 검색
    pub fn find_obj_name(&self, name: &str, order: usize) -> Option<Arc<Mutex<Object>>>;
    pub fn find_obj_inven(&self, name: &str, order: usize) -> Option<Arc<Mutex<Object>>>;
    pub fn find_obj_in_use(&self, name: &str, order: usize) -> Option<Arc<Mutex<Object>>>;

    // 속성 접근
    pub fn get(&self, key: &str) -> Value;
    pub fn set(&mut self, key: &str, value: Value);
    pub fn get_int(&self, key: &str) -> i64;
    pub fn get_string(&self, key: &str) -> String;

    // 관계
    pub fn get_env(&self) -> Option<Arc<Mutex<Object>>>;      // 상위
    pub fn get_inventory(&self) -> Vec<Arc<Mutex<Object>>>;  // 하위 전체
}
```

## 2. 상속 계층 구조 (Mudlib 정의)

Python의 상속을 Rhai mudlib로 구현:

```
Object (Driver)
    ↓
Body (Driver - stats/combat 기본)
    ↓
    ├── Player (Mudlib - 접속/명령어)
    └── Mob (Mudlib - AI/행동패턴)
```

```
Object (Driver)
    ↓
    ├── Item (Mudlib - 아이템 효과)
    ├── Room (Mudlib - 방 설명/이벤트)
    └── Box (Mudlib - 컨테이너)
```

### Body (Driver) - Stats/Combat 기본

```rust
// src/player/body.rs (기존 구현 활용)
pub struct Body {
    pub base: Object,     // Object 기본
    pub hp: i64,          // 체력
    pub max_hp: i64,
    pub mp: i64,          // 내공
    pub max_mp: i64,
    pub str: i64,         // 힘
    pub dex: i64,         // 민첩
    pub armor: i64,       // 방어력
    pub attpower: i64,    // 공격력
    pub act: ActState,    // 상태 (STAND, FIGHT, etc.)
}
```

### Player vs Mob (Mudlib 분리)

```rhai
// lib/player.rhai - Player 기본 동작
fn create() {
    // 플레이어 생성 시 초기화
}

fn init() {
    // 다른 오브젝트 만났을 때
}

fn heart_beat() {
    // 1초마다 호출 (HP/MP 회복, 중독 처리 등)
}

fn on_login() {
    // 로그인 시
}

fn on_logout() {
    // 로그아웃 시 (저장 등)
}
```

```rhai
// lib/mob.rhai - Mob 기본 동작
fn create() {
    // 몹 생성 시 데이터 로드
}

fn reset() {
    // 주기적 리셋 (재생성)
}

fn heart_beat() {
    // AI 행동 (이동, 공격, 대화)
}

fn on_attacked(attacker) {
    // 공격받았을 때 반응
}
```

## 3. 데이터 로딩 구조 (Driver + Mudlib)

### data/config/*.json - 게임 데이터

```
data/config/
├── skill.json     # 무공 스킬 데이터
├── itempath.json  # 아이템 경로
├── mobpath.json   # 몹 경로
├── mappath.json   # 맵 경로
├── murim.json     # 무림 정보
├── cmd.json       # 명령어 설정
└── ...
```

### 데이터 로딩 흐름

```
┌─────────────┐      JSON      ┌──────────────┐
│   Player   │ ─────────────→ │ data/config │
└─────────────┘                │  *.json     │
       ↓                         └──────────────┘
   "스크립트 로드"                        ↑
       ↓                                │
┌─────────────┐    Driver API     ┌──────────────┐
│  Mudlib     │ ─────────────────→ │ Data Loader  │
│  (Rhai)     │                   │  (Rust)      │
└─────────────┘                   └──────────────┘
```

**Driver 제공 efuns:**
```rust
pub fn load_json(path: &str) -> Result<Value, Box<dyn Error>>;
pub fn get_item_data(name: &str) -> Option<Value>;
pub fn get_mob_data(name: &str) -> Option<Value>;
pub fn get_skill_data(name: &str) -> Option<Value>;
pub fn get_room_data(name: &str) -> Option<Value>;
```

**Mudlib 사용:**
```rhai
// lib/item.rhai
fn load_item(name) {
    let data = get_item_data(name);
    if data == () {
        return null;
    }

    let item = Object();
    item.set("이름", data["이름"]);
    item.set("종류", data["종류"]);
    // ... 속성 설정

    return item;
}
```

## 4. 핵심 Efunc 목록 (Driver → Mudlib)

### Object 조작
```rust
// 기본
insert(obj, target)           // target에 obj를 넣기
append(obj, target)           // target에 obj를 추가 (맨 뒤)
remove(obj, container)        // container에서 obj 제거
move_object(obj, destination) // obj를 destination으로 이동

// 검색
find_obj_name(container, name, order)  // 이름으로 찾기
find_obj_inven(container, name, order)  // 인벤토리에서 찾기
present(name, env)                    // env에서 이름으로 찾기

// 정보
environment(obj)              // obj의 env 반환
all_inventory(obj)            // obj의 objs 전체
this_player()                 // 현재 플레이어
```

### 속성 접근
```rust
get_attr(obj, key)            // 속성 가져오기
set_attr(obj, key, value)     // 속성 설정
get_int(obj, key)             // int 속성
get_string(obj, key)          // string 속성
check_attr(obj, key, attr)    // 속성에 attr이 있는지 확인
```

### 플레이어/몹 전용
```rust
// 플레이어
send_line(player, msg)        // 플레이어에게 메시지
send_room(room, msg)          // 방에 메시지
players_in_room(room)         // 방에 있는 플레이어 목록

// 스킬 관련
get_skill_list(obj)           // 스킬 목록
get_skill_level(obj, skill)   // 스킬 레벨
add_skill_exp(obj, skill, exp) // 스킬 경험치 추가
```

## 5. objs/ 디렉토리 Mudlib 구조

```
objs/                          # Mudlib (Rhai)
├── lib/                       # 라이브러리 (공통 코드)
│   ├── std/
│   │   ├── object.rhai       # Object 기본 동작
│   │   ├── body.rhai         # Body 기본 동작
│   │   ├── player.rhai       # Player 기본 동작
│   │   ├── mob.rhai          # Mob 기본 동작
│   │   ├── item.rhai         # Item 기본 동작
│   │   ├── room.rhai         # Room 기본 동작
│   │   └── container.rhai    # Box/Container 기본 동작
│   └── inherit/
│       ├── weapon.rhai       # 무기 상속
│       ├── armor.rhai        # 방어구 상속
│       └── ...
│
├── data/                      # 데이터 로드 함수
│   ├── item_loader.rhai      # 아이템 데이터 로딩
│   ├── mob_loader.rhai       # 몹 데이터 로딩
│   ├── skill_loader.rhai     # 스킬 데이터 로딩
│   └── map_loader.rhai       # 맵 데이터 로딩
│
├── templates/                 # 템플릿 (factory 함수)
│   ├── create_item.rhai      # 아이템 생성
│   ├── create_mob.rhai       # 몹 생성
│   ├── create_room.rhai      # 방 생성
│   └── create_player.rhai    # 플레이어 생성
│
└── instances/                 # 동적 인스턴스 (선택적)
    └── ... (실행 시 생성되는 인스턴스)
```

## 6. Python → Rust+Rhai 변환 매핑

| Python (objs/) | 역할 | Rust Driver | Rhai Mudlib |
|---------------|------|-------------|--------------|
| `object.py` | Object 기본 | `src/object/base.rs` | `lib/std/object.rhai` |
| `body.py` | Stats/Combat | `src/player/body.rs` | `lib/std/body.rhai` |
| `player.py` | 플레이어 | `src/player/player.rs` | `lib/std/player.rhai` |
| `mob.py` | NPC | `src/player/mob.rs` | `lib/std/mob.rhai` |
| `room.py` | 방 | `src/world/room.rs` | `lib/std/room.rhai` |
| `item.py` | 아이템 | `src/world/item.rs` | `lib/std/item.rhai` |
| `box.py` | 컨테이너 | `src/world/box.rs` | `lib/std/container.rhai` |
| `skill.py` | 스킬 | `src/world/skill.rs` | `lib/skill/` |
| `event.py` | 이벤트 | `src/world/event.rs` | `lib/event/` |

## 7. 구현 우선순위

### Phase 1: 핵심 Object 시스템 (Driver)
- [ ] Object struct (env/objs, attr/temp)
- [ ] insert/remove/move_object efuns
- [ ] find_obj_name 검색 efuns
- [ ] get/set/get_int/get_string efuns

### Phase 2: 데이터 로더 (Driver)
- [ ] JSON 로드 시스템 (data/config/*.json)
- [ ] get_item_data, get_mob_data efuns
- [ ] get_skill_data, get_room_data efuns

### Phase 3: Mudlib 기본 (Rhai)
- [ ] lib/std/object.rhai - 기본 동작
- [ ] lib/std/player.rhai - 플레이어 동작
- [ ] lib/std/mob.rhai - 몹 동작
- [ ] lib/data/item_loader.rhai - 아이템 생성

### Phase 4: Apply 시스템 연동
- [ ] Master Object의 create/reset/init 호출
- [ ] Object 생성 시 mudlib create() 호출
- [ ] 주기적 mudlib reset() 호출
- [ ] Object 만날 때 mudlib init() 호출

## 8. 핵심 파일 정리

### 필수 Driver 파일
```
src/
├── object/
│   └── base.rs          # Object struct (env/objs/attr)
├── player/
│   ├── body.rs          # Body (stats/combat)
│   ├── player.rs        # Player (connection)
│   └── mob.rs           # Mob struct
├── world/
│   ├── room.rs          # Room struct
│   ├── item.rs          # Item struct
│   └── loader.rs        # JSON data loader
└── script/
    └── mod.rs           # Rhai engine + efuns
```

### 필수 Mudlib 파일
```
lib/
├── std/
│   ├── object.rhai      # create(), init(), reset()
│   ├── player.rhai      # on_login(), on_logout()
│   ├── mob.rhai         # heart_beat() AI
│   └── loader.rhai      # 데이터 로드 wrapper
└── data/
    └── loader.rhai      # get_item_data() 등 wrapper
```
