# efun과 Rhai 명령 역할 분리

## 원칙

| 구분 | Rust (efun) | Rhai (cmds/*.rhai) |
|------|-------------|---------------------|
| **역할** | 데이터 접근, 로직, 유틸 | 출력 포맷, 문구, 레이아웃 |
| **이유** | 안정적인 메커니즘, 타입·성능 | 포맷은 자주 바뀜, 스크립트만 수정하면 됨 |

- **efun**: 유용한 함수를 `engine.register_fn("이름", ...)` 형태로 등록.  
  예: `get_all_online_players`, `fill_space`, `strip_ansi`, `view_map_data`, `find_target`, `item_create`, `item_drop` 등.
- **Rhai 명령**: efun을 호출해 **어떤 문자열을 어떻게 보여줄지**만 작성.  
  예: `send_line(ob, "┌───┐")`, `" ★ 총 " + cnt + "명의 ..."`, ANSI 코드 배치, 3열 정렬 등.

## 규칙

1. **새로운 기능**이 필요하면:  
   - 메커니즘/데이터/로직 → Rust에 efun 추가  
   - 화면에 보이는 문장·구도·색·표 → Rhai 쪽에서만 작성

2. **출력 포맷 변경**이 필요하면:  
   - Rhai(cmd/main)만 고치고, efun 시그니처는 유지.  
   - Rust에 하드코딩된 문구/레이아웃은 두지 않는다.

3. **예시 (누구)**
   - efun: `get_all_online_players()` → `[{이름, 무림별호, 성격, 레벨초기화, 소속}, ...]`
   - Rhai: 그 배열을 받아 `fill_space`, `send_line`, 헤더/푸터, "★ 총 N명의 ..." 등 **전부** 포맷팅.

## 참고

- efun 목록/용도: `src/script/mod.rs`의 `create_engine`, `create_engine_with_body_and_output` 등에서 `register_fn` 검색.
- cmds 쪽: `cmds/*.rhai`가 efun을 조합해 출력을 만든다.
