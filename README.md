# MUD Server (Rust/Rhai)

Python MUD 서버의 관찰 가능한 동작을 Rust 게임 엔진과 Rhai 명령 스크립트로
마이그레이션한 프로젝트입니다. Python 원본은 호환성 비교 기준으로 함께 보존합니다.

## 현재 상태

- Python 원본의 관찰 가능한 동작을 기준으로 Rust/Rhai 구현을 계속 비교·이관하고
  있습니다. 완료 여부는 명령 수나 파일 수가 아니라 같은 상태에서의 실제 출력과
  상태 변화 비교로 판정합니다.
- `cmds/`에는 공개 명령뿐 아니라 비공개 hook과 helper를 포함한 Rhai 스크립트가
  있습니다. 이들은 수정 후 서버 재시작 없이 반영됩니다.
- Python과의 실제 동작 차이가 발견되면 해당 범위를 다시 미완료로 돌리고 비교
  증거와 회귀 테스트를 보강합니다.

세부 완료 조건과 명령별 기록은
[COMMAND_PARITY_STATUS.md](COMMAND_PARITY_STATUS.md)를 참고하세요.

## 최근 게임 요소

### 고정물과 이벤트

- `Fixture`는 방에 배치되지만 일반 소지품처럼 획득하지 않는 상호작용 물체입니다.
  문, 장치, 함정, 고정 보관함, 통로, 제단·우물 같은 시설을 한 구조로 표현합니다.
- 방·아이템·Fixture에는 트리거별 Rhai 이벤트를 연결할 수 있습니다. 이벤트는
  시스템 명령보다 먼저 조사하므로, 예를 들어 `밧줄 풀어`의 고유 이벤트가 포장
  해제 명령보다 우선합니다.
- 이벤트 소스와 Fixture 정의는 변경된 파일만 다시 컴파일합니다. 평소에는 캐시된
  Rhai AST를 실행하므로, 핫리로드와 반복 컴파일 비용 절감을 함께 지원합니다.

### 런타임 월드 편집

- 관리자는 방, 출구, 몹, 아이템, Fixture, 이벤트를 서버 재시작 없이 생성·수정할
  수 있습니다. 저장 파일과 실행 중인 월드 캐시를 함께 갱신합니다.
- 신뢰된 사용자는 양도 불가 권한 아이템과 존 제한을 통해 자신의 존 안에서만 방,
  출구, Fixture를 확장할 수 있습니다. 아이템 이벤트에는 전역 정의나 이벤트 소스
  작성 권한을 주지 않아 권한 경계를 유지합니다.

자세한 입력 형식과 권한 모델은
[런타임 월드 편집 문서](docs/RUNTIME_WORLD_EDITING.md)를 참고하세요.

### Soul 캐릭터와 무리

- 한 Soul은 주 캐릭터, 보조 캐릭터, 용병을 포함해 최대 4명까지 관리하며,
  `[번호|이름] 전환`으로 조종 대상을 바꿀 수 있습니다.
- 동행 캐릭터는 이동과 몹 전투를 함께 수행하고, 이탈하면 주 캐릭터의 위치를
  기준으로 합류합니다.
- 경험치는 실제 피해 기여도를 중심으로 나누며, 큰 레벨 차이·지나치게 높은 몹·
  미미한 기여에는 패널티를 적용해 이른바 버스 육성을 제한합니다.

세부 동작과 경험치 분배식은
[Soul / 무리 코어 문서](docs/SOUL_BODY_FORMATION.md)를 참고하세요.

### 아이템 확장

- 동일한 소모품·투척 무기 등은 수량 스택으로 저장하고, 강화·UUID·기연처럼 원본과
  다른 아이템은 원본과의 차이만 별도 저장합니다. 단일 기연 아이템은 항상 고유하게
  관리합니다.
- `포장` 아이템은 여러 개의 동일 아이템을 하나의 소지품 칸으로 묶습니다.
  `[포장아이템] 풀어`로 원본 수량을 되돌릴 수 있습니다.
- 착용 세트는 `세트그룹`, `세트조건`, `세트효과`로 정의합니다. 조건을 만족한
  단계 효과는 누적되며, 소지품만으로 효과를 주는 부적류는 독립적인 `소지효과`를
  사용합니다.
- 소모품은 `사용효과`로 실제 시간 기준의 버프를 제공할 수 있고, `영구효과`로
  힘·민첩성·맷집·내공·체력·명중·회피·필살·운을 반복 복용에 따라 영구 증가시킬
  수 있습니다. 자유 특성 재분배는 명중·회피·필살·운만 허용합니다.

