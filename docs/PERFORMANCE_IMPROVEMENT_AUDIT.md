# 성능 개선 후보 감사

기준일: 2026-07-15

## 목적과 범위

이 문서는 현재 Rust/Rhai 서버의 정적 감사와 성능 개선 결과를 함께 기록한다.
아직 전체 동시 접속 부하 프로파일을 마친 것은 아니므로, 구현하지 않은 후보의
우선순위는 호출 빈도, 잠금 범위, 동기 파일 I/O, 할당량이라는 구조적 근거에
따른 가설이다. Python 관찰 동작과 출력 순서를 보존하는 것이 전제다.

## 이미 적용된 기반

- 일반 명령 Rhai 소스는 `ScriptStorage`가 컴파일한 AST를 워커-로컬 캐시에
  보관하고 `run_ast_with_scope`로 실행한다. 명령마다 합친 소스를 재컴파일하는
  경로는 일반 명령 실행에 없다. (`src/script/mod.rs`의
  `ScriptStorage::cached_asts`, `ScriptStorage::execute`)
- 일반 명령 엔진은 명령별로 생성되지만 전역 Rhai 실행 mutex는 없다.
  스크립트 저장소에는 공유 read lock을 잡으므로 여러 실행은 함께 읽을 수 있고,
  hot reload의 write만 그 동안 기다린다.
- `get_skill_data()`는 `GlobalData`의 `skill.json` 스냅샷을 사용한다. 따라서
  일반 명령의 이 efun은 파일을 매번 다시 읽지 않는다.
- fixture와 item 이벤트는 수정 시각 기반 워커-로컬 AST 캐시를 이미 가진다.
  이벤트의 정상 재실행에서 매번 컴파일되지는 않는다.

## 1차 적용 및 실측 결과 (2026-07-15)

동일한 debug test binary 안에서 파일 기반 기존 경로와 캐시 경로를 각각 1,000회
호출했다. 절대 시간은 release 서버 지연 시간이 아니라 **같은 조건에서 반복
파일 I/O와 JSON 파싱을 제거한 효과**를 보는 마이크로 벤치마크다.

| 개선 | 기존 | 개선 후 | 차이 |
|---|---:|---:|---:|
| Rhai `get_murim_config()` 1,000회 | 2.219초 | 14.183ms | 156.4배 |
| 아이템 Object 템플릿 1,000회 | 70.397ms | 7.466ms | 9.4배 |
| 40인 방 관리자 컨텍스트 200회 | 3.552초 | 124.899ms | 28.4배 |

적용 내용은 다음과 같다.

- 일반 Rhai 명령의 `get_murim_config()`를 `GlobalData`의 `murim.json` 스냅샷으로
  통일했다. `메인설정 업데이트`가 같은 Arc의 데이터를 교체하므로 Engine을 다시
  만들지 않아도 새 값이 보인다.
- heartbeat에도 같은 `GlobalData`를 전달하고 `입력초과에러수`, 나이 주기,
  별호 이벤트 기준, 약초 확률 상한, 사용자 아이템 한도를 tick 시작에 캐시에서
  스냅샷한다. 활성 사용자·보상 대상마다 파일을 다시 읽지 않는다.
- `object_from_item_json()`은 Python 호환 Object 변환 결과를 파일 수정시각과
  길이로 캐시한다. 호출마다 독립 deep clone을 반환하며 파일이 바뀌면 다시
  파싱한다. 전체 원본 필드를 버릴 수 있는 typed `ItemCache`로 대체하지 않았다.
- 모든 명령에서 같은 방 사용자의 계산 속성·호위·원시 속성을 JSON으로 만들고
  직렬화하던 작업을 지연시켰다. 명령 시작에는 `Body` 스냅샷만 복제하며,
  관리자 efun이나 임의 이벤트 스크립트가 실제 요청할 때 기존 상세 구조를 한 번
  만들고 해당 명령 안의 후속 efun 호출은 이를 재사용한다.
  명령 이름 allowlist를 쓰지 않아 item/fixture/mob 이벤트의 임의 efun 호출도
  유지한다. 준비 시간은 `muc_perf`의 `room_admin_snapshot_us`로 기록한다.

실제 서버 비교는 Python 9903과 Rust 9999를 함께 실행해 확인했다.

- `quick`: `능력치`, `소지품`, `봐`, `저장` 4개 출력 전체 exact match
- `items`: 장비/품목표/버려/줘/구입/판매/먹어/입어/벗어 9개 비교 일치,
  최종 바이너리와 양쪽 서버를 새로 시작한 깨끗한 월드에서 합계 18/18 통과
