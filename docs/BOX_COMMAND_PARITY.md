# 보관함 `넣어` / `꺼내` Python 대조 기록

기준 소스는 `cmds/넣어.py`, `cmds/꺼내.py`, `objs/box.py`,
`objs/room.py`, `objs/object.py` 이다. 기존 Rust/Rhai의 가상 은전 출금과
"구현 중"이라는 문구는 Python에 없어 제거했다.

## 현재 재검증한 범위

- Box와 플레이어 모두 개별 `Object` identity의 `objs` 순서를 사용한다.
  `inv_stack`을 Box 명령에서 생성하거나 수량으로 압축하지 않는다.
- `Object.insert()`의 앞쪽 삽입, Arc identity 제거, JSON 로드 시 배열
  역순, 현재 Vec 순서의 저장을 고정했다.
- `모두`, `속성아이템`, `약초`, `속성무기`, `속성방어구`,
  이름/반응이름/순번/숫자 선택의 필터와 검사 순서를 이관했다.
- `넣어` 선택 분기에서 공용보관함 제한을 첫 아이템에만
  검사하는 Python 동작, 수량 0/음수가 모든 일치 항목을 옮기는
  동작, bulk/selected의 ONEITEM 보관 소유자 문자열 차이를
  보존했다.
- `꺼내` 수량 한계 비교는 Python처럼 `>`이다. 한계와 같을 때
  한 개를 더 허용하며, 숫자 `0`은 `box.objs[-1]`로 마지막
  항목을 선택한다. 무게 검사가 수량 검사보다 먼저다.
- Box 은전은 출금 기능이 아니라 `addMoney()`를 통한 수량 확장만
  있다. 최대치 도달 시 남은 은전을 반환하는 소모량을 재현했다.
- Box 로드는 `data/box/<index>.json`을 읽지만, Python `save()`이
  `self.path + '.json'`에 쓰는 결과인 `<index>.json.json` 저장 경로를
  그대로 보존했다.
- JSON의 `반응이름`, `옵션`, `아이템속성`은 배열/문자열 형태를
  보존한다. Box 로드는 Player와 달리 문자열 `반응이름`을
  배열로 바꾸지 않는다.
- `설치리스트`가 JSON 문자열이면 Python `Room.create()`가 문자를
  한 글자씩 순회하는 현재 동작도 추측해 배열로 고치지 않았다.
- 모든 문구, ANSI, 조사, 개수 표시는 `cmds/넣어.rhai` / `cmds/꺼내.rhai`
  안에만 있다.
- 관찰자는 `WorldState` 같은 방 순서 인덱스와 이름→연결 인덱스로만
  수집한다. Rhai가 Python `sendRoom()`의 `\r\n<msg>\r\n`과 수신자별
  `lpPrompt()` 바이트를 완성해 opaque 연결 토큰으로 전송한다.
  `get_all_online_players()`와 전체 client 순회는 사용하지 않는다.

## 의도적으로 성공시키지 않는 분기

1. Rust는 아직 player/mob/item/Box 전체의 하나인 Python `room.objs`
   삽입 순서를 갖고 있지 않다. 대상 이름에 경쟁하는 non-Box가
   있으면 첫 객체를 추측하지 않고 mutation 전 `unsupported`로 중단한다.
2. Python `넣어.py` `약초` bulk의 동명 집계가 1개인 경우,
   포맷 `%s`는 3개인데 인수는 2개라서 아이템/ONEITEM 변경 후
   `TypeError`가 발생한다. 예외 후 부분 mutation을 Rust 명령 경로로
   안전하게 재현할 수 없어 계획 단계에서 전체를 중단한다.
3. 인덱스가 없는 child나 잘못된 Box `아이템` JSON은 조용히
   누락/저장하지 않고 mutation 전 중단한다. Python의 예외 중간 상태를
   정상 성공으로 바꾸지 않는다.
4. `설치.rhai`와 이동 `viewMapData` Box 표시는 이 작업의 이관 범위가
   아니며 아직 별도 미이관 범위다. 현재 이동 로직은 `설치리스트`가
   있는 목적지를 안전 차단하므로 상자방 전체 진입은 아직 완료가 아니다.

## 검증

- Box 로직/Rhai/저장 회귀: 13/13
- raw 관찰자 `sendRoom + lpPrompt` 전송: 1/1
- inventory 개별-object 호환 회귀: 12/12
- `only_python_global_list_commands_scan_all_online_players`: 통과
- `cargo check --all-targets`: 통과
- `cargo fmt --all -- --check`: 통과
