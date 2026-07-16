# MUD Python → Rust 마이그레이션 프로젝트

## 프로젝트 규칙

### 기본 원칙
- **파이썬 코드를 모두 마이그레이션**해야 합니다.
- **rhai 스크립트 구조**에 맞게 마이그레이션 해야 합니다.
- **cmds/ 디렉토리의 파이썬 코드**는 실행시간에 업데이트하는 구조로 되어있었고, 이 코드들을 rhai 구조에 맞게 마이그레이션 해야 합니다.
- **모든 동작은 파이썬에서 기대하던 대로 동작**해야 합니다. 기술적으로는 다르더라도, 결과적으로 같아야 합니다.
- **객체 속성**은 변수나 상수가 아닌, **해시맵**으로 관리해야 합니다. 그래야 rhai에서 접근이 용이합니다.
- **유용한 함수**(데이터 접근, 로직, 유틸)는 모두 **efun** 형태로 Rust 엔진에 등록해 두고, rhai에서는 그 efun을 호출해 쓰세요.
- **출력 내용·포맷**(문구, 레이아웃, ANSI 꾸밈 등)은 언제든 바뀔 수 있으므로 **Rhai 명령(cmd/main)**에서만 다루고, Rust에는 출력 포맷을 넣지 마세요.

---

## 현재 재검증 완료 범위 (2026-07-10)

아래 항목은 Python 원본과 직접 대조하고 개별 회귀 테스트를 통과한 범위이다.
이 목록은 전체 마이그레이션 완료를 의미하지 않는다.

- 같은 방 조회/알림: `WorldState` 방별 삽입 순서 인덱스와 이름→접속
  인덱스로 해당 방 사용자만 조회한다. Rhai의 `get_all_online_players()`는
  Python도 전역 목록을 보는 `누구`, `어디`에만 남아 있다.
- 한 단어 방향·고유 출구 이동과 같은 방 follower 후속 이동은 현재
  표현 가능한 안전한 방/이벤트 분기에서 Python 순서와 출력 검증 완료
- `버려`: 선택·전체·수량·방 50개 제한·파괴·ONEITEM과 Python의 반복
  변수 재사용 동작 검증 완료. 기존 압축 스택과 객체의 혼합 순서는 미완료
- `무림별호`: Python의 속성/이벤트/공지/makeHome/귀환 이동 순서와 플레이어 ANSI 이름 형식 재대조
- `줄임말`: Python 배열 저장 호환, 삽입 순서, `*` 치환, `;` 후속 명령, 중첩/수량 제한
- `소지품`: 관리자의 같은 방 대상 조회, 장착/숨김 처리, 은전/금전, 문구·ANSI·공백
- `추적`: Python `Room.Zones` 생성/순서, 모든 방 순회, 사망·리젠 상태로 방에 남은 몹 포함
- `자동무공`, `자동무공삭제`, 순위 동률 정렬/초기화, 명령 권한 메타데이터
- `무공`: Python 카테고리/MAIN_CONFIG 순서, 숙련도, EUC-KR 3열 폭,
  비전수련/비전이름, 관리자 같은 방 대상 조회와 Rhai 핫리로드 실행 경로
- `mud-test` 연결/실패 판정과 CLI nonzero exit 전파
- `조회`/`입금` 보험료 상태 전이와 Python 문구/ANSI/CRLF
- `무공상태` 활성 상태 출력 데이터, 관리자 같은 방 대상 스냅샷
- `무공전수`/`무공전수2`/`무공제거` 대상 Body 변경 요청 경로
- `자동경로` alias 경로 저장·삭제 및 다음 명령 재진입
- `cmds/master.rhai` Master apply 호출과 등록 스크립트 heartbeat 호출