- 최종 바이너리의 `basic`은 양쪽 명령 실행 20/20 성공, 출력 9/10 일치였다.
  전역 접속자 목록을 보는 `어디`만 동시 비교 서버의 접속 상태 차이로 Python
  140바이트/Rust 60바이트였다. 성능 변경의 완료 근거로 이 항목까지 일치했다고
  확대 해석하지 않는다.
- 캐시 clone 독립성, 파일 변경 hot reload, 관리자 설정 reload, 파일 fallback
  방지 회귀 테스트 통과

재현 명령:

```bash
cargo test --lib benchmark_cached_murim_config_against_file_backed_efun \
  -- --ignored --nocapture --test-threads=1
cargo test --lib benchmark_cached_item_object_against_file_parse \
  -- --ignored --nocapture --test-threads=1
cargo test --lib benchmark_lazy_room_admin_snapshot_against_eager_json \
  -- --ignored --nocapture --test-threads=1
python3 skills/mud-test/mud-test.py quick --py-port=9903 --rust-port=9999 --verbose
python3 skills/mud-test/mud-test.py items --py-port=9903 --rust-port=9999 --verbose
```

이 사실은 “Rhai가 느리다”를 곧바로 결론 내리기보다, 명령별 준비·efun·락·I/O
비용을 분리해서 봐야 함을 뜻한다.

## 우선순위 요약

| 우선순위 | 후보 | 왜 중요한가 | 먼저 확인할 지표 |
|---|---|---|---|
| P0 | 남은 명령 전 스냅샷을 capability별로 생성 | 상세 관리자 컨텍스트는 지연화했지만 view 등 공통 비용이 남아 있다 | 스냅샷 시간, 락 보유 시간, 명령별 p95 |
| P0 | 명령별 Rhai 엔진 구성 비용 축소 | AST는 캐시됐지만 매 명령 약 410개 efun 등록과 Body→Map 복사가 남아 있다 | `engine_setup_us`, `scope_setup_us`, 할당량 |
| P0 | `clients`/월드 전역 락 임계구역 축소 | 한 방의 긴 처리와 1초 heartbeat가 다른 접속자의 명령 접근을 막을 수 있다 | lock wait/hold 시간, 동시 명령 p99 |
| P1 | 설정·아이템 원본 JSON의 반복 파일 파싱 제거 | 동기 디스크 I/O와 JSON 파싱이 명령·heartbeat 경로에 남아 있다 | 파일 read 횟수/바이트, parse 시간 |
| P1 | 아이템 카탈로그·템플릿 원본 캐시 | 일부 명령이 전체 `data/mob`, `data/item`을 순회하고 템플릿을 반복 파싱한다 | 디렉터리 순회 수, 템플릿 cache hit율 |
| P2 | 이벤트 후보 탐색과 이벤트 엔진 준비량 축소 | 일반 다단어 명령마다 아이템/fixture/mob 이벤트 후보를 앞에서 검사한다 | 이벤트 miss 비용, 인벤토리 크기별 시간 |
| P2 | 출력 배치와 flush 정책 측정 | 한 메시지마다 TCP flush 하므로 방 전체 알림이 많은 상황에서 syscall이 늘어난다 | write/flush 횟수, 송신 대기 시간 |

## P0-1. 모든 명령에서 수행되는 사전 스냅샷

### 근거

`handle_single_game_command`는 보통 명령 처리 전에 다음을 수행한다.

- 현재 방의 다른 사용자 설명/이름 맵을 매번 수집한다.
  (`src/network/client.rs:4571`)
- 현재 방, 그리고 한 단어 명령이면 인접 출구 방까지의 사용자 view 스냅샷을
  매번 만든다. 보기·이동만이 아니라 일반 한 단어 명령도 이 조건에 들어간다.
  (`src/network/client.rs:4591-4627`)
- 같은 방 모든 사용자의 상세 Body/인벤토리/호위/원시 속성을 JSON으로 만드는
  기존 무조건 경로는 제거됐다. 현재는 저비용 `Body` 스냅샷만 만들고 관리자
  efun이 실제 호출될 때 상세 값을 계산한다. 이벤트 스크립트도 같은 지연 경로를
  사용하므로 관리자 명령 목록에 없는 호출도 동작한다.
- 레거시 압축 바닥 스택 확인도 명령마다 수행한다. 비어 있어도 월드 read lock을
  두 번 취득한다. 실제 압축 스택이 있으면 개별 아이템으로 바꾸며 world write
  lock을 취득한다. (`src/script/mod.rs:3588-3641`)

