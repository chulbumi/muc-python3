# MUD Server (Rust)

Python 기반 MUD 서버를 Rust로 마이그레이션하는 프로젝트입니다.

## 현재 상태

- `cmds/`에 204개의 Rhai 명령 파일이 있습니다.
- Python과의 동작 호환성은 현재 재검증 중입니다. 파일 개수만으로 명령의 동작 이관이 완료되었다고 판단하지 않습니다.
- 테스트 개수는 코드 변경에 따라 달라지므로 문서에 고정하지 않습니다. 아래 명령이나 CI의 최신 결과를 확인하세요.

## 빌드 및 실행

```bash
# 개발 빌드
cargo build --bin murim_server

# 실행 (기본 포트 9999)
cargo run --bin murim_server
# Cargo.toml의 default-run도 murim_server이므로 cargo run과 동일합니다.

# 다른 포트로 실행
cargo run --bin murim_server -- 8888
# 또는
MUD_PORT=8888 cargo run --bin murim_server

# 테스트
cargo test

# 릴리즈 빌드
cargo build --release --bin murim_server
# 결과 파일: target/release/murim_server
```

## 환경 변수

- `MUD_PORT` 또는 `PORT`: 서버 포트 (기본값: 9999)

## Python 비교 서버 실행

현재 `server.py`를 직접 실행하면 `reactor.listenTCP(9903, ...)` 경로로 9903 포트에 바인딩합니다.
같은 파일에 9900 포트의 Twisted `application` 서비스 선언도 있지만, 모듈 최상위에서 별도의 reactor를 직접 실행하므로 현재 비교 절차에서는 직접 실행 경로를 사용합니다.

```bash
# Python 비교 서버 (현재 직접 실행 포트 9903)
python3 server.py

# Rust 서버 (기본 포트 9999)
cargo run --bin murim_server

# 저장소의 mud-test 기본 Python 포트는 9900이므로 현재 직접 실행 서버에는 명시적으로 9903을 전달합니다.
./skills/mud-test/mud-test quick -p 9903 -r 9999
```

## 프로젝트 구조

- `src/` - Rust 소스 코드
  - `script/mod.rs` - Rhai 스크립트 엔진 및 EFUN 등록
  - `world/` - 월드 관리 (방, 몹, 아이템, 스킬)
  - `player/` - 플레이어 데이터 및 상태 관리
  - `command/` - 명령어 레지스트리 및 핸들러
  - `network/` - 네트워크 서버 및 클라이언트
- `cmds/` - Rhai 스크립트 명령어
- `data/` - 게임 데이터 (JSON)

## 개발 문서

상세한 개발 문서는 [AGENTS.md](AGENTS.md)를 참고하세요.

## CI/CD

GitHub Actions를 통해 자동으로 테스트가 실행됩니다:
- 코드 포맷팅 검사 (`cargo fmt --check`)
- Clippy 정적 분석 (`cargo clippy`)
- 단위 테스트 실행 (`cargo test`)
- `murim_server` 릴리즈 빌드 및 같은 이름의 artifact 업로드
