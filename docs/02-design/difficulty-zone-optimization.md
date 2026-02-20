# 난이도존 최적화 설계

## 문제점 (현재 구현)

```
data/map/낙양성/     <- 기본존 (1111개 방)
data/map/낙양성1/    <- 난이도 1 (1111개 방 복사)
data/map/낙양성2/    <- 난이도 2 (1111개 방 복사)
data/map/낙양성3/    <- 난이도 3 (1111개 방 복사)
...
data/mob/낙양성/     <- 기본존 몹
data/mob/낙양성1/    <- 난이도 1 몹 (복사)
data/mob/낙양성2/    <- 난이도 2 몹 (복사)
...
```

**메모리 낭비:** 동일한 정적 데이터가 난이도별로 중복 저장됨

## 제안하는 아키텍처

### 1. 정적 데이터 (공유)

```rust
/// 방 템플릿 - 모든 난이도에서 공유
pub struct RoomTemplate {
    /// 기본 존 이름 (예: "낙양성")
    pub zone: String,
    /// 방 번호/이름
    pub name: String,
    /// 표시 이름
    pub display_name: String,
    /// 설명
    pub description: Vec<String>,
    /// 맵 속성 (전투금지, PK허용 등)
    pub properties: Vec<String>,
    /// 출구 (정적)
    pub exits: HashMap<String, Exit>,
    /// 스폰할 몹 ID 목록
    pub mob_ids: Vec<String>,
    /// 레벨 제한
    pub level_limit: i64,
    pub level_upper: i64,
    /// 안전지대 여부
    pub safe_zone: bool,
    /// PK 허용
    pub pk_allowed: bool,
}

/// 몹 템플릿 - 모든 난이도에서 공유
pub struct MobTemplate {
    /// 몹 이름
    pub name: String,
    /// 기본 레벨
    pub base_level: i64,
    /// 기본 체력
    pub base_hp: i64,
    /// 기본 힘
    pub base_str: i64,
    /// 기본 맷집
    pub base_arm: i64,
    /// 기본 민첩
    pub base_agi: i64,
    /// 설명
    pub desc1: String,
    pub desc2: Vec<String>,
    /// 반응이름
    pub reaction_names: Vec<String>,
    /// 성격
    pub personality: i64,
    /// 몹 타입
    pub mob_type: i64,
    /// 아이템 드롭 목록
    pub items: Vec<String>,
    /// ... 기타 정적 속성
}
```

### 2. 동적 데이터 (난이도별 독립)

```rust
/// 방 인스턴스 - 난이도별로 별도 생성
pub struct RoomInstance {
    /// 공유 템플릿 참조
    pub template: Arc<RoomTemplate>,
    /// 난이도 (0=기본, 1~7=난이도)
    pub difficulty: u8,
    /// 현재 방에 있는 플레이어 (동적)
    pub players: Vec<String>,
    /// 현재 방에 있는 NPC (동적)
    pub npcs: Vec<String>,
    /// 바닥에 떨어진 아이템 (동적)
    pub items: Vec<String>,
    /// 활성화된 몹 인스턴스 ID (동적)
    pub active_mobs: Vec<u64>,
}

/// 몹 인스턴스 - 난이도 적용된 스탯
pub struct MobInstance {
    /// 공유 템플릿 참조
    pub template: Arc<MobTemplate>,
    /// 고유 인스턴스 ID
    pub instance_id: u64,
    /// 난이도
    pub difficulty: u8,
    /// 현재 위치
    pub zone: String,
    pub room: String,
    /// 난이도 적용된 스탯
    pub level: i64,        // = base_level + difficulty_bonus
    pub hp: i64,
    pub max_hp: i64,       // = base_hp * difficulty_multiplier
    pub strength: i64,     // = base_str * difficulty_multiplier
    pub arm: i64,
    pub agility: i64,
    /// 현재 상태
    pub alive: bool,
    pub spawn_time: i64,
    pub death_time: i64,
    pub targets: Vec<String>,
    pub act: i32,
}
```

### 3. 난이도 설정

```rust
/// 난이도별 스탯 배율 설정
pub struct DifficultyConfig {
    /// 레벨 보너스
    pub level_bonus: i64,
    /// 체력 배율
    pub hp_multiplier: f64,
    /// 힘 배율
    pub str_multiplier: f64,
    /// 맷집 배율
    pub arm_multiplier: f64,
    /// 경험치 보너스 배율
    pub exp_multiplier: f64,
    /// 골드 보너스 배율
    pub gold_multiplier: f64,
    /// 아이템 드롭 확률 보너스
    pub drop_bonus: f64,
}

impl DifficultyConfig {
    pub fn get(difficulty: u8) -> Self {
        match difficulty {
            0 => Self { // 기본
                level_bonus: 0,
                hp_multiplier: 1.0,
                str_multiplier: 1.0,
                arm_multiplier: 1.0,
                exp_multiplier: 1.0,
                gold_multiplier: 1.0,
                drop_bonus: 0.0,
            },
            1 => Self { // 난이도 1
                level_bonus: 2000,
                hp_multiplier: 2.5,
                str_multiplier: 2.0,
                arm_multiplier: 2.0,
                exp_multiplier: 1.5,
                gold_multiplier: 1.5,
                drop_bonus: 0.1,
            },
            2 => Self { // 난이도 2
                level_bonus: 4000,
                hp_multiplier: 5.0,
                str_multiplier: 3.5,
                arm_multiplier: 3.5,
                exp_multiplier: 2.0,
                gold_multiplier: 2.0,
                drop_bonus: 0.2,
            },
            // ... 난이도 3~7
            _ => Self::get(0),
        }
    }
}
```

