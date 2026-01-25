# AGENTS.md / Driver–Mudlib 구조 체크리스트

> FluffOS/MudOS 스타일: **Driver(엔진)** = 메커니즘, **Mudlib + Rhai** = 정책·동작.  
> 인게임 스크립트는 향후 **전부 Rhai**로 전환 예정 (몹 이벤트, 초기도우미, heart_beat 등).

---

## 1. AGENTS.md / .cursorrules 준수 현황

| 규칙 | 상태 | 비고 |
|------|------|------|
| 파이썬 → Rhai 마이그레이션 | 🔶 진행 중 | cmds/*.rhai 199개, cmds/*.py 190개 잔존 |
| 객체 속성 = 해시맵 | ✅ 충족 | `Object.attr`, `temp`, `inv_stack` (HashMap) |
| efun은 Rust, Rhai에서 호출 | ✅ 충족 | `script/mod.rs`에 `get_skill`, `get_global`, `send_line`, `find_target` 등 다수 `register_fn` |
| 출력·포맷은 Rhai(cmd)만 | 🔶 대체로 충족 | cmds/*.rhai에서 포맷 담당. **예외**: `world/event.rs`, `run_autoscript_chunk` 내 "☞ 그런 아이템이~" 등 하드코딩 |
| Driver / Mudlib 분리 | 🔶 부분 | DRIVER_MUDLIB_ARCHITECTURE.md 설계는 있으나, 인게임 스크립트 상당수가 아직 Rust 파서 |

---

## 2. 엔진(Driver) vs 게임라이브러리(Mudlib) + Rhai

### 2.1 엔진(Driver) — Rust

역할: **메커니즘** (데이터 구조, I/O, 로더, 스케줄링).

| 구성요소 | 경로 | 역할 |
|----------|------|------|
| Object | `src/object/base.rs` | attr/temp/env/objs/inv_stack, get/set, find_obj_name 등 |
| Body | `src/player/body.rs` | HP/MP/스탯/전투 기본 |
| Network | `src/network/` | TCP, Broadcaster, 클라이언트 I/O |
| Loader | `src/loader/` | JSON, `load_script`(Rhai) |
| Data | `src/data/mod.rs` | GlobalData, get_skill, get_murim_config, get_map_path |
| Scheduler | `src/scheduler/` | `CallOutScheduler`, `HeartBeatRegistry` (레지스트리만, **실행 미연동**) |
| Script 엔진 | `src/script/mod.rs` | Rhai Engine, efun 등록, `ScriptStorage`, `load_script_file` (data/script 텍스트) |
| World | `src/world/` | Room, Mob cache/instance, **event.rs**(이벤트/autoscript **Rust 파서**) |
| Master | `src/master/mod.rs` | create/reset/init 등 **스텁** (Mudlib 호출 미구현) |

### 2.2 게임라이브러리(Mudlib) + Rhai

역할: **정책·동작·출력 포맷**.

| 구분 | 경로 | 상태 |
|------|------|------|
| **명령(cmd)** | `cmds/*.rhai` | ✅ Rhai. `register_script_commands`로 등록, `ScriptStorage.execute()` 호출 |
| **라이브러리** | `lib/data/` (loader.rhai, global.rhai), `lib/std/` (object, player, mob, item, room) | ✅ 파일 존재. `lib/std/mob.rhai`, `player.rhai`는 **스켈레톤만**, **엔진에서 호출 안 함** |
| **데이터 로드** | `get_skill`, `get_global`, `get_murim_config`, `get_map_path` 등 | ✅ efun으로 등록, Rhai에서 사용 |

---

## 3. 인게임 스크립트: Rhai 전환 대상

아래는 **아직 Rust에서 파서/로직으로 처리** 중이며, **Rhai로 옮기면** AGENTS.md·Driver–Mudlib 구조에 맞고, 수정·확장이 쉬워짐.

### 3.1 몹 이벤트 (NPC 이벤트)

| 항목 | 현재 | 전환 방향 |
|------|------|-----------|
| **위치** | `src/world/event.rs` | |
| **입력** | `data/mob/*.json`의 `이벤트:대화`, `이벤트:...` (라인 단위 텍스트) | 유지 또는 Rhai 스크립트 경로로 점진 전환 |
| **실행** | `do_event()`: `$출력`, `$위치이동`, `$이벤트확인`, `$이벤트설정`, `$이벤트삭제`, `$무림별호조건`, `$변수확인`, `$아이템주기`, `$스크립트호출`, `$엔터$` 등을 **Rust에서 switch/match로 파싱** | **Rhai 스크립트**: efun(`send_line`, `set_position`, `set_user_event`, `give_item` 등)만 호출하고, 분기·문구·포맷은 전부 Rhai에서 |
| **트리거** | `try_mob_event()`, `try_mob_event_resume()` — `[대상] [명령] [인자]` 해석 후 `do_event` | 트리거 조건·키 매칭은 Driver(efun)에 두고, **실행 블록만 Rhai**로 할 수 있음 |

### 3.2 초기도우미 (Doumi)

| 항목 | 현재 | 전환 방향 |
|------|------|-----------|
| **위치** | `src/network/client.rs` (doumi.json 로드, `DoumiScriptData`, `load_doumi_json`) | |
| **입력** | `data/config/doumi.json` → `도우미메인설정.초기도우미`, `빠른도우미` (라인 단위) | 유지. 실행부만 Rhai로 |
| **실행** | `$줄`, `$틱`, `$키입력`, `$이름획득`, `$암호획득`, `$성별획득` 등 **Rust에서 직접 파싱** | **Rhai 스크립트**: efun(`wait_key_input`, `request_new_name`, `request_new_pass`, `request_sex` 등) 조합. `$틱` → `call_later`/`call_out` 연동 |

### 3.3 Autoscript (data/script/ — 무기강화 등)

| 항목 | 현재 | 전환 방향 |
|------|------|-----------|
| **위치** | `src/world/event.rs` — `run_autoscript_chunk()` | |
| **입력** | `load_script_file("data/script/...")` → 라인 단위 텍스트 | `data/script/`를 **.rhai**로 옮기거나, 기존 텍스트를 Rhai가 한 줄씩 해석하는 형식으로 |
| **실행** | `$출력시작/끝`, `$종료`, `$키입력`, `$단어입력`, `$한줄입력`, `$입력확인`, `$아이템확인`, `$옵션출력`, `$옵션확인`, `$아이템삭제`, `$무기강화` **Rust 하드코딩** | **Rhai**: `$무기강화` 같은 도메인 로직 전부 Rhai + efun. `$아이템확인` 등은 efun(`autoscript_confirm_item`, `autoscript_option_select` 등)으로 단순화 |

### 3.4 몹(Mob) heart_beat / create / reset / init

| 항목 | 현재 | 전환 방향 |
|------|------|-----------|
| **위치** | `src/scheduler/heart_beat.rs`, `src/master/mod.rs` | |
| **heart_beat** | `HeartBeatRegistry`에 등록만 하고, `process_object()`는 `has_script(object_id)` 체크 후 **Rhai `heart_beat()` 호출 안 함**. `GameLoop`에서 `HeartBeatManager.process_all()` **호출 없음** | 1) GameLoop에서 `process_all()` 호출 2) 몹/오브젝트별 `heart_beat.rhai` 또는 `lib/std/mob.rhai`의 `heart_beat()`를 **실제로** 실행 |
| **create/reset/init** | `Master::create`, `Master::reset`, `Master::init` — scope에 인자만 넣고 **Mudlib 쪽 함수 호출 미구현** | 오브젝트 생성/리셋/접촉 시 `lib/std/mob.rhai`, `room.rhai` 등의 `create()`, `reset()`, `init()`를 Rhai로 호출 |

### 3.5 몹 부가 스크립트 (RawMobData 필드)

`world/mob.rs`의 `RawMobData`에는 다음이 **문자열/경로**로만 있고, **실행부는 아직 Rust에 없음** (JSON 파싱·보관만):

| 필드 | 용도 | 현재 실행 위치 | 전환 방향 |
|------|------|----------------|-----------|
| `auto_scripts` | 자동 스크립 | ❌ 미호출 | Rhai 스크립트로 통일, efun으로 호출 |
| `death_script` | 사망 시 | ❌ 미호출 | Rhai `on_die(mob, killer)` 등으로 신규 구현 |
| `combat_start_script`, `combat_script` | 전투 시작/행동 | ❌ 미호출 | Rhai + efun(`start_combat`, `mob_combat_action` 등)으로 신규 구현 |

---

## 4. lib/std/*.rhai와 엔진 연동

| 파일 | 내용 | 엔진 연동 |
|------|------|-----------|
| `lib/std/object.rhai` | create, init, reset 스켈레톤 | ❌ 호출 안 함 |
| `lib/std/player.rhai` | create, on_login, on_logout, heart_beat, add_exp, level_up 등 | ❌ 호출 안 함 |
| `lib/std/mob.rhai` | create, reset, heart_beat, on_attacked, on_die, start_talk | ❌ 호출 안 함 |
| `lib/std/item.rhai`, `room.rhai` | 기본 동작 | ❌ 호출 안 함 |

**필요**:  
- 오브젝트 생성/리셋/접촉/사망/전투 시점에 **Master 또는 직접** `lib/std/*.rhai`의 해당 함수를 Rhai `eval`/`call_fn`으로 실행.  
- `heart_beat`는 `HeartBeatRegistry` + GameLoop와 연동해, 등록된 오브젝트의 `heart_beat()`를 주기적으로 Rhai 호출.

---

## 5. 요약: 이미 Rhai vs 아직 Rust

| 구분 | Rhai (AGENTS.md 방향에 부합) | Rust (전환 대상) |
|------|------------------------------|------------------|
| **명령** | cmds/*.rhai — `register_script_commands` → `ScriptStorage.execute` | (일부 빌트인은 Rust에 두고, 점진 이관) |
| **데이터 접근** | get_skill, get_global, get_murim_config, get_map_path, get_item_data, get_mob_data 등 efun | — |
| **출력** | cmds/*.rhai에서 send_line, 봐, 소지품 등 포맷 | world/event do_event, run_autoscript_chunk 내 일부 문구 |
| **몹 이벤트** | — | `world/event.rs` `do_event` ($출력, $위치이동, $이벤트확인, $아이템주기, $스크립트호출 등) |
| **초기도우미** | — | `network/client.rs` doumi $줄, $틱, $키입력, $이름/암호/성별획득 |
| **Autoscript** | — | `world/event.rs` `run_autoscript_chunk` ($아이템확인, $옵션확인, $무기강화, $아이템삭제 등) |
| **몹 heart_beat** | lib/std/mob.rhai 스켈레톤 | `HeartBeatRegistry.process_object`가 Rhai 호출 안 함, GameLoop 연동 없음 |
| **create/reset/init** | lib/std/*.rhai 스켈레톤 | `Master::create/reset/init`가 Mudlib 호출 안 함 |

---

## 6. 권장 조치 (우선순위)

1. **Master apply와 lib/std 연동**  
   - `create`, `reset`, `init`에서 오브젝트 타입별 `lib/std/*.rhai`의 `create()`, `reset()`, `init()`를 Rhai로 호출.

2. **Heart beat 실제 동작**  
   - GameLoop에서 `HeartBeatManager.process_all()` 호출.  
   - `process_object`에서 해당 오브젝트의 `heart_beat()` Rhai 함수 실행.  
   - 몹 스폰/리젠 시 `set_heart_beat(true)` 등으로 등록.

3. **몹 이벤트의 Rhai 전환**  
   - `$출력`, `$위치이동`, `$이벤트확인` 등 분기와 문구를 Rhai 스크립트로.  
   - efun: `event_output`, `event_move`, `event_set`, `event_give_item`, `event_call_script` 등으로 최소한의 메커니즘만 Rust에 두기.

4. **초기도우미 Rhai 전환**  
   - `$이름획득`, `$암호획득`, `$성별획득`, `$키입력`, `$틱` 등을 efun + Rhai 스크립트로.  
   - doumi.json은 “진입점 스크립트 경로 + 옵션” 정도만 두고, 실제 플로우는 Rhai.

5. **Autoscript Rhai 전환**  
   - `$무기강화`, `$아이템확인`, `$옵션확인` 등 도메인 로직을 Rhai로.  
   - efun으로 `autoscript_*` 계열만 제공.

6. **출력 문구 정리**  
   - `world/event.rs`, `run_autoscript_chunk` 안의 "☞ 그런 아이템이~", "무기강화를 종료합니다" 등을 Rhai로 이동하거나 efun 인자/반환으로 넘기기.

---

## 7. 참고 문서

- `AGENTS.md`, `.cursorrules`
- `docs/DRIVER_MUDLIB_ARCHITECTURE.md`
- `docs/EFUN_RHAI_CONVENTION.md`
- `docs/ARCHITECTURE_REVIEW.md`
