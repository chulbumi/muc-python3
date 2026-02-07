# Plan: 초기도우미.rhai Step-Based Conversion

**Feature**: `초기도우미-step-based-conversion`
**Created**: 2026-02-01
**Status**: Plan

## 1. Background

The current `lib/doumi/초기도우미.rhai` script is a monolithic 814-line script that runs sequentially from beginning to end. This causes issues:

- Content duplication when resuming (story repeats from beginning)
- No step-based navigation (must restart from top each time)
- Cannot pause/resume at specific story points

The **빠른도우미.rhai** script has already been successfully converted to a step-based pattern and is working correctly. This proven architecture should be applied to `초기도우미.rhai`.

## 2. Objectives

### Primary Objective
Convert `초기도우미.rhai` from monolithic sequential execution to step-based suspend/resume architecture.

### Success Criteria
1. Each story segment runs only once (no duplication)
2. Resume continues from exact suspension point
3. Story flow preserved exactly as original
4. Character creation (name/password/gender) works correctly
5. End of script properly triggers game entry

## 3. Current State Analysis

### Existing Script Structure
The `초기도우미.rhai` contains:
- **Opening story**: Escape scene with 일주, 소룡, 아이 (lines 4-228)
- **Combat scene**: 흑백쌍괴 battle (lines 230-430)
- **Arrival scene**: 낙양 arrival, death (lines 432-546)
- **Interior scene**: 내실 with 노인/왕대협 (lines 548-616)
- **Character creation**: get_name(), get_password(), get_sex() (lines 586-598)
- **Tutorial**: Equipment, communication commands (lines 618-812)
- **Finish**: finish_script() call (line 813)

### Existing Wait Points
| Line | Function | Purpose |
|------|----------|---------|
| 70 | get_enter() | After opening rain scene |
| 96 | get_enter() | After 일주 farewell speech |
| 110 | get_key_input("흑백쌍괴 봐") | Monster appears |
| 138 | get_enter() | After monster intro |
| 168 | get_enter() | Before combat starts |
| 228 | get_enter() | After 일주's death |
| 264 | get_enter() | After escape intro |
| 326 | get_key_input("봐") | Continue to combat |
| 352 | get_key_input("흑백쌍괴 쳐") | Start combat |
| 376 | get_key_input("혈천섬광 시전") | Skill use |
| 430 | get_enter() | After combat victory |
| 464 | get_enter() | After arrival scene |
| 524 | get_enter() | At death scene |
| 546 | get_enter() | After death |
| 588 | get_name() | Character name input |
| 594 | get_password() | Password input |
| 598 | get_sex() | Gender selection |
| 616 | get_enter() | After 13 years time skip |
| 634 | get_enter() | After age 18 speech |
| 654 | get_enter() | After feather gift |
| 678 | get_key_input("소지품") | Tutorial: inventory |
| 712 | get_enter() | After inventory demo |
| 730 | get_key_input("안녕하세요 말") | Tutorial: 말 command |
| 742 | get_enter() | After 말 demo |
| 756 | get_key_input("안녕하세요 외침") | Tutorial: 외침 command |
| 768 | get_enter() | After 외침 demo |
| 790 | get_enter() | After tutorial summary |
| 812 | get_enter() | Final instructions |
| 813 | finish_script() | End |

## 4. Proposed Step Structure

Based on the existing `빠른도우미.rhai` pattern, the script will be divided into these step functions:

