# Rhai API 변환 현황

## efun vs 출력 포맷
- **efun**: 데이터·로직·유틸만 Rust에 등록. (이 목록의 API들이 efun.)
- **출력 포맷**: 문구·레이아웃·ANSI 등은 Rhai(cmd/main)에서만.  
→ 자세한 원칙: [docs/EFUN_RHAI_CONVENTION.md](../docs/EFUN_RHAI_CONVENTION.md)

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

## API 검증 상태

### 플레이어 액션
- [x] `send_line(ob, msg)` - Rhai 출력 수집
- [x] `send_room(ob, msg)` - Rhai 출력 수집 후 방 인덱스 라우팅
- [x] `enter_room`에 해당하는 이동 efun - 이동 전이와 검사는 Rust, 출력은 Rhai
- [x] 저장 efun/명령 - Python JSON 형식 저장

### 데이터 접근
- [x] `get(ob, key)` - 객체 속성 조회 (ob['key'])
- [x] `set(ob, key, value)` - 객체 속성 설정/상태 전이 efun
- [x] `get_int(ob, key)` - 정수 속성 조회
- [x] `get_string(ob, key)` - 문자열 속성 조회

### 환경/방 정보
- [x] `check_attr(env, attr)` - 방 속성 확인
- [x] `get_env(ob)` - 현재 방/위치 데이터 접근

### 채널/플레이어 목록
- [x] 채널별 ordered membership 조회
- [x] 같은 방 플레이어 조회는 방 인덱스 사용
- [x] 전역 플레이어 조회는 Python 전역 명령에만 제한

### 유틸리티
- [x] `fill_space(width, s)` - 문자열 포맷팅
- [x] `strip_ansi(s)` - ANSI 코드 제거
- [x] `to_int(s)` - 문자열을 정수로 변환

## 변환 현황
- `.rhai` 파일: 207개, Python 기준 파일: 190개
- 파일 존재만으로 완료를 판정하지 않으며, placeholder와 상태 전이를 계속 대조 중

## 다음 단계
1. 필요한 API 함수들을 모두 구현
2. 파이썬 스크립트를 Rhai로 일괄 변환
3. 변환된 스크립트들이 파이썬과 동일하게 동작하는지 테스트
