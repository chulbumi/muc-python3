# Design: 초기도우미.rhai Step-Based Conversion

**Feature**: `초기도우미-step-based-conversion`
**Created**: 2026-02-01
**Status**: Design
**References**: `docs/01-plan/features/초기도우미-step-based-conversion.plan.md`

## 1. Architecture Overview

### Current Architecture
```
┌─────────────────────────────────────────────────────────────┐
│  초기도우미.rhai (Monolithic)                                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ start_script() → [814 lines sequential] → finish()     ││
│  │                                                         ││
│  │ get_enter()  ───┐                                       ││
│  │ get_key_input()──┤── Suspend/Resume from TOP           ││
│  │ get_name()     ───┘   (Content repeats!)               ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### Target Architecture
```
┌─────────────────────────────────────────────────────────────┐
│  초기도우미.rhai (Step-Based)                               │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  step1_opening(ob) → wait_enter("step2_farewell")      ││
│  │  step2_farewell(ob) → wait_enter("step3_monster_appear")││
│  │  ... (27 step functions)                               ││
│  │  finish(ob) → finish_script()                          ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  lib/doumi/common.rhai (Shared Utilities)                  │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  wait_enter(next_step)      → throw doumi_suspend      ││
│  │  wait_input(next_step, op)  → throw doumi_suspend      ││
│  │  wait_key_input(next_step, expected) → throw suspend   ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  src/doumi/mod.rs (Executor)                                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  run_doumi(script, ob, current_step, resume, ...)      ││
│  │    → Calls only the specified step function            ││
│  │    → Returns DoumiSuspend with next_step               ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## 2. Step Function Design

### Function Signature Pattern
```rhai
fn step_<name>(ob) {
    // Story content (send_line calls)
    // set_tick calls for delays

    // Suspension point
    wait_enter("next_step_name");
    // OR
    wait_input("next_step_name", "op_name");
    // OR
    wait_key_input("next_step_name", "expected_input");
}
```

### Step Function List (27 Functions)

#### Act 1: The Escape (steps 1-8)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 1 | `step1_opening` | 4-70 | Rain scene, escape begins | `wait_enter("step2_farewell")` |
| 2 | `step2_farewell` | 72-96 | 일주's farewell speech | `wait_enter("step3_monster_appear")` |
| 3 | `step3_monster_appear` | 98-138 | 흑백쌍괴 appears | `wait_key_input("step4_pre_combat", "흑백쌍괴 봐")` |
| 4 | `step4_pre_combat` | 140-168 | Before combat dialogue | `wait_enter("step5_combat_start")` |
| 5 | `step5_combat_start` | 170-228 | Combat begins, 일주 dies | `wait_enter("step6_escape_continue")` |

#### Act 2: The Journey (steps 6-13)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 6 | `step6_escape_continue` | 230-264 | Escape journey | `wait_enter("step7_pursuit")` |
| 7 | `step7_pursuit` | 266-326 | Pursuers arrive | `wait_key_input("step8_combat2_start", "봐")` |
| 8 | `step8_combat2_start` | 328-352 | Second combat begins | `wait_key_input("step9_combat_skill", "흑백쌍괴 쳐")` |
| 9 | `step9_combat_skill` | 354-376 | 혈천섬광 skill | `wait_key_input("step10_combat_victory", "혈천섬광 시전")` |
| 10 | `step10_combat_victory` | 378-430 | Victory, loot | `wait_enter("step11_arrival")` |

#### Act 3: Arrival (steps 11-14)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 11 | `step11_arrival` | 432-464 | Arrival at 낙양 | `wait_enter("step12_death_scene")` |
| 12 | `step12_death_scene` | 466-524 | Death at gate | `wait_enter("step13_aftermath")` |
| 13 | `step13_aftermath` | 526-546 | Aftermath | `wait_enter("step14_interior")` |

#### Act 4: Interior (steps 14-15)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 14 | `step14_interior` | 548-586 | 내실 scene with 노인/왕대협 | `wait_input("step15_name", "get_name")` |

#### Act 5: Character Creation (steps 15-17)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 15 | `step15_name` | 588-590 | Name input | `wait_input("step16_password", "get_password")` |
| 16 | `step16_password` | 592-594 | Password input | `wait_input("step17_gender", "get_sex")` |
| 17 | `step17_gender` | 596-599 | Gender selection | `wait_enter("step18_time_skip")` |