정의 예시는 [포장 아이템 문서](docs/ITEM_PACKAGE.md)와
[세트·소지·복용 효과 문서](docs/ITEM_SET_AND_POSSESSION_EFFECTS.md)에 있습니다.

## 빠른 실행

`run_murim.sh`가 Rust MUD 서버를 빌드하고 실행하는 권장 스크립트입니다.

```bash
# 기본 포트 9999
./run_murim.sh

# 포트 지정
./run_murim.sh 8888
```

스크립트는 포트 값과 점유 프로세스를 확인합니다. 같은 포트의 기존
`murim_server`는 정상 종료 후 재시작하며, 다른 프로그램이 점유 중이면 종료하지
않고 PID와 프로세스 이름을 알려줍니다.

Cargo로 직접 실행할 수도 있습니다.

```bash
cargo build --bin murim_server
cargo run --bin murim_server
cargo run --bin murim_server -- 8888
MUD_PORT=8888 cargo run --bin murim_server
```

서버 포트는 명령행 인자, `MUD_PORT`, `PORT`, 기본값 `9999` 순서로 결정됩니다.

## 테스트

```bash
# Rust 전체 테스트
cargo test --lib

# Python/Rust 대표 명령 exact socket 비교
# Python 서버와 Rust 서버를 각각 실행한 뒤 사용합니다.
./skills/mud-test/mud-test quick -p 9903 -r 9999
```

`mud-test quick`은 `능력치`, `소지품`, `봐`, `저장`의 ANSI와 CRLF를 포함한
출력 전체가 일치해야 성공합니다.

### Python 비교 서버

```bash
# 터미널 1: Python 기준 서버 (직접 실행 포트 9903)
python3 server.py

# 터미널 2: Rust 서버 (기본 포트 9999)
./run_murim.sh

# 터미널 3: 비교
./skills/mud-test/mud-test quick -p 9903 -r 9999
```

`server.py`에는 9900 포트의 Twisted application 선언도 있지만 직접 실행 경로는
9903을 사용합니다. 따라서 직접 실행 서버와 비교할 때는 `-p 9903`을 지정해야
합니다.

## 설계 원칙

- 게임 객체 속성은 Rhai에서 접근 가능한 해시맵으로 관리합니다.
- 데이터 접근과 공용 로직은 Rust efun으로 엔진에 등록합니다.
- 사용자 문구, 레이아웃, ANSI 꾸밈은 Rust가 아닌 Rhai 명령에서 처리합니다.
- Rhai와 데이터 정의는 변경을 감지해 필요한 대상만 재로딩하며, 정상 실행 경로는
  캐시된 컴파일 결과를 사용합니다.
- 사용자 암호는 bcrypt로 저장하며, 로그인·암호변경 등 민감한 원문 입력은 로그에
  남기지 않습니다.
- PvP는 tick 시작 snapshot을 기준으로 양쪽 행동을 계산한 뒤 피해와 사망을 동시에
  판정하여 처리 순서에 따른 선공 이점을 제거합니다.

## 프로젝트 구조

- `src/` — Rust 게임 엔진
  - `script/` — Rhai 엔진, efun, 기능별 실행 경로와 기능별 `*_test.rs`
  - `world/` — 방, 몹, 아이템, 스킬과 월드 상태
  - `player/` — 플레이어 데이터와 저장 상태
  - `combat/` — 몹 전투와 PvP 처리
  - `command/` — 명령 등록과 dispatch
  - `network/` — 접속, 입출력, 사용자 세션
- `cmds/` — 사용자 출력과 명령 orchestration을 담당하는 Rhai 스크립트
- `lib/` — Rhai 공용 라이브러리와 객체 lifecycle
- `data/` — 맵, 몹, 아이템, 사용자, 설정 JSON
- `skills/mud-test/` — Python/Rust raw socket 비교 도구

## CI/CD

GitHub Actions에서 다음 검사를 수행합니다.

- `cargo fmt --check`
- `cargo clippy`
- `cargo test`
- `murim_server` release build와 artifact 업로드

마이그레이션 원칙, 비교 절차, 수동 테스트 방법은 [AGENTS.md](AGENTS.md), 최신
완료·잔여 작업은 [COMMAND_PARITY_STATUS.md](COMMAND_PARITY_STATUS.md)에 정리되어
있습니다.