이 작업은 `broadcaster.clients` mutex와 `WorldState` read lock이 보유된 구간에
상당 부분 포함된다. 방 인원·인벤토리·인접 방 수가 늘수록 평범한 `점수`,
`소지품`, 채팅성 명령의 비용도 같이 증가할 수 있다.

### 제안

1. 남은 컨텍스트에는 명령 메타데이터로 필요한 capability를 명시한다. 예: `room_view`,
   `room_admin`, `room_inventory`, `global_players`, `movement_observers`.
2. 실제 필요한 명령에만 해당 스냅샷을 만들고, `봐`/이동/관리자 명령의
   Python 순서를 회귀 테스트로 고정한다.
3. 관리자 상세 컨텍스트에는 클라이언트 락 안에서 불변 `Body`만 복제하고 JSON
   변환·문자열 직렬화를 제거했다. 같은 원칙을 나머지 컨텍스트에도 적용한다.
4. 레거시 바닥 스택이 모두 이관된 뒤에는 매 명령 확인을 제거하거나, 방별
   `has_legacy_stack` 표시로 빠른 빈 경로를 둔다.

### 완료 판정

`점수`, `소지품`, 일반 채팅, `봐`, 한 단어 이동, 관리자 대상 조회를 각각
동일 상태의 Python과 비교한다. 남은 일반 명령에서는 불필요한 스냅샷 0개, 필요한
명령에서는 기존과 같은 대상 순서·출력·상태가 유지되어야 한다.

## P0-2. AST 이후에 남은 Rhai 엔진 구성 및 Scope 비용

### 근거

일반 명령은 캐시 AST를 실행하지만, 매번 새 `Engine`을 만든다.

- `create_engine_with_output`의 공통 efun 등록 약 77개
- `create_engine_with_body_and_output`의 body/명령 전용 efun 등록 약 333개
- `build_ob_from_body`가 Body의 모든 속성과 계산 속성을 새 Rhai `Map`으로 복사하고,
  이를 `player`, `me`, `ob`, `this`에 넣는다.

즉, 약 410개의 efun 등록, 출력 collector mutex 3개, scope 및 Dynamic/Map 생성은
매 명령 발생한다. 현재 trace는 `engine_setup_us`, `scope_setup_us`,
`rhai_and_efun_us`, `postprocess_us`, `total_us`를 이미 분리해 남긴다.
(`src/script/mod.rs:20385-20490` 부근)

### 제안

1. 먼저 trace를 명령 이름별 histogram으로 수집한다. `engine_setup_us`가 작으면
   이 항목은 뒤로 미룬다.
2. 비용이 확인되면 immutable 공통 efun을 worker-local 기반 엔진/모듈로 한 번만
   등록하고, 명령마다 달라지는 Body·출력·전송 efun만 invocation context로
   주입하는 설계를 실험한다.
3. Rhai가 `sync` AST가 아니므로 엔진/AST 공유 범위는 worker-local을 유지한다.
   user별 mutex로 바꿔 전역 직렬화를 되살리지 않는다.
4. `ob`의 전체 속성 복사가 실제로 필요한 명령과 key를 계측한 뒤, 속성 접근을
   lazy efun으로 바꾸거나 명령별 최소 스냅샷으로 줄일 수 있는지 검토한다.

### 주의

Body 포인터를 캡처하는 efun, call_out, hot reload 세대 변경, Rhai `Map`을 수정한
후 Rust Body로 되돌리는 현재 계약을 모두 보존해야 한다. 엔진 풀은 재진입·이전
명령 Scope 누수·잘못된 Body 포인터를 막는 검증이 필수다.

## P0-3. 전역 락과 heartbeat의 임계구역

### 근거

- `WorldState`는 단일 `std::sync::RwLock<WorldState>`다.
  방 위치, room cache, mob cache, 바닥 아이템, fixture가 모두 여기에 들어 있다.
  (`src/world/mod.rs:132-160`, `1488`)
- 명령 전 스냅샷은 `clients` mutex와 world read lock을 함께 잡는다.
  명령 실행 전에는 world lock을 명시적으로 풀지만, 그 전 작업량이 크다.
  (`src/network/client.rs:4522-5090`)
- heartbeat는 1초마다 `broadcaster.clients` mutex를 잡은 채 활성 사용자 전체의
  timeout, 전투, 자동 소비, 무공 만료, 보상·자동 습득을 처리한다.
  일부 경로는 world write lock과 동기 파일 읽기까지 이어진다.
  (`src/server/game_loop.rs:138-340`, `340-650`)

