# Rhai API 변환 현황

## .clinerules 요구사항
- 모든 파이썬 코드를 마이그레이션 해야 함 (190개 .py 파일)
- cmds/ 디렉토리의 파이썬 코드는 실행시간에 업데이트하는 구조
- 모든 동작은 파이썬에서 기대하던 대로 동작해야 함

## 현재 구현된 API
- [x] `random(min, max)` - 랜덤 숫자
- [x] `abs(n)` - 절대값
- [x] `contains(s, pattern)` - 문자열 포함
- [x] `starts_with(s, pattern)` - 접두사 확인
- [x] `ends_with(s, pattern)` - 접미사 확인
- [x] `trim(s)` - 공백 제거
- [x] `length(s)` - 문자열 길이
- [x] `len(arr)` - 배열 길이
- [x] `ansi(msg, conv)` - ANSI 색상 코드 변환
- [x] `han_iga(name)` - 한국어 조사 (이/가)
- [x] `han_eul(name)` - 한국어 조사 (을/를)
- [x] `han_wa(name)` - 한국어 조사 (와/과)
- [x] `get_player_data(ob, key)` - 플레이어 데이터 조회
- [x] `set_player_data(ob, key, value)` - 플레이어 데이터 설정
- [x] `print(s)` - 디버그 출력

## 필요한 API (미구현)

### 플레이어 액션
- [ ] `send_line(ob, msg)` - 플레이어에게 메시지 전송 (ob.sendLine)
- [ ] `send_room(ob, msg)` - 방에 있는 모두에게 메시지 전송 (ob.sendRoom)
- [ ] `enter_room(ob, room, type1, type2)` - 방 이동 (ob.enterRoom)
- [ ] `save(ob)` - 플레이어 저장 (ob.save)

### 데이터 접근
- [ ] `get(ob, key)` - 객체 속성 조회 (ob['key'])
- [ ] `set(ob, key, value)` - 객체 속성 설정 (ob[key] = value)
- [ ] `get_int(ob, key)` - 정수 속성 조회 (getInt(ob[key]))
- [ ] `get_string(ob, key)` - 문자열 속성 조회 (ob.getString(key))

### 환경/방 정보
- [ ] `check_attr(env, attr)` - 방 속성 확인 (ob.env.checkAttr)
- [ ] `get_env(ob)` - 현재 방 객체 (ob.env)

### 채널/플레이어 목록
- [ ] `get_players(channel)` - 채널의 모든 플레이어 (ob.channel.players)
- [ ] `find_player(name)` - 이름으로 플레이어 찾기

### 유틸리티
- [ ] `fill_space(width, s)` - 문자열 포맷팅
- [ ] `strip_ansi(s)` - ANSI 코드 제거
- [ ] `to_int(s)` - 문자열을 정수로 변환

## 변환 현황
- 완료: 11/190 스크립트 (5.8%)
- 남음: 179개 파이썬 스크립트

## 다음 단계
1. 필요한 API 함수들을 모두 구현
2. 파이썬 스크립트를 Rhai로 일괄 변환
3. 변환된 스크립트들이 파이썬과 동일하게 동작하는지 테스트