Rust는 개별 바닥 아이템·몹·플레이어의 `room.objs` 삽입 순서를 저장한다.
몹은 안정적인 runtime `instance_id`로 참조하므로 동일 템플릿 복제 개체의
`env.findObjName()`/숫자 선택 순서도 보존된다. 다만 legacy Rust 전용 압축
바닥 스택은 개별 Python Item 식별자가 없고, 동일 이름으로 중복 접속한
플레이어는 `WorldState` 이름 키로 서로 구분할 수 없어 추가 이관이 필요하다.

---

## 이전 진행 기록 (2026-02-15, 현재 재검증 필요)

이 절의 완료 표시는 당시 작업 기록입니다. 현재 작업 트리의 전체 Python ↔ Rust 동작 비교를 증명하지 않습니다.
현재 확인 가능한 사실은 `cmds/`에 Rhai 파일이 207개(플레이어 명령 206개와 내부 이동 처리 스크립트 1개) 있다는 것이며, 파일 존재만으로 Python 동작의 이관 완료를 판정하지 않습니다.
과거 `test_results.md`는 2026-02-15의 실패한 비교 결과물로 배포 트리에서 제거했습니다. 새 비교 결과도 Git에는 넣지 않으며, 현재 실행의 포트·조건·출력으로만 판정합니다.

### 기록된 작업 (완료 여부는 현재 미검증)
1. **Rhai 스크립트 기반 명령어 시스템**
   - 현재 207개의 `.rhai` 파일 존재(플레이어 명령 206개, 내부 처리 1개)
   - 핫리로드 지원 (실행 중 .rhai 파일 수정 시 즉시 반영)

2. **캐릭터 데이터 호환성**
   - `무공이름`, `무공숙련도`를 배열 형식으로 저장하여 Python 호환성 확보
   - `src/script/mod.rs`의 `save_body_to_json()` 함수 수정

3. **DOUMI 캐릭터 생성 시스템**
   - 빠른도우미, 초기도우미, 나만바라바 기능 작동

4. **기본 명령어 테스트**
   - 이전 문서에 기본 응답 확인으로 기록됐으나 현재 실행 증거는 없어 재검증 필요

5. **✅ 캐릭터 시작 위치 로드 수정 (2026-01-31 완료)**
   - `src/network/client.rs`: 저장된 `위치` → `귀환지맵` → `낙양성:42` 순서로 검사
   - `src/world/mod.rs`: `PlayerPosition::start_fallback()` 메서드 추가
   - Python 서버와 동일한 기본 시작 위치 (낙양성:42) 사용

6. **✅ 캐릭터 데이터 로드 수정 (2026-01-31 완료)**
   - **체력/능력치 로드**: JSON에서 체력, 힘, 민첩성 등을 제대로 로드
   - **은전 표시**: 소지품 명령에서 은전 10000개 정상 표시
   - **위치 호환성**: Python JSON의 "현재방" 필드를 "위치"로 변환
   - **금화/은화 변환**: 레거시 "금화"/"은화"를 "은전"으로 변환
   - `src/script/mod.rs`: `build_ob_from_body()`에 한글 속성키 추가

7. **전체 시스템 비교 테스트 기록 (2026-01-31, 현재 재검증 필요)**
   - **전투 시스템**: 데미지 계산, 경험치, 명중률 비교 완료
   - **NPC 대화**: 이벤트 기반 대화 시스템 검증 완료
   - **아이템 시스템**: 구입/판매/버려/줘 명령 테스트 완료
   - **멀티플레이어**: 채팅, 주고받기 기능 확인 완료
   - 당시 호환성 95% 이상으로 기록되었으나 현재 전체 비교 근거는 없음

8. **✅ 전투 시스템 방어력 계산 수정 (2026-02-15)**
   - `src/world/mob.rs`: `arm` (맷집) 필드 추가
   - `src/combat/processor.rs`: 방어력 계산에 `mob_data.arm` 사용
   - Python 공식과 일치: `(c1 + c2) - mob.getArm()`

