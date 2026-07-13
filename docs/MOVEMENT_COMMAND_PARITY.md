# Python 방향/출구 이동 이관 근거

기준 원본은 `objs/player.py`의 `parse_command`, `enterRoom`, `exitRoom`,
`viewMapData`와 `objs/room.py`의 `initExit`, `setHiddenExit`, `sortExit`,
`getExit`, `update`이다. Rust의 과거 방향 명령과 네트워크
`handle_movement` 문구는 기준으로 사용하지 않았다.

## 실행 구조

- 방향은 Python처럼 명령 객체가 아니다. `cmds/__movement.rhai`는
  `CommandRegistry`의 private handler로만 등록되어 명령 목록/별칭 lookup에
  노출되지 않는다.
- private handler도 매 실행 시 `ScriptStorage`의 최신 source를 읽으므로 서버
  실행 중 문구·ANSI·레이아웃 수정이 반영된다.
- 네트워크 순서는 방의 `명령금지` → 몹 이벤트 → 한 단어 출구 → 일반 명령이다.
  한 단어 출구는 같은 이름의 일반 명령보다 먼저 처리한다.
- `e/w/s/n/u/d/ne/nw/se/sw`와 한글 초성 별칭은 Python 전역 alias를 거친 뒤
  동일한 출구 처리로 들어간다.
- 같은 방 사용자는 `WorldState::get_players_in_room`과 요청된 이름만 찾는
  broadcaster index로 조회한다. 이동/알림/view snapshot 경로에서 전체 접속자
  map을 순회하지 않는다.

## 보존한 동작

- 10방향 부재 문구, `Move where?`, 정확한 출구명과 Python `dict` 순서의 첫
  접두 출구 선택
- 숨김 출구의 `$` 제거/재삽입 순서와 숨김 이동 퇴장 문구
- 목적지가 여러 개인 출구의 균등 index 난수 선택, 난이도 zone suffix 전파
- 이동 가능 상태, 레벨 상·하한, 힘/민첩 상한, 인원, 정·사파, 방파 제한 순서
- `exitRoom` → destination `Room.update` → 위치/index 갱신 → `viewMapData` →
  입장 이벤트 → 도착 알림 → 체력감소 순서
- 기본/사용자 퇴장·진입 스크립트의 `[공]` 치환과 조사 처리
- `viewMapData`의 헤더, 관리자 room index, 설명 설정, 장·단 나침반,
  몹 상태, 같은 방 플레이어 설명을 Rhai에서 조립
- `체력감소 0`도 hazard branch를 실행하고 음수 값은 Python `minusHP`처럼
  체력을 늘린다.
- leader follower 목록은 connection identity 순서를 유지한다. 이동 시작 시
  같은 출발방 follower만 고르고, leader 퇴장 알림에서 제외한 뒤 leader
  명령/프롬프트가 끝나면 같은 출구 명령을 FIFO로 실행한다.
- 원시 입력은 pending-input 경로를 제외하고 Python `stripANSI`를 먼저 적용한
  뒤 빈 입력, `!`/`prevCmd`, 말하기, 사용자 줄임말 순서로 처리한다. 이미
  확장된 줄임말의 첫 명령을 다시 strip하지 않는다.

## 추측하지 않고 이동 전에 중단하는 분기

아래는 현재 Rust 상태만으로 Python 결과를 증명할 수 없어 위치나 체력을
바꾸기 전에 중단하며 Python에 없는 오류 문구를 만들지 않는다.

- runtime에 변경되어 Python `Exits`의 삽입 순서를 복원할 수 없는 출구
- destination의 설치 보관함/바닥 아이템: 통합 `room.objs` 순서와
  `timeofdrop`이 아직 없음
- 치명적 `체력감소`: Python 보험 아이템 선별 drop과 혼수 input callback이
  아직 없음
- destination `자동이동`, 남아 있는 `autoMoveList`
- interactive/위치이동/nested 입장 이벤트
- Python `Mob.update`를 동일하게 실행할 수 없는 mob 상태, 선공 전투와 지속
  combat tick

Rust는 player/mob/item을 아우르는 Python `room.objs` 삽입 순서를 아직 갖지
않는다. 따라서 그 순서가 필요한 destination은 임의 순서를 표시하지 않고 위
preflight에서 차단한다.