따라서 한 사용자의 긴 전투/보상 처리나 인원 많은 방의 준비가 다른 접속자의
명령에서 `clients` mutex 대기로 나타날 가능성이 있다. 이는 Rhai 평가 시간과
별도로 측정해야 한다.

### 제안

1. `clients`, world read/write, script storage read/write에 대기 시간과 보유 시간을
   명령명·heartbeat 단계와 함께 기록한다.
2. heartbeat는 (a) 최소 상태 수집, (b) 락 밖 계산, (c) 짧은 상태 반영과 메시지
   전달로 분리할 수 있는지 검토한다.
3. world는 방 단위 상태, 전역 인덱스, template cache의 락을 분리하는 방향을
   검토한다. 단, 방 간 이동·Room.objs 순서·전투 대상 전이는 원자성 경계를
   명확히 정의한 뒤에만 분할한다.
4. `clients` lock 아래에서 파일 I/O, JSON 직렬화, 긴 목록 순회를 하지 않는 것을
   원칙으로 한다.

## P1-1. 남은 동기 설정 JSON 읽기와 파싱

`get_skill_data()`와 Rhai `get_murim_config()`는 캐시화됐지만, 설정 전체가 같은
상태는 아니다.

- `get_murim_config_int`, `get_murim_main_config_list`에는 `murim.json`을 직접 읽고
  파싱하는 내부 경로가 남아 있다. heartbeat의 `입력초과에러수`와 일반 Rhai
  `get_murim_config()`는 1차 개선에서 `GlobalData`로 전환했지만, 다른 Rust 내부
  호출은 아직 파일 기반이다. (`src/script/mod.rs:345-380`)
- `magic_map.json`, `script.json`, `dropitem.json`도 사용 시 동기 파일 읽기와 JSON
  파싱을 한다. (`src/script/mod.rs:397`, `src/script/combat_commands.rs:1500`,
  `src/server/game_loop.rs:1156`)
- `resolve_skill_definition`, `skill_definition_names`,
  `mob_has_combat_skill`는 아직 `skill.json`을 직접 읽는다.
  (`src/script/mod.rs:1430`, `3918-3955`)

### 제안

`GlobalData`에 있는 것은 Rhai 및 Rust 양쪽에서 같은 스냅샷을 사용하도록
통일한다. `data/config` 파일은 raw JSON 또는 필요한 projection을 버전과 함께
캐시하고, `업데이트`가 원자적으로 새 버전을 교체한다. 글로벌 데이터가 없는
도구/단위 테스트의 파일 fallback은 명시적으로 유지한다.

먼저 heartbeat의 사용자당 설정 파일 읽기를 없애는 것이 가장 안전한 첫 대상이다.

## P1-2. 아이템 템플릿과 카탈로그

### 근거

- `object_from_item_json`은 1차 개선에서 Python 호환 Object 템플릿 캐시로
  전환됐다. 파일 metadata 확인과 deep clone은 남지만 반복 파일 본문 읽기와 JSON
  파싱은 제거됐다. 비테스트 런타임 참조가 많아 이 한 지점의 개선이 전체 호출자에
  적용된다. (`src/script/mod.rs`의 `object_from_item_json`)
- `get_item_catalog()`는 `data/mob` 전체를 순회·파싱하고 이어서 `data/item` 전체를
  순회·파싱한다. `아이템찾기`, `방어구찾기`, `올숙리스트`, `조제`에서 호출한다.
  (`src/script/mod.rs:3790-3915`, `cmds/*.rhai`)
- `ItemCache`는 존재하지만 RawItemData는 전체 원본 JSON 속성을 보존하지 않는다.
  기존 `object_from_item_json`을 무조건 typed cache로 바꾸면 Python 호환 필드가
  사라질 위험이 있다.

### 제안

Python 호환 Object template 캐시와 파일 변경 재파싱은 적용됐다. 다음 단계는
카탈로그를 item/mob 수정 시에만 재생성하는 검색 인덱스(이름·반응이름·종류·사용
가능 여부)로 바꾸는 것이다. 관리자 reload가 인덱스 세대를 교체하게 한다.

정확한 파일 순서, 중복 이름 선택, `사용아이템`에서 발견되는 항목 순서는 Python
비교로 고정해야 한다.

## P2-1. 이벤트 후보 탐색

일반 다단어 입력은 등록 명령으로 가기 전에 item → fixture → mob 이벤트 후보를
차례로 검사한다. (`src/network/client.rs:5146-5160`)