9. **✅ 숙련도 시스템 구현 (2026-02-15)**
   - `src/player/body.rs`: 무기 타입/숙련도 조회 메서드 추가
     - `get_weapon_type()`: 장착 무기의 종류 반환 (1~5)
     - `get_mastery(weapon_type)`: 해당 무기 숙련도 반환
     - `get_weapon_skill()`: 무기 기량 반환
     - `get_mastery_diff()`: 숙련도 차이 계산 (ss = s1 - s2)
   - `src/combat/processor.rs`: 전투 데미지에 숙련도 반영
     - Python 공식과 일치: `c2 = getAttPower() - ss`
   - 당시 호환성 98% 이상으로 기록되었으나 현재 전체 비교 근거는 없음

10. **✅ 명중률 계산 공식 수정 (2026-02-15)**
    - `src/combat/processor.rs`: Python 명중률 공식과 정확히 일치하도록 수정
    - 공식: `CHANCE = 100 - ((mob_level - player_level + 90) / 3) + (hit * 0.2) - (miss * 0.2)`
    - 범위: 5%~95% 클램핑

11. **✅ 스킬 데미지 숙련도 보너스 구현 (2026-02-15)**
    - `src/combat/processor.rs`: Python 스킬 레벨별 데미지 보너스 적용
    - 11레벨(초급): 1.3x, 12레벨(중급): 1.5x, 13레벨: 1.7x
    - 14레벨: 2.0x, 15레벨: 2.5x, 16레벨+: 3.0x

12. **호환성 완료 주장 (2026-02-15 기록, 현재 미검증)**
    - 현재 작업 트리에서 전투, 아이템, 이동/맵, NPC/이벤트, 저장/로드 전체를 Python과 재비교한 실행 증거가 없음
    - 완료 조건은 각 기능의 Python 관찰 동작과 Rust 결과가 같은 상황에서 일치하는 것임

13. **✅ NPC/이벤트 관리자 명령어 구현 (2026-02-15)**
    - `src/world/mob.rs`: 몹 이벤트 관리 메서드 추가
      - `check_mob_event()`: 몹 이벤트 확인
      - `set_mob_event()`: 몹 이벤트 설정
      - `del_mob_event()`: 몹 이벤트 삭제
    - `src/script/mod.rs`: 이벤트 관리 efun 함수 추가
      - `check_mob_event()`, `set_mob_event()`, `del_mob_event()`
      - `get_admin_level()`, `admin_force_command()`
    - `cmds/이벤트설정.rhai`: 이벤트 설정 명령어 구현 (관리자 레벨 1000)
    - `cmds/이벤트삭제.rhai`: 이벤트 삭제 명령어 구현 (관리자 레벨 1000)
    - `cmds/명령.rhai`: 플레이어 강제 명령 실행 구현 (관리자 레벨 2000)

14. **✅ 말하기(Say) 명령어 호환성 수정 (2026-02-15)**
    - Python: 줄 끝 공백/문장부호로 말하기 인식 (예: '안녕 ')
    - Rust: '말 [내용]' 명령어 방식 + Python 방식 모두 지원
    - `src/network/client.rs`: `trim()` → `trim_end_matches('\r').trim_end_matches('\n')` 수정
      - 줄 끝 공백을 보존하여 Python과 동일한 말하기 감지 방식 지원
    - `objs/alias.py`: '능력치' → '점수' alias 추가

### 발견된 주요 이슈

#### ~~1. 캐릭터 위치 로드 버그 (Rust 서버)~~ **✅ 해결됨**
- **해결**: `src/network/client.rs` 수정, 저장된 위치/귀환지맵 순서로 처리

#### ~~2. 시작 방 불일치~~ **✅ 해결됨**
- **해결**: Rust도 이제 `낙양성:42`를 기본값으로 사용

#### ~~3. 캐릭터 능력치 로드 버그~~ **✅ 해결됨**
- **해결**: `src/script/mod.rs`의 `build_ob_from_body()`에 한글 속성키 추가