#### Act 6: Time Skip (steps 18-20)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 18 | `step18_time_skip` | 600-616 | 13 years later | `wait_enter("step19_age_speech")` |
| 19 | `step19_age_speech` | 618-634 | Age 18 speech | `wait_enter("step20_feather_gift")` |
| 20 | `step20_feather_gift` | 636-654 | Gift of 매의깃털 | `wait_enter("step21_tutorial_inventory")` |

#### Act 7: Tutorial (steps 21-24)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 21 | `step21_tutorial_inventory` | 656-712 | 소지품 command | `wait_key_input("step22_tutorial_say", "소지품")` |
| 22 | `step22_tutorial_say` | 714-742 | 말 command | `wait_key_input("step23_tutorial_shout", "안녕하세요 말")` |
| 23 | `step23_tutorial_shout` | 744-768 | 외침 command | `wait_key_input("step24_tutorial_summary", "안녕하세요 외침")` |
| 24 | `step24_tutorial_summary` | 770-790 | Tutorial summary | `wait_enter("step25_final")` |

#### Act 8: Departure (steps 25-26)
| Step | Function | Lines | Content | Wait |
|------|----------|-------|---------|------|
| 25 | `step25_final` | 792-812 | Final instructions | `wait_enter("finish")` |
| 26 | `finish` | 813 | End | `finish_script(ob)` |

## 3. Character Creation Design

### Name Input (step15_name → step16_password)
```rhai
fn step15_name(ob) {
    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"그 아이의 이름은 무엇이던가?\"");

    // Wait for user input
    wait_input("step16_password", "get_name");
}

fn step16_password(ob) {
    // Get user's name from resume input
    let name = _doumi_resume_input;

    // Store in ob for later use
    ob["이름"] = name;

    // Display with particle inflection
    send_line(ob, "\x1b[1m왕대협\x1b[0;37;40m이 말합니다. \"" + name + "(이라/라)고 합니다.\"");
    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"음! 좋은 이름이군 그렇다면 암호는??\"");

    wait_input("step17_gender", "get_password");
}
```

### Password Input (step16_password → step17_gender)
```rhai
fn step17_gender(ob) {
    let password = _doumi_resume_input;
    ob["암호"] = password;

    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"그런데 그아이는 남자인가? 여자인가?\"");

    wait_input("step18_time_skip", "get_sex");
}
```

### Gender Input (step17_gender → step18_time_skip)
```rhai
fn step18_time_skip(ob) {
    let sex = _doumi_resume_input;
    ob["성별"] = sex;

    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다 \"자고로 상처입은 짐승이 찾아와도 보살펴");
    send_line(ob, "주는법 집사는 그 아이를 거두어 가르치시오\"");

    // Time skip ellipsis
    send_line(ob, "....");
    send_line(ob, ".......");
    send_line(ob, "..........");
    send_line(ob, ".............");
    send_line(ob, "그로부터 13년후");

    wait_enter("step19_age_speech");
}
```

## 4. Particle Inflection Design

### [공](이라/라) Pattern
The `send_line` function in `src/doumi/mod.rs` already handles:
- `[공]` → Replaces with name from `ob["이름"]`
- `[공](이라/라)` → Name + particle (이/라 based on batchim)
- `[공](아/야)` → Name + particle (아/야 based on batchim)
- `[공](이/가)` → Name + particle (이/가 based on batchim)

Example usage in steps:
```rhai
send_line(ob, "[공](이라/라)고 합니다.");  // "다가타(이라/라)고 합니다."
send_line(ob, "[공](아/야) 잘하는구나 ^^"); // "다가타(아/야) 잘하는구나 ^^"
```

## 5. Tick (Delay) Design

### set_tick() Usage
Each `set_tick(n)` sets delay to n*100ms before next output.

| Step | Original set_tick | Value | Purpose |
|------|-------------------|-------|---------|
| step1_opening | `set_tick(3)` | 300ms | Initial rain scene pacing |
| step5_combat_start | `set_tick(7)` | 700ms | Combat tension |
| step10_combat_victory | `set_tick(3)` | 300ms | After combat |

## 6. File Structure

### Files to Modify
```
lib/doumi/
├── 초기도우미.rhai          # Convert to step-based
├── 초기도우미.rhai.bak      # Backup of original
└── common.rhai              # No changes (already has utilities)
```