### 4. 캐시 구조

```rust
/// 최적화된 방 캐시
pub struct RoomCache {
    /// 템플릿 캐시: "zone:name" -> RoomTemplate
    templates: HashMap<String, Arc<RoomTemplate>>,
    /// 인스턴스 캐시: "zone:name:difficulty" -> RoomInstance
    instances: HashMap<String, RoomInstance>,
    /// 데이터 디렉토리
    data_dir: PathBuf,
}

impl RoomCache {
    /// 방 가져오기 - 난이도 지정
    pub fn get_room(&mut self, zone: &str, name: &str, difficulty: u8) -> Arc<RwLock<RoomInstance>> {
        let instance_key = format!("{}:{}:{}", zone, name, difficulty);

        // 인스턴스가 있으면 반환
        if let Some(instance) = self.instances.get(&instance_key) {
            return Arc::new(RwLock::new(instance.clone()));
        }

        // 템플릿 로드 (공유)
        let template_key = format!("{}:{}", zone, name);
        let template = self.load_template(&template_key);

        // 새 인스턴스 생성
        let instance = RoomInstance {
            template: template.clone(),
            difficulty,
            players: Vec::new(),
            npcs: Vec::new(),
            items: Vec::new(),
            active_mobs: Vec::new(),
        };

        self.instances.insert(instance_key.clone(), instance);
        // ...
    }
}

/// 최적화된 몹 캐시
pub struct MobCache {
    /// 템플릿 캐시: "zone:filename" -> MobTemplate
    templates: HashMap<String, Arc<MobTemplate>>,
    /// 인스턴스: "zone:room:difficulty" -> Vec<MobInstance>
    instances: HashMap<String, Vec<MobInstance>>,
    /// 다음 인스턴스 ID
    next_instance_id: u64,
}
```

## 메모리 절약 효과

### 기존 방식
```
낙양성 (1111개 방) × 5개 난이도 = 5555개 방 객체
각 방 객체 = 약 1KB
총 메모리 = 5555 × 1KB = 5.5MB
```

### 최적화 방식
```
낙양성 템플릿 (1111개) = 1111 × 0.8KB = 0.9MB
인스턴스 (1111 × 5) = 5555 × 0.2KB = 1.1MB
총 메모리 = 2.0MB (64% 절약)
```

## 이동 명령 처리

```rust
// 기존: "낙양성:42" -> 단일 방
// 개선: "낙양성:42:0" (기본), "낙양성:42:1" (난이도1), ...

fn move_player(player: &mut Player, direction: &Direction) {
    // 플레이어의 현재 난이도 확인
    let difficulty = player.get_difficulty();

    // 현재 방에서 출구 찾기
    let current_room = room_cache.get_room(&player.zone, &player.room, difficulty);
    let exit = current_room.get_exit(direction);

    // 목적지 방 (같은 난이도 유지)
    let dest_room = room_cache.get_room(&exit.zone, &exit.room, difficulty);

    // 이동 처리
    player.zone = exit.zone;
    player.room = exit.room;
    // ...
}
```

## 난이도 진입 처리

```rust
// 난이도존 진입 NPC 또는 포털
fn enter_difficulty_zone(player: &mut Player, target_difficulty: u8) {
    // 레벨 체크
    let min_level = get_min_level_for_difficulty(target_difficulty);
    if player.level < min_level {
        player.send_line("레벨이 부족합니다.");
        return;
    }

    // 난이도 설정
    player.difficulty = target_difficulty;

    // 해당 난이도의 시작 위치로 이동
    let start_room = room_cache.get_room("낙양성", "1", target_difficulty);
    player.move_to_room(start_room);

    player.send_line(&format!("난이도 {}존에 입장했습니다.", target_difficulty));
}
```

## 구현 단계

1. **1단계: RoomTemplate/RoomInstance 분리**
   - 기존 Room 구조체를 템플릿과 인스턴스로 분리
   - RoomCache 수정

2. **2단계: MobTemplate/MobInstance 분리**
   - 기존 Mob 구조체 분리
   - 난이도별 스탯 계산 로직 추가

3. **3단계: 난이도 설정 시스템**
   - DifficultyConfig 구현
   - config 파일에서 난이도 설정 로드

4. **4단계: 플레이어 난이도 추적**
   - Player에 difficulty 필드 추가
   - 이동/전투 시 난이도 반영

5. **5단계: 기존 난이도존 파일 정리**
   - 복사된 map/mob 파일 제거
   - 기본 템플릿만 유지

## 호환성

- 기존 Python 플레이어 데이터와 호환
- 난이도 필드가 없는 경우 0(기본)으로 처리
- 점진적 마이그레이션 가능