| Step Function | Purpose | Next Step |
|---------------|---------|-----------|
| step1_opening | Initial rain/escape scene | step2_farewell |
| step2_farewell | 일주's farewell | step3_monster_appear |
| step3_monster_appear | 흑백쌍괴 introduction | step4_pre_combat |
| step4_pre_combat | Before combat dialogue | step5_combat_start |
| step5_combat_start | Combat begins | step6_combat_skill |
| step6_combat_skill | Skill use scene | step7_combat_victory |
| step7_combat_victory | After combat | step8_escape_continue |
| step8_escape_continue | Escape journey | step9_pursuit |
| step9_pursuit | Pursuers arrive | step10_combat_start2 |
| step10_combat_start2 | Second combat | step11_after_combat2 |
| step11_after_combat2 | Combat victory | step12_arrival |
| step12_arrival | Arrival at 낙양 | step13_death_scene |
| step13_death_scene | Death at gate | step14_interior |
| step14_interior | 내실 scene | step15_name |
| step15_name | Character name input | step16_password |
| step16_password | Password input | step17_gender |
| step17_gender | Gender selection | step18_time_skip |
| step18_time_skip | 13 years later | step19_age_speech |
| step19_age_speech | Age 18 instructions | step20_feather_gift |
| step20_feather_gift | Gift of 매의깃털 | step21_tutorial_inventory |
| step21_tutorial_inventory | 소지품 command | step22_tutorial_say |
| step22_tutorial_say | 말 command | step23_tutorial_shout |
| step23_tutorial_shout | 외침 command | step24_tutorial_summary |
| step24_tutorial_summary | Tutorial summary | step25_final |
| step25_final | Final instructions | finish |
| finish | End script | - |

**Total: 27 step functions**

## 5. Implementation Strategy

### Phase 1: Preparation
1. Copy `초기도우미.rhai` to `초기도우미.rhai.bak` (backup)
2. Create new step-based structure skeleton

### Phase 2: Step Function Conversion
1. Split story content into step functions
2. Each step function:
   - Takes `ob` as parameter
   - Contains story content (send_line calls)
   - Ends with `wait_enter()` or `wait_input()` or `wait_key_input()`
3. Replace old `get_enter()`, `get_key_input()`, `get_name()`, `get_password()`, `get_sex()` with new equivalents

### Phase 3: Character Creation Integration
The character creation section (lines 586-598) will use `wait_input()` pattern:
```rhai
fn step15_name(ob) {
    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"왕대협~  아이의 상태는 어떤가??\"");
    // ... dialogue ...
    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"그 아이의 이름은 무엇이던가?\"");
    wait_input("step16_password", "get_name");
}

fn step16_password(ob) {
    let name = _doumi_resume_input;  // User's name input
    ob["이름"] = name;
    send_line(ob, "\x1b[1m왕대협\x1b[0;37;40m이 말합니다. \"" + name + "(이라/라)고 합니다.\"");
    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"음! 좋은 이름이군 그렇다면 암호는??\"");
    wait_input("step17_gender", "get_password");
}
```

### Phase 4: Testing
1. Test full story flow
2. Verify resume works at each step
3. Verify character creation works
4. Verify particle inflection ([공](이라/라), [공](아/야)) works correctly

## 6. Dependencies

### Existing Components (Already Working)
- `src/doumi/mod.rs` - run_doumi() with step-based execution
- `src/network/client.rs` - Session management with doumi_step tracking
- `lib/doumi/common.rhai` - wait_enter(), wait_input(), wait_key_input() functions

### No Changes Needed
The Rust backend already supports the step-based pattern from the `빠른도우미.rhai` conversion.

## 7. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Story content mis-segmentation | Medium | Carefully review each story segment boundary |
| Tutorial commands not working | Low | Commands are external to script; unchanged |
| Particle inflection issues | Low | Already tested with 빠른도우미.rhai |
| Too many steps causing complexity | Low | 27 steps is manageable; each is small |

## 8. Timeline Estimate

- **Step skeleton creation**: 30 minutes
- **Content migration to steps**: 2 hours
- **Testing and refinement**: 1 hour
- **Total**: ~3.5 hours

## 9. Next Steps

1. Create Design document: `/pdca design 초기도우미-step-based-conversion`
2. Implement conversion
3. Test with full flow
4. Verify resume functionality

## 10. References

- Working example: `lib/doumi/빠른도우미.rhai`
- Common functions: `lib/doumi/common.rhai`
- Backend: `src/doumi/mod.rs`