#### ~~4. 전투 시스템 방어력 계산~~ **✅ 해결됨 (2026-02-15)**
- **Python**: `(c1 + c2) - (mob.getArm() + mob.getArmor())` 공식
- **Rust 수정**: `mob_data.arm` (맷집)을 방어력으로 사용하도록 수정
- **파일**: `src/world/mob.rs` - `arm` 필드 추가, `src/combat/processor.rs` - 방어력 계산 수정
- **호환성**: Python 공식과 일치하도록 개선됨

#### 5. Python 서버 명령어 응답 비교 **현재 재검증 필요**
- 2026-02의 `test_results.md`는 Python 9900 포트를 사용한 과거 결과라 현재 직접 실행 포트(9903) 기준의 증거가 아니며, 배포 트리에서 제거함

---

## Python 서버와의 비교 방법

### 1. 명령어 동작 비교
각 명령어를 **동일한 상황**에서 실행하고 실제 출력을 비교:

```python
# 테스트 패턴
1. 양쪽 서버에 동일 캐릭터로 로그인
2. 동일한 방(위치)에 있는지 확인
3. 명령어 실행
4. 출력 내용 비교 (단순 응답 유무가 아닌 실제 내용 비교)
5. 수치 계산, 상태 변화 등의 결과 비교
```

### 2. 데이터 형식 확인
- 캐릭터 JSON 파일이 양쪽 서버에서 모두 읽을 수 있는지
- `무공이름`, `무공숙련도`가 배열 형식인지 확인

### 3. 상황별 테스트
- 전투: 몹이 있는 방에서 공격 후 데미지/경험치 비교
- 아이템: 아이템 획득/장착/사용 후 상태 비교
- 이동: 동일한 방에서 이동 명령 후 도착지 비교

---

## 앞으로 할 일 (TODO)

### 마이그레이션 완료 여부: 미검증
- [ ] 모든 Python 명령을 같은 상태와 입력으로 Rust/Rhai와 비교
- [ ] 전투, 아이템, 이동, NPC/이벤트, 저장/로드 상태 변화를 비교
- [ ] Python ↔ Rust 캐릭터 데이터 양방향 로드를 현재 작업 트리에서 재검증

### 향후 개선 사항 (선택)
- [ ] 성능 테스트 (동시 접속자, 응답 시간)
- [ ] 모니터링 시스템 구축
- [ ] 로그 분석 도구

### 마이그레이션 검증 범위

#### 1. Python 명령어 이관
- 현재 `.rhai` 파일 207개 존재(플레이어 명령 206개, 내부 처리 1개)
- Python `cmds/`의 각 관찰 동작이 구현됐는지는 개별 비교 필요

#### 2. 핵심 시스템 비교 대상
1. **전투 시스템** - 공격, 데미지 계산, 경험치, 레벨업
2. **아이템 시스템** - 획득, 장착, 사용, 버리기
3. **스킬 시스템** - 무공 학습, 사용, 자동무공
4. **소셜 시스템** - 파티, 문파, 채팅, PvP
5. **경제 시스템** - 상점, 거래, 은전

#### 3. 방/맵 데이터 호환성
- 방 데이터, 출구, NPC, 몹 스폰 위치를 같은 시작 상태에서 비교해야 함

---

## 테스트 체크리스트

### 기능 테스트
- [ ] 로그인/로그아웃
- [ ] 캐릭터 생성/삭제
- [ ] 위치 저장/로드 (시작 위치, 귀환지)
- [ ] 이동 (동서남북위아래)
- [ ] 전투 (공격, 데미지, 경험치, 사망)
- [ ] 아이템 (획득, 장착, 사용, 버리기)
- [ ] 스킬 (무공 학습, 사용, 자동무공)
- [ ] 소셜 (파티, 문파, 채팅, PvP)
- [ ] 상점 (구입, 판매)
- [ ] 저장/로드

### 데이터 호환성 테스트
- [ ] Python에서 저장한 캐릭터를 Rust에서 로드
- [ ] Rust에서 저장한 캐릭터를 Python에서 로드
- [ ] 무공 데이터 배열 형식 확인
- [ ] 위치 데이터 정확성 확인

