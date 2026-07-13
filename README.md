# MUD Server (Rust/Rhai)

Python MUD 서버의 관찰 가능한 동작을 Rust 게임 엔진과 Rhai 명령 스크립트로
마이그레이션한 프로젝트입니다. Python 원본은 호환성 비교 기준으로 함께 보존합니다.

## 현재 상태

- Python `CmdObj`가 있는 플레이어 명령 **189개**의 직접 비교를 완료했습니다.
- 사용자 전송, 방 객체 선택 순서, PvP 동시 타격/사망, Python↔Rust 저장 호환,
  실행·로그·socket 출력까지 통합 감사를 완료했습니다.
- `cmds/`에는 플레이어 명령, 비공개 hook, helper를 포함한 Rhai 소스 **210개**가
  있습니다. 파일 수와 실제 공개 명령 수는 같지 않습니다.
- 최신 기록 기준 Rust 라이브러리 테스트 **861개**, Python password 테스트 2개와
  mud-test harness 테스트 8개가 통과했습니다.
- 이후 Python과의 실제 동작 차이가 발견되면 해당 항목을 다시 미완료로 돌리고
  비교 증거와 회귀 테스트를 보강합니다.

세부 완료 조건과 명령별 기록은
[COMMAND_PARITY_STATUS.md](COMMAND_PARITY_STATUS.md)를 참고하세요.

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
- `cmds/*.rhai`는 실행 중 변경 사항을 다시 읽는 hot-reload 구조입니다.
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