### No Backend Changes Required
The existing `src/doumi/mod.rs` and `src/network/client.rs` already support:
- `current_step` parameter for specific function execution
- `next_step` in `DoumiSuspend` for step chaining
- `ob` Map for state preservation
- `_doumi_resume_input` for user input access

## 7. Implementation Order

### Phase 1: Backup & Skeleton
1. `cp lib/doumi/초기도우미.rhai lib/doumi/초기도우미.rhai.bak`
2. Create skeleton with all 27 step function signatures

### Phase 2: Story Migration (Act by Act)
1. **Act 1 (steps 1-5)**: Opening escape scene
2. **Act 2 (steps 6-10)**: Journey and combat
3. **Act 3 (steps 11-13)**: Arrival at 낙양
4. **Act 4 (steps 14)**: Interior scene
5. **Act 5 (steps 15-17)**: Character creation
6. **Act 6 (steps 18-20)**: Time skip
7. **Act 7 (steps 21-24)**: Tutorial
8. **Act 8 (steps 25-26)**: Final instructions

### Phase 3: Testing
1. Test full story flow
2. Test resume at each step
3. Test character creation
4. Test particle inflection

## 8. Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│  Client (Player)                                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Input (name, password, Enter)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  src/network/client.rs                                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  LoginSession                                               ││
│  │    - doumi_step: Option<String>  (e.g., "step16_password")  ││
│  │    - doumi_ob: HashMap<String, String>                     ││
│  │    - doumi_resume_op: Option<String>                       ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ run_doumi_to_result(script, ob, step, resume)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  src/doumi/mod.rs                                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  run_doumi()                                                ││
│  │    1. Load common.rhai + 초기도우미.rhai                    ││
│  │    2. Call specific step: step16_password(ob)              ││
│  │    3. Get suspend info: { next_step: "step17_gender" }     ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ DoumiRunResult::Suspend
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  lib/doumi/초기도우미.rhai                                     │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  fn step16_password(ob) {                                  ││
│  │      let name = _doumi_resume_input;  // From client       ││
│  │      ob["이름"] = name;                                     ││
│  │      send_line(ob, "...");                                 ││
│  │      wait_input("step17_gender", "get_password");          ││
│  │  }                                                          ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## 9. Test Cases

### TC1: Full Story Flow
| Input | Expected Output |
|-------|-----------------|
| Connect, select 초기도우미 | step1_opening executes |
| Press Enter | step2_farewell executes |
| ... (continue through all steps) | finish_script() called |

### TC2: Resume After Name Input
| Input | Expected Output |
|-------|-----------------|
| Enter name "테스터" | step16_password executes, displays "테스터(이라/라)고 합니다" |
| Enter password "1234" | step17_gender executes |
| Enter gender "남" | step18_time_skip executes |

### TC3: Particle Inflection
| Name | Batchim? | Expected Output |
|------|----------|-----------------|
| 다가타 | Yes | 다가타(이라/라) |
| 철수 | No | 철수(가) |

### TC4: Resume at Various Points
| Resume Step | Expected Continuation |
|-------------|----------------------|
| step7_pursuit | Continues to step8_combat2_start |
| step15_name | Prompts for name input |
| step21_tutorial_inventory | Shows inventory tutorial |

## 10. Validation Checklist

- [ ] All 27 step functions defined
- [ ] Each step ends with wait_* or finish_script
- [ ] All send_line calls preserved
- [ ] All set_tick calls preserved
- [ ] Character creation stores values in ob
- [ ] Particle inflection works correctly
- [ ] Resume works at each step
- [ ] Full story flows from start to finish

## 11. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Large file (~800 lines) | Work in small batches, test each act |
| Complex story flow | Keep step names descriptive of content |
| Tutorial commands may break | Commands are external, unchanged |
| Particle inflection bugs | Already tested in 빠른도우미.rhai |

## 12. Next Steps

1. **Do Phase**: `/pdca do 초기도우미-step-based-conversion`
2. Implementation begins with Act 1 (steps 1-5)
3. Test each act before proceeding
4. Final verification with full story run

## 13. References

- Plan Document: `docs/01-plan/features/초기도우미-step-based-conversion.plan.md`
- Working Example: `lib/doumi/빠른도우미.rhai`
- Common Utilities: `lib/doumi/common.rhai`
- Backend: `src/doumi/mod.rs`