### 성능 테스트
- [ ] 동시 접속자 수
- [ ] 명령어 응답 시간
- [ ] 메모리 사용량

---

## Python 서버 실행 방법 (비교용)

```bash
# Python 서버 직접 실행 (현재 reactor listener 포트 9903)
cd /home/ubuntu/muc-python3
python3 server.py

# Rust 서버 (포트 9999)
cargo run --bin murim_server

# 현재 포트로 비교 테스트
./skills/mud-test/mud-test quick -p 9903 -r 9999
```

`server.py`에는 9900 포트의 Twisted `application` 서비스 선언과 9903 포트의 직접 `reactor.listenTCP` 실행 경로가 함께 있습니다.
현재 직접 실행 명령은 9903을 사용합니다. `mud-test` 래퍼의 기본 Python 포트는 9900이므로 직접 실행 서버를 시험할 때 `-p 9903`을 반드시 지정합니다.

---

## 보고서 파일

- `test_results.md` - 비교 도구가 만드는 로컬 결과물이며 Git에는 포함하지 않습니다. 생성 시각과 포트를 확인하고 현재 결과로 간주하지 마세요.
- 이전 문서에 언급된 `PDCA_COMPARE_REPORT.md`, `FINAL_TEST_REPORT.md`, `CRITICAL_DIFFERENCES.md`, `ACTUAL_DIFFERENCES_FOUND.md`는 현재 작업 트리에 없습니다.

---

## MUD 테스트 스킬 사용법

### 스킬 개요
`skills/mud-test/` 디렉토리에 Python/Rust MUD 서버 비교 테스트 스킬이 구성되어 있습니다.

### 실행 방법

#### 1. 별칭 명령어 (권장)

시스템 전체 별칭이 설치된 경우 어디서든 실행 가능:

> 현재 `server.py` 직접 실행 포트는 9903이지만 래퍼 기본값은 9900입니다. 아래 비교 예시는 모두 `-p 9903`을 명시합니다.

```bash
# 별칭 설치 확인
which mud-test
# 출력: /home/ubuntu/.local/bin/mud-test

# 빠른 테스트 실행 (권장)
mud-test quick -p 9903

# 전체 테스트
mud-test all -p 9903

# 특정 카테고리 테스트
mud-test basic -p 9903
mud-test combat -p 9903
mud-test movement -p 9903

# 상세 출력 모드
mud-test quick -p 9903 -v

# 커스텀 포트 지정
mud-test quick -p 9901 -r 9998
```

**별칭 설치 방법:**
```bash
ln -s /home/ubuntu/muc-python3/skills/mud-test/mud-test ~/.local/bin/mud-test
```

#### 2. Bash 래퍼로 직접 실행
```bash
# 도움말
./skills/mud-test/mud-test -h

# 전체 테스트
./skills/mud-test/mud-test all -p 9903

# 옵션과 함께 실행
./skills/mud-test/mud-test basic -p 9903 -v      # verbose 모드
./skills/mud-test/mud-test combat -p 9901 -r 9998  # 커스텀 포트
./skills/mud-test/mud-test quick -p 9903 --host 192.168.1.100  # 원격 호스트
```

#### 3. Python 비교 스크립트로 직접 실행

비교 스크립트는 Python/Rust 포트를 옵션으로 받습니다. 현재 Python 직접 실행 서버에는 `--py-port=9903`을 지정합니다:
```bash
# 도움말 보기
python3 skills/mud-test/mud-test.py help

# 전체 테스트 실행
python3 skills/mud-test/mud-test.py all --py-port=9903

# 특정 테스트만 실행
python3 skills/mud-test/mud-test.py basic --py-port=9903
python3 skills/mud-test/mud-test.py combat --py-port=9903
python3 skills/mud-test/mud-test.py movement --py-port=9903
python3 skills/mud-test/mud-test.py items --py-port=9903
python3 skills/mud-test/mud-test.py npc --py-port=9903

# 빠른 비교 테스트 (핵심 명령어만)
python3 skills/mud-test/mud-test.py quick --py-port=9903

# 서버 상태 확인
python3 skills/mud-test/mud-test.py status --py-port=9903

# 마지막 리포트 보기
python3 skills/mud-test/mud-test.py report
```

