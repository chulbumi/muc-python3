# Python 주기 동작 이관 현황

기준 소스는 `loop.py`, `client.py`, `objs/player.py`, `objs/room.py`,
`objs/mob.py`이다. 아래에서 구현 완료라고 적은 항목은 해당 Python 실행 경로와
현재 Rust 코드가 직접 연결되어 있고 단위 회귀 테스트가 있는 범위만 뜻한다.

## Python에서 실제 실행되는 순서

`server.py`는 객체를 로드한 뒤 `Loop()`를 만들고 reactor를 실행한다.
`Loop.run()`은 약 1초마다 다음 순서로 다시 예약된다.

1. 60회마다 `Room.Zones` 중 한 존을 선택해 그 존의 모든 방을 `update()`한다.
2. `Client.players`의 복사본을 순회한다.
   - `INACTIVE` 상태에서 마지막 수신 후 10초가 지나면 안내 후 연결을 끊는다.
   - 그 밖의 상태에서 마지막 수신 후 180초가 지나면 안내 후 연결을 끊는다.
   - `ACTIVE` 플레이어만 `Player.update()`를 호출한다.
   - 연결 플레이어가 위치한 방을 중복 없이 모은다.
3. 접속자가 한 명이라도 있으면 위에서 모은 방을 `Room.update()`하고,
   이동 몹 목록의 첫 몹 하나를 `updateMoving()`한다.

`client.py:dataReceived()`는 완성된 명령줄이 아니라 바이트 chunk를 받을 때마다
idle 기준 시각을 갱신한다.

`Player.update()`의 순서는 다음과 같다.

1. 초당 명령 수 초과 연결 해제 검사 후 `cmdCnt` 초기화
2. 플레이어 tick 및 나이 tick 증가, 나이 증가 이벤트 검사
3. 60 tick마다 무림별호 이벤트 조건 검사
4. 600 tick마다 저장
5. 전투면 한 라운드 진행, 사망이면 사망 단계 진행 후 즉시 반환,
   그 밖의 상태면 진행 중 공격 무공과 남은 target 정리
6. 30 tick마다 현재 상태에 따라 체력/내공 회복
   (`STAND` 10%, `REST` 20%, `FIGHT` 5%)
7. 서 있거나 전투 중이면 자동 체력약/내공약 사용
8. 방어 무공 만료 검사

`Room.update()`는 방 안의 아이템과 몹을 삽입 순서대로 갱신한다. 몹은 60 tick마다
회복하고, 대화/선공/아이템 리젠/시체→리젠→재생/방어 무공 만료를 처리한다.

## 현재 Rust에서 직접 연결된 범위

- `GameLoop`는 별도로 만든 빈 `Vec<Player>`가 아니라 실제 authoritative 저장소인
  `Broadcaster.clients`의 `Client.player`를 직접 순회한다.
- 네트워크 입력은 Python처럼 delimiter 파싱 전, 수신 chunk 시점에 idle 시각을
  갱신한다.
- 10초/180초 idle 판정과 Python의 실제 `sendLine()` 결과 바이트를 전달한 뒤
  연결 종료 sentinel을 보낸다.
- `ACTIVE`인 실제 접속 플레이어의 `Body.tick`을 매초 갱신한다.
- 600 tick 저장은 Python과 같이 회복보다 먼저 실행한다. 저장 경로는 설정으로
  주입할 수 있어 테스트는 실제 `data/user`가 아닌 임시 디렉터리만 사용한다.
- 30 tick 상태별 회복은 Python의 정수 버림 및 최대값 clamp를 그대로 사용한다.
- 비전투/비사망 상태에서 남은 공격 무공과 target을 정리한다.
- 기존 call-out scheduler도 같은 실제 loop에서 계속 처리한다.

## 아직 이관되지 않은 정확한 차이

다음 항목은 현재 Rust 상태 모델로 Python 결과를 재현할 근거가 부족하므로 임의로
동작을 만들지 않았다.

- 전투 명령은 현재 입력 시 즉시 한 라운드를 수행한다. Python은 target과 몹의
  dex/skill 진행 상태를 보존하면서 `Player.update()`에서 매초 계속 진행한다.
  Rust `Body.targets`는 WorldState의 `MobInstance` identity와 연결되어 있지 않다.
- Python의 플레이어 사망은 0~9의 열 단계, `낙양성:7` 이동, 입력 차단,
  33% 체력으로 `REST` 복귀를 수행한다. 현재 Rust의 기존 `do_death()`는 단계와
  문구가 다르므로 game loop에서 호출하지 않는다.
- 초당 명령 경고/강제 종료는 Rust 명령 처리기가 `cmd_cnt`를 증가시키지 않아
  아직 판정할 수 없다.
- 나이 증가와 무림별호 이벤트, 자동 체력약/내공약은 아직 tick에 연결되지 않았다.
- 방어 무공은 Python의 종료 시각과 해제 스크립/보너스 환원 상태가 모두 필요하다.
- Rust는 Python `room.objs`의 player/mob/item 통합 삽입 순서를 보존하지 않는다.
  따라서 방 아이템 소멸, 몹 대화/선공/리젠, 이동 몹 순서를 아직 같은 방식으로
  갱신할 수 없다.
- 기존 `MobCache::update_respawns()`는 Python의 `시체 * REGEN_MULTIPLY` 단계와
  `리젠` 배수/360초 clamp, 방 메시지를 재현하지 않으므로 주기 loop에 연결하지
  않았다.

이 문서는 위 미이관 항목을 완료로 간주하는 근거로 사용하면 안 된다.

## 2026-07-10 검증

- `server::game_loop::tests` 6개 통과
- 실제 `murim_server`를 별도 포트로 실행하고 미로그인 raw socket을 연결한 결과,
  10초 뒤 Python과 같은 제한시간 문구 및 끝의 빈 줄을 수신하고 TCP가 닫힘을 확인
- 저장 회귀 테스트는 임시 디렉터리를 주입하며 `data/user`를 사용하지 않음
