# MUD Server (Rust)

Python 기반 MUD 서버를 Rust로 마이그레이션하는 프로젝트입니다.

## 현재 상태

- **구현 완료**: 205개 Rhai 스크립트 명령어
- **단위 테스트**: 333개 통과
- **통합 테스트**: 36/72 통과 (50%)

## 빌드 및 실행

```bash
# 개발 빌드
cargo build

# 실행 (기본 포트 9999)
cargo run

# 다른 포트로 실행
cargo run -- 8888
# 또는
MUD_PORT=8888 cargo run

# 테스트
cargo test

# 릴리즈 빌드
cargo build --release
```

## 환경 변수

- `MUD_PORT` 또는 `PORT`: 서버 포트 (기본값: 9999)

## 프로젝트 구조

- `src/` - Rust 소스 코드
  - `script/mod.rs` - Rhai 스크립트 엔진 및 EFUN 등록
  - `world/` - 월드 관리 (방, 몹, 아이템, 스킬)
  - `player/` - 플레이어 데이터 및 상태 관리
  - `command/` - 명령어 레지스트리 및 핸들러
  - `network/` - 네트워크 서버 및 클라이언트
- `cmds/` - Rhai 스크립트 명령어 (205개)
- `data/` - 게임 데이터 (JSON)

## 개발 문서

상세한 개발 문서는 [AGENTS.md](AGENTS.md)를 참고하세요.

## CI/CD

GitHub Actions를 통해 자동으로 테스트가 실행됩니다:
- 코드 포맷팅 검사 (`cargo fmt --check`)
- Clippy 정적 분석 (`cargo clippy`)
- 단위 테스트 실행 (`cargo test`)