- item 이벤트는 인벤토리 개별 객체와 stack key를 검사하며, stack key마다 현재
  `object_from_item_json`을 호출할 수 있다. (`src/script/item_event.rs:73-115`)
- fixture 이벤트는 현재 방 fixture를 순회한다.
- mob 이벤트는 현재 방의 mob을 순회하고 후보 RawMobData를 clone한다.
  (`src/world/event.rs:1483-1565`)

이벤트 AST 자체는 이미 mtime 캐시되어 있으므로, 우선순위는 컴파일이 아니라
**miss 경로의 대상 탐색과 템플릿 로드**다. 방/인벤토리 크기별로 이벤트 miss 비용을
측정한 뒤, trigger 첫 단어 인덱스나 이벤트 보유 여부의 빠른 인덱스를 검토한다.
명령 우선순위(item → fixture → mob → 이동/등록 명령)는 바꾸면 안 된다.

## P2-2. 출력·전송

클라이언트 송신 task는 채널에서 꺼낸 메시지마다 `write_all` 후 `flush`한다.
(`src/network/client.rs:360-390` 부근) 방 전체 알림, 전투 출력, 다수 줄 출력에서는
작은 write/flush가 늘 수 있다.

측정 전에는 바꾸지 않는다. 개선이 필요하면 한 명령/한 heartbeat의 같은 수신자
출력을 이미 유지하는 CRLF 순서대로 합친 후 한 번 송신하는 방식과, prompt/사망/
로그아웃 같은 즉시 flush가 필요한 경로를 분리해 비교한다.

## 측정 계획

### 1. 명령 단계별 지연 시간

기존 `ScriptStorage::execute` trace 필드에 아래를 추가하거나 집계한다.

| 단계 | 측정값 |
|---|---|
| 명령 전 컨텍스트 | snapshot 생성 시간, room/online/player 수 |
| Rhai 준비 | AST cache hit/miss, engine setup, scope setup |
| Rhai/efun | 전체 eval 시간과 호출한 주요 efun별 시간 |
| 명령 후 처리 | output 확장, JSON 직렬화, 전송 큐 시간 |
| 동시성 | clients/world/script/global-data lock wait 및 hold 시간 |

명령명, 입력 길이, 방 인원, 인벤토리 개별 수/stack 수, 현재 접속자 수를 tag로
기록하고 p50/p95/p99 및 최대값을 본다. 단일 평균만으로 우선순위를 결정하지
않는다.

### 2. 시나리오

동일한 release binary와 고정된 월드/캐릭터 스냅샷으로 다음을 실행한다.

1. 저비용: `점수`, `소지품`, 짧은 채팅, 한 단어 이동
2. 방 조회: `봐` (빈 방/사용자 10명/몹·아이템 다수)
3. 데이터 조회: `무공상태`, `무공리스트`, `아이템찾기`, `조제`
4. 상태 변경: `먹어`, `버려`, `구입`, `투척`, fixture/item/mob 이벤트
5. 동시성: 위 명령을 1/10/50 접속자로 반복하고, 별도로 1초 heartbeat 전투와
   hot reload를 겹친다.

Python/Rust 패리티가 필요한 시나리오는 `skills/mud-test` raw TCP 방식으로 출력과
상태를 먼저 고정하고, 부하 생성은 그 이후 별도 클라이언트로 한다.

### 3. 프로파일 도구

- `tracing` JSON/OTLP 또는 histogram으로 명령 단계와 lock 시간을 수집
- Linux `perf record`/flamegraph로 CPU hot path 확인
- `strace -c -f` 또는 eBPF로 `openat`, `read`, `write`, `fsync` 및 네트워크 write
  횟수 확인
- allocator profiling(가능한 경우)으로 Rhai Map/Dynamic·JSON 직렬화·문자열 join의
  할당 비중 확인

## 권장 실행 순서

1. 측정부터 추가해 명령별 비용을 Rhai 준비, Rhai/efun, snapshot, lock, I/O로 분리
2. P0-1에서 상세 관리자 컨텍스트처럼 나머지 사전 스냅샷도 지연화하거나
   capability가 필요한 명령으로 제한
3. heartbeat의 설정 읽기와 P1 설정 캐시 누락을 제거
4. item template/catalog 원본 캐시를 도입
5. 수치상 `engine_setup_us`가 의미 있을 때만 worker-local 엔진 재사용을 설계
6. 마지막으로 실제 lock wait가 큰 경우에만 world/client 락 분할을 진행

각 단계는 Python 비교, Rust 회귀 테스트, 단일/동시 부하 프로파일을 모두 통과한
뒤 다음 단계로 넘어간다.