### 사용 가능한 명령어

| 명령어 | 설명 |
|--------|------|
| `all` | 모든 테스트 시나리오 실행 (기본값) |
| `basic` | 기본 명령어 테스트 (능력치, 소지품, 점수 등) |
| `movement` | 이동 명령어 테스트 (동서남북위아래) |
| `combat` | 전투 시스템 테스트 (공격, 스킬, 도망) |
| `items` | 아이템 상호작용 테스트 (구입, 판매, 버리기) |
| `npc` | NPC 대화 테스트 |
| `quick` | 빠른 비교 테스트 (핵심 명령어 4개만) |
| `status` | 서버 연결 상태 확인 |
| `report` | 마지막 테스트 리포트 표시 |
| `help` | 도움말 표시 |

### 옵션

| 옵션 | 설명 | 기본값 |
|------|------|--------|
| Bash 래퍼: `--py-port N`, `--py-port=N`, `-p N`; Python 스크립트: `--py-port=N` | Python 서버 포트 | 9900 |
| Bash 래퍼: `--rust-port N`, `--rust-port=N`, `-r N`; Python 스크립트: `--rust-port=N` | Rust 서버 포트 | 9999 |
| Bash 래퍼: `--host HOST`, `--host=HOST`; Python 스크립트: `--host=HOST` | 서버 호스트주소 | localhost |
| `--verbose` / `-v` | 상세 출력 활성화 | false |
| Bash 래퍼: `--report FILE`, `--report=FILE`, `-o FILE`; Python 스크립트: `--report=FILE` | 리포트 파일 경로 | test_results.md |
| Bash 래퍼: `-h`, `--help` | 도움말 표시 | - |

### 테스트 결과
테스트 결과는 Git에서 제외된 로컬 `test_results.md` 파일에 자동 저장됩니다.

### Python 서버 테스트 (Socket 방식)

Python 서버는 telnetlib의 IAC 네고시에이션 문제를 피하기 위해 **raw socket**을 사용합니다.

**Socket 테스트 스크립트 (`mud-test-socket.py`):**
- Raw socket 연결로 telnetlib IAC 문제 회피
- UTF-8 인코딩 사용
- DOUMI 캐릭터 생성 자동화 (`나만바라바` → `빠른도우미` → Enter 반복)
- CRLF (`\r\n`) 라인 엔딩 사용

**DOUMI 캐릭터 생성 흐름:**
1. `나만바라바` 명령어로 DOUMI 모드 진입
2. 캐릭터 이름 입력 (한글만 가능, 예: `테스터`)
3. `빠른도우미` (옵션 1) 선택
4. Enter 키 연속 입력으로 기본값 수락
5. `낙양성` 입장 메시지 확인 시 완료

**직접 Socket 테스트 실행:**
```bash
python3 skills/mud-test/mud-test-socket.py --host localhost --port 9903 --name 테스터
```

### 예상 실행 결과 예시
```
╔══════════════════════════════════════════════════════════════════╗
║                    MUD Test Skill - Help                        ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║  Usage: /mud-test [command] [options]                           ║
║                                                                  ║
║  Commands:                                                       ║
║    all           Run all test scenarios (default)               ║
║    basic         Test basic commands                            ║
║    movement      Test movement commands                         ║
║    combat        Test combat system                             ║
║    items         Test item interactions                         ║
║    npc           Test NPC dialogue                              ║
║    quick         Quick comparison test                          ║
║    status        Show server connection status                  ║
║    report        Show last test report                          ║
║    help          Show this help message                         ║
...
```
