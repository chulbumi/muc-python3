# MUD Server Test Report

**Generated:** 2026-02-08T04:27:10.014299

**Test Duration:** 2026-02-08T04:25:50.517047 to 2026-02-08T04:27:10.014299

## Test Configuration

- **Host:** localhost
- **Python Server Port:** 9900
- **Rust Server Port:** 9999
- **Number of Characters:** 2
- **Base Password:** test1234

---

## Test Summary

- **Total Tests:** 72
- **Passed Tests:** 36
- **Failed Tests:** 36

**Pass Rate:** 50.0%

## Server Status

- **Python:** ONLINE

- **Rust:** ONLINE

---

## Comparison Results

- **Total Comparisons:** 36
- **Matching Outputs:** 0
- **Different Outputs:** 36

### Detailed Comparisons

#### Command: `능력치` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=20
- Line 1: Python='', Rust='┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑'
- Line 2: Python='(missing)', Rust='▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │'
- Line 3: Python='(missing)', Rust='┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥'
- Line 4: Python='(missing)', Rust='│ [레  벨]       [     1] │ [나  이]              18 │'
- Line 5: Python='(missing)', Rust='│ [체  력] 450/0           │ [성  격] ----------      │'
- Line 6: Python='(missing)', Rust='│ [  힘  ]     0 +     15 │ [성  별]                 │'
- Line 7: Python='(missing)', Rust='│ [맷  집]      0 +      0 │ [소  속] ----------      │'
- Line 8: Python='(missing)', Rust='│ [민  첩]               0 │ [직  위] ----------      │'
- Line 9: Python='(missing)', Rust='│ [命  中]               0 │ [回  避]               0 │'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `점수` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=20
- Line 1: Python='', Rust='┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑'
- Line 2: Python='(missing)', Rust='▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │'
- Line 3: Python='(missing)', Rust='┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥'
- Line 4: Python='(missing)', Rust='│ [레  벨]       [     1] │ [나  이]              18 │'
- Line 5: Python='(missing)', Rust='│ [체  력] 450/0           │ [성  격] ----------      │'
- Line 6: Python='(missing)', Rust='│ [  힘  ]     0 +     15 │ [성  별]                 │'
- Line 7: Python='(missing)', Rust='│ [맷  집]      0 +      0 │ [소  속] ----------      │'
- Line 8: Python='(missing)', Rust='│ [민  첩]               0 │ [직  위] ----------      │'
- Line 9: Python='(missing)', Rust='│ [命  中]               0 │ [回  避]               0 │'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `무공` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=10
- Line 1: Python='', Rust='━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
- Line 2: Python='(missing)', Rust='◁ 당신의 무공 ▷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
- Line 3: Python='(missing)', Rust='───────────────────────────────────────'
- Line 4: Python='(missing)', Rust='☞ 깨우친 무공이 없습니다.'
- Line 5: Python='(missing)', Rust='───────────────────────────────────────'
- Line 6: Python='(missing)', Rust='▷ 비전'
- Line 7: Python='(missing)', Rust='☞ 오의를 깨우친 무공이 없습니다.'
- Line 8: Python='(missing)', Rust='━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
- Line 9: Python='(missing)', Rust=''

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  | X |
| 아이템 |  |  |

---

#### Command: `소지품` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=8
- Line 1: Python='', Rust='━━━━━━━━━━━━━━━━━'
- Line 2: Python='(missing)', Rust='◁     소     지     품     ▷'
- Line 3: Python='(missing)', Rust='─────────────────'
- Line 4: Python='(missing)', Rust='☞ 아무것도 없습니다.'
- Line 5: Python='(missing)', Rust='─────────────────'
- Line 6: Python='(missing)', Rust='▶ 은전 :                10000 개'
- Line 7: Python='(missing)', Rust='─────────────────'
- Line 8: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  | X |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `누구` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=7
- Line 1: Python='', Rust='┌─────────────────────────────────────┐'
- Line 2: Python='(missing)', Rust='│ ◁     무       림       크       래       프       트      １-１      ▷ │'
- Line 3: Python='(missing)', Rust='└─────────────────────────────────────┘'
- Line 4: Python='(missing)', Rust='[무명객]테스터러스트'
- Line 5: Python='(missing)', Rust='──────────────────────────────────────'
- Line 6: Python='(missing)', Rust='★ 총 1명의 무림인이 활동하고 있습니다.'
- Line 7: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `봐` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=14
- Line 1: Python='', Rust='[[[[] 낙양성 []]]]'
- Line 2: Python='(missing)', Rust=''
- Line 3: Python='(missing)', Rust='산들바람에 여유롭게 흔들리는 꽃들은 주위배경과 절묘하게 조'
- Line 4: Python='(missing)', Rust='화되어 있고 아담하게 꾸며진 정원 사이로 머리가 희끗 희끗한'
- Line 5: Python='(missing)', Rust='초로가 한가롭게 산책을 하고 있다. 무림에 입문한 사람을 위한'
- Line 6: Python='(missing)', Rust='『안내문』이 붙어 있다.'
- Line 7: Python='(missing)', Rust=''
- Line 8: Python='(missing)', Rust=''
- Line 9: Python='(missing)', Rust='◁○     〔서ː초보수련장〕쪽으로 이동할 수 있습니다.'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  | X |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `지도` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=6
- Line 1: Python='', Rust='낙양성'
- Line 2: Python='(missing)', Rust=''
- Line 3: Python='(missing)', Rust=''
- Line 4: Python='(missing)', Rust='◁○     〔서ː초보수련장〕쪽으로 이동할 수 있습니다.'
- Line 5: Python='(missing)', Rust=''
- Line 6: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  | X |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `어디` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='테스터러스트     ▷ 낙양성'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  | X |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `도움말` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=5
- Line 1: Python='', Rust='=== 도움말 ==='
- Line 2: Python='(missing)', Rust='이동: 북(ㅂ) 남(ㄴ) 동(ㄷ) 서(ㅅ) 위(ㅇ) 아래(ㅁ) 북서(nw) 북동(ne) 남서(sw) 남동(se)'
- Line 3: Python='(missing)', Rust='보기: look, 봐, 보'
- Line 4: Python='(missing)', Rust='종료: quit, 끝, 종료'
- Line 5: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `저장` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='* 저장 되었습니다.'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `move_동` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 동쪽으로 갈 수 없습니다.'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `move_서` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=13
- Line 1: Python='', Rust='[[[[] 하남성 낙양 []]]]'
- Line 2: Python='(missing)', Rust=''
- Line 3: Python='(missing)', Rust='따스한 햇살을 받으며 포근하게 자리잡고 있는 아담한 집이 한'
- Line 4: Python='(missing)', Rust='채 있다. 주인의 성격을 대변하는듯 깔끔하게 정돈되어 있고,'
- Line 5: Python='(missing)', Rust='마당에는 아름다운 꽃들이 흐드러지게 피어있다.'
- Line 6: Python='(missing)', Rust=''
- Line 7: Python='(missing)', Rust='△'
- Line 8: Python='(missing)', Rust='○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.'
- Line 9: Python='(missing)', Rust='▽'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `move_남` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=15
- Line 1: Python='', Rust='[[[[] 하남성 낙양 []]]]'
- Line 2: Python='(missing)', Rust=''
- Line 3: Python='(missing)', Rust='길가에는 한적한 기운이 서려 있고 따스한 햇빛이 조용히 내리'
- Line 4: Python='(missing)', Rust='고 있다. 성곽에는 한가로운 들꽃들이 듬성듬성 자라나 남북으'
- Line 5: Python='(missing)', Rust='로 이어지고 있는 매우 운치 있는 곳이다.'
- Line 6: Python='(missing)', Rust=''
- Line 7: Python='(missing)', Rust='△'
- Line 8: Python='(missing)', Rust='○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.'
- Line 9: Python='(missing)', Rust='▽'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `move_북` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=13
- Line 1: Python='', Rust='[[[[] 하남성 낙양 []]]]'
- Line 2: Python='(missing)', Rust=''
- Line 3: Python='(missing)', Rust='따스한 햇살을 받으며 포근하게 자리잡고 있는 아담한 집이 한'
- Line 4: Python='(missing)', Rust='채 있다. 주인의 성격을 대변하는듯 깔끔하게 정돈되어 있고,'
- Line 5: Python='(missing)', Rust='마당에는 아름다운 꽃들이 흐드러지게 피어있다.'
- Line 6: Python='(missing)', Rust=''
- Line 7: Python='(missing)', Rust='△'
- Line 8: Python='(missing)', Rust='○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.'
- Line 9: Python='(missing)', Rust='▽'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `move_위` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 위쪽으로 갈 수 없습니다.'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `move_아래` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 아래쪽으로 갈 수 없습니다.'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `move_봐` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=14
- Line 1: Python='', Rust='[[[[] 하남성 낙양 []]]]'
- Line 2: Python='(missing)', Rust=''
- Line 3: Python='(missing)', Rust='따스한 햇살을 받으며 포근하게 자리잡고 있는 아담한 집이 한'
- Line 4: Python='(missing)', Rust='채 있다. 주인의 성격을 대변하는듯 깔끔하게 정돈되어 있고,'
- Line 5: Python='(missing)', Rust='마당에는 아름다운 꽃들이 흐드러지게 피어있다.'
- Line 6: Python='(missing)', Rust=''
- Line 7: Python='(missing)', Rust='△'
- Line 8: Python='(missing)', Rust='○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.'
- Line 9: Python='(missing)', Rust='▽'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `상태` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=20
- Line 1: Python='', Rust='┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑'
- Line 2: Python='(missing)', Rust='▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │'
- Line 3: Python='(missing)', Rust='┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥'
- Line 4: Python='(missing)', Rust='│ [레  벨]       [     1] │ [나  이]              18 │'
- Line 5: Python='(missing)', Rust='│ [체  력] 450/0           │ [성  격] ----------      │'
- Line 6: Python='(missing)', Rust='│ [  힘  ]     0 +     15 │ [성  별]                 │'
- Line 7: Python='(missing)', Rust='│ [맷  집]      0 +      0 │ [소  속] ----------      │'
- Line 8: Python='(missing)', Rust='│ [민  첩]               0 │ [직  위] ----------      │'
- Line 9: Python='(missing)', Rust='│ [命  中]               0 │ [回  避]               0 │'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `공격` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: 쳐 [대상]'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `습득` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 무슨 말인지 모르겠어요. *^_^*'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `시전` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: 시전 [스킬명] ([대상])'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `도망` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 무림인은 아무때나 도망가는것이 아니라네'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `장비` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=6
- Line 1: Python='', Rust='=== 장비 ==='
- Line 2: Python='(missing)', Rust='(장비한 아이템이 없습니다)'
- Line 3: Python='(missing)', Rust=''
- Line 4: Python='(missing)', Rust='=== 보너스 ==='
- Line 5: Python='(missing)', Rust=''
- Line 6: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  | X |

---

#### Command: `품목표` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 품목을 보여줄 상인이 없어요. ^^'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `버려 검` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 그런 아이템이 소지품에 없어요.'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  | X |

---

#### Command: `줘` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: [대상] [물품] [개수] 주다'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `구입` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: [물품이름] [수량] 구입'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `판매` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: [아이템 이름] [수량] 판매'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  | X |

---

#### Command: `먹어` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: [아이템 이름] 먹어'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  | X |

---

#### Command: `입어` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: [아이템 이름] 입어'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  | X |

---

#### Command: `벗어` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 사용법: [아이템 이름] 벗어  또는  [모두/전부] 벗어'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  | X |

---

#### Command: `말 안녕` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='당신이 말합니다 : '안녕''
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `대화` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 무슨 말인지 모르겠어요. *^_^*'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `물어` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='당신이 야스리를 꺼내서 이빨을 날카롭게 다듬습니다. '서걱 서걱~~''
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `정보` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=20
- Line 1: Python='', Rust='┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑'
- Line 2: Python='(missing)', Rust='▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │'
- Line 3: Python='(missing)', Rust='┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥'
- Line 4: Python='(missing)', Rust='│ [레  벨]       [     1] │ [나  이]              18 │'
- Line 5: Python='(missing)', Rust='│ [체  력] 450/0           │ [성  격] ----------      │'
- Line 6: Python='(missing)', Rust='│ [  힘  ]     0 +     15 │ [성  별]                 │'
- Line 7: Python='(missing)', Rust='│ [맷  집]      0 +      0 │ [소  속] ----------      │'
- Line 8: Python='(missing)', Rust='│ [민  첩]               0 │ [직  위] ----------      │'
- Line 9: Python='(missing)', Rust='│ [命  中]               0 │ [回  避]               0 │'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

#### Command: `퀘스트` - DIFFER

**Differences:**

- Line count differs: Python=1, Rust=2
- Line 1: Python='', Rust='☞ 무슨 말인지 모르겠어요. *^_^*'
- Line 2: Python='(missing)', Rust='[ 450/0, 0/0 ]'

**Keywords Present:**

| Keyword | Python | Rust |
|---------|--------|------|
| 체력 |  |  |
| 내력 |  |  |
| 은전 |  |  |
| 경험치 |  |  |
| 레벨 |  |  |
| HP |  |  |
| MP |  |  |
| Gold |  |  |
| EXP |  |  |
| Level |  |  |
| 낙양성 |  |  |
| 방파 |  |  |
| 무공 |  |  |
| 아이템 |  |  |

---

## Python Server Test Results

### `능력치` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.884936

---

### `점수` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.884971

---

### `무공` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.884980

---

### `소지품` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.884987

---

### `누구` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.884994

---

### `봐` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.885001

---

### `지도` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.885008

---

### `어디` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.885016

---

### `도움말` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.885023

---

### `저장` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:33.885030

---

### `move_동` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:43.933647

---

### `move_서` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:43.933663

---

### `move_남` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:43.933671

---

### `move_북` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:43.933679

---

### `move_위` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:43.933687

---

### `move_아래` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:43.933694

---

### `move_봐` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:43.933701

---

### `상태` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:50.959116

---

### `공격` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:50.959128

---

### `습득` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:50.959136

---

### `시전` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:50.959143

---

### `도망` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:50.959151

---

### `장비` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971373

---

### `품목표` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971385

---

### `버려 검` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971392

---

### `줘` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971400

---

### `구입` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971407

---

### `판매` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971414

---

### `먹어` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971421

---

### `입어` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971427

---

### `벗어` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:26:55.971435

---

### `말 안녕` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:27:05.001345

---

### `대화` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:27:05.001359

---

### `물어` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:27:05.001367

---

### `정보` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:27:05.001374

---

### `퀘스트` - FAIL

- **Output Length:** 0 bytes
- **Execution Time:** 0.00s
- **Timestamp:** 2026-02-08T04:27:05.001382

---

## Rust Server Test Results

### `능력치` - PASS

- **Output Length:** 910 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:34.890015

**Output Preview:**
```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑
 ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │
┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥
│ [레  벨]       [     1] │ [나  이]              18 │
│ [체  력] 450/0           │ [성  격] ----------      │
│ [  힘  ]     0 +     15 │ [성  별]                 │
│ [맷  집]      0 +      0 │ [소  속] ----------      │
│ [민  첩]               0 │ [직  위] ----------      │
│ [命  中]               0 │ [回  避]               0 │
│ [必  殺]               0 │ [  運  ]               0 │
│ [내  공] 0/0             │ [배...
```

---

### `점수` - PASS

- **Output Length:** 910 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:35.894888

**Output Preview:**
```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑
 ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │
┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥
│ [레  벨]       [     1] │ [나  이]              18 │
│ [체  력] 450/0           │ [성  격] ----------      │
│ [  힘  ]     0 +     15 │ [성  별]                 │
│ [맷  집]      0 +      0 │ [소  속] ----------      │
│ [민  첩]               0 │ [직  위] ----------      │
│ [命  中]               0 │ [回  避]               0 │
│ [必  殺]               0 │ [  運  ]               0 │
│ [내  공] 0/0             │ [배...
```

---

### `무공` - PASS

- **Output Length:** 366 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:36.897145

**Output Preview:**
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
◁ 당신의 무공 ▷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
───────────────────────────────────────
☞ 깨우친 무공이 없습니다.
───────────────────────────────────────
▷ 비전
☞ 오의를 깨우친 무공이 없습니다.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[ 450/0, 0/0 ] 
```

---

### `소지품` - PASS

- **Output Length:** 255 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:37.900944

**Output Preview:**
```
━━━━━━━━━━━━━━━━━
  ◁     소     지     품     ▷  
─────────────────
☞ 아무것도 없습니다.
─────────────────
▶ 은전 :                10000 개 
─────────────────
[ 450/0, 0/0 ] 
```

---

### `누구` - PASS

- **Output Length:** 287 bytes
- **Execution Time:** 1.01s
- **Timestamp:** 2026-02-08T04:26:38.907000

**Output Preview:**
```
┌─────────────────────────────────────┐
│ ◁     무       림       크       래       프       트      １-１      ▷ │
└─────────────────────────────────────┘
  [무명객]테스터러스트    
 ──────────────────────────────────────
 ★ 총 1명의 무림인이 활동하고 있습니다.
[ 450/0, 0/0 ] 
```

---

### `봐` - PASS

- **Output Length:** 361 bytes
- **Execution Time:** 1.01s
- **Timestamp:** 2026-02-08T04:26:39.917881

**Output Preview:**
```

[[[[] 낙양성 []]]]

산들바람에 여유롭게 흔들리는 꽃들은 주위배경과 절묘하게 조
화되어 있고 아담하게 꾸며진 정원 사이로 머리가 희끗 희끗한
초로가 한가롭게 산책을 하고 있다. 무림에 입문한 사람을 위한
『안내문』이 붙어 있다.

      
◁○     〔서ː초보수련장〕쪽으로 이동할 수 있습니다.
      

단아한 기품을 풍기는 왕대협이 서있다.

[ 450/0, 0/0 ] 
```

---

### `지도` - PASS

- **Output Length:** 113 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:40.921436

**Output Preview:**
```

낙양성

      
◁○     〔서ː초보수련장〕쪽으로 이동할 수 있습니다.
      
[ 450/0, 0/0 ] 
```

---

### `어디` - PASS

- **Output Length:** 54 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:41.924445

**Output Preview:**
```
테스터러스트     ▷ 낙양성
[ 450/0, 0/0 ] 
```

---

### `도움말` - PASS

- **Output Length:** 149 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:42.926307

**Output Preview:**
```
=== 도움말 ===
이동: 북(ㅂ) 남(ㄴ) 동(ㄷ) 서(ㅅ) 위(ㅇ) 아래(ㅁ) 북서(nw) 북동(ne) 남서(sw) 남동(se)
보기: look, 봐, 보
종료: quit, 끝, 종료
[ 450/0, 0/0 ] 
```

---

### `저장` - PASS

- **Output Length:** 38 bytes
- **Execution Time:** 1.01s
- **Timestamp:** 2026-02-08T04:26:43.932391

**Output Preview:**
```
* 저장 되었습니다.
[ 450/0, 0/0 ] 
```

---

### `move_동` - PASS

- **Output Length:** 57 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:44.935521

**Output Preview:**
```
☞ 동쪽으로 갈 수 없습니다.
[ 450/0, 0/0 ] 
```

---

### `move_서` - PASS

- **Output Length:** 374 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:45.937609

**Output Preview:**
```

[[[[] 하남성 낙양 []]]]

따스한 햇살을 받으며 포근하게 자리잡고 있는 아담한 집이 한
채 있다. 주인의 성격을 대변하는듯 깔끔하게 정돈되어 있고,
마당에는 아름다운 꽃들이 흐드러지게 피어있다.

  △  
  ○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.
  ▽  

조그만 생쥐가 바쁘게 뛰어갑니다.
조그만 생쥐가 바쁘게 뛰어갑니다.
[ 450/0, 0/0 ] 
```

---

### `move_남` - PASS

- **Output Length:** 424 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:46.939784

**Output Preview:**
```

[[[[] 하남성 낙양 []]]]

길가에는 한적한 기운이 서려 있고 따스한 햇빛이 조용히 내리
고 있다. 성곽에는 한가로운 들꽃들이 듬성듬성 자라나 남북으
로 이어지고 있는 매우 운치 있는 곳이다.

  △  
  ○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.
  ▽  

조그만 강아지가 발발발 돌아다니고 있다.
조그만 강아지가 발발발 돌아다니고 있다.
건장하게 생긴 청년이 당신을 바라보고 있습니다.
 
[ 450/0, 0/0 ] 
```

---

### `move_북` - PASS

- **Output Length:** 374 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:47.942056

**Output Preview:**
```

[[[[] 하남성 낙양 []]]]

따스한 햇살을 받으며 포근하게 자리잡고 있는 아담한 집이 한
채 있다. 주인의 성격을 대변하는듯 깔끔하게 정돈되어 있고,
마당에는 아름다운 꽃들이 흐드러지게 피어있다.

  △  
  ○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.
  ▽  

조그만 생쥐가 바쁘게 뛰어갑니다.
조그만 생쥐가 바쁘게 뛰어갑니다.
[ 450/0, 0/0 ] 
```

---

### `move_위` - PASS

- **Output Length:** 57 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:48.943667

**Output Preview:**
```
☞ 위쪽으로 갈 수 없습니다.
[ 450/0, 0/0 ] 
```

---

### `move_아래` - PASS

- **Output Length:** 58 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:49.947623

**Output Preview:**
```
☞ 아래쪽으로 갈 수 없습니다.
[ 450/0, 0/0 ] 
```

---

### `move_봐` - PASS

- **Output Length:** 376 bytes
- **Execution Time:** 1.01s
- **Timestamp:** 2026-02-08T04:26:50.958549

**Output Preview:**
```

[[[[] 하남성 낙양 []]]]

따스한 햇살을 받으며 포근하게 자리잡고 있는 아담한 집이 한
채 있다. 주인의 성격을 대변하는듯 깔끔하게 정돈되어 있고,
마당에는 아름다운 꽃들이 흐드러지게 피어있다.

  △  
  ○▷   〔동ː남ː북〕쪽으로 이동할 수 있습니다.
  ▽  

조그만 생쥐가 바쁘게 뛰어갑니다.
조그만 생쥐가 바쁘게 뛰어갑니다.

[ 450/0, 0/0 ] 
```

---

### `상태` - PASS

- **Output Length:** 910 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:51.964080

**Output Preview:**
```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑
 ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │
┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥
│ [레  벨]       [     1] │ [나  이]              18 │
│ [체  력] 450/0           │ [성  격] ----------      │
│ [  힘  ]     0 +     15 │ [성  별]                 │
│ [맷  집]      0 +      0 │ [소  속] ----------      │
│ [민  첩]               0 │ [직  위] ----------      │
│ [命  中]               0 │ [回  避]               0 │
│ [必  殺]               0 │ [  運  ]               0 │
│ [내  공] 0/0             │ [배...
```

---

### `공격` - PASS

- **Output Length:** 54 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:52.965443

**Output Preview:**
```
☞ 사용법: 쳐 [대상]
[ 450/0, 0/0 ] 
```

---

### `습득` - PASS

- **Output Length:** 62 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:53.967601

**Output Preview:**
```
☞ 무슨 말인지 모르겠어요. *^_^*
[ 450/0, 0/0 ] 
```

---

### `시전` - PASS

- **Output Length:** 63 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:54.969281

**Output Preview:**
```
☞ 사용법: 시전 [스킬명] ([대상])
[ 450/0, 0/0 ] 
```

---

### `도망` - PASS

- **Output Length:** 64 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:55.970971

**Output Preview:**
```
☞ 무림인은 아무때나 도망가는것이 아니라네
[ 450/0, 0/0 ] 
```

---

### `장비` - PASS

- **Output Length:** 99 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:56.973597

**Output Preview:**
```
=== 장비 ===
(장비한 아이템이 없습니다)

=== 보너스 ===

[ 450/0, 0/0 ] 
```

---

### `품목표` - PASS

- **Output Length:** 48 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:57.976473

**Output Preview:**
```
☞ 품목을 보여줄 상인이 없어요. ^^
[ 450/0, 0/0 ] 
```

---

### `버려 검` - PASS

- **Output Length:** 46 bytes
- **Execution Time:** 1.01s
- **Timestamp:** 2026-02-08T04:26:58.982291

**Output Preview:**
```
☞ 그런 아이템이 소지품에 없어요.
[ 450/0, 0/0 ] 
```

---

### `줘` - PASS

- **Output Length:** 51 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:26:59.985248

**Output Preview:**
```
☞ 사용법: [대상] [물품] [개수] 주다
[ 450/0, 0/0 ] 
```

---

### `구입` - PASS

- **Output Length:** 48 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:00.989654

**Output Preview:**
```
☞ 사용법: [물품이름] [수량] 구입
[ 450/0, 0/0 ] 
```

---

### `판매` - PASS

- **Output Length:** 50 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:01.992540

**Output Preview:**
```
☞ 사용법: [아이템 이름] [수량] 판매
[ 450/0, 0/0 ] 
```

---

### `먹어` - PASS

- **Output Length:** 45 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:02.995462

**Output Preview:**
```
☞ 사용법: [아이템 이름] 먹어
[ 450/0, 0/0 ] 
```

---

### `입어` - PASS

- **Output Length:** 45 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:03.998028

**Output Preview:**
```
☞ 사용법: [아이템 이름] 입어
[ 450/0, 0/0 ] 
```

---

### `벗어` - PASS

- **Output Length:** 61 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:05.000948

**Output Preview:**
```
☞ 사용법: [아이템 이름] 벗어  또는  [모두/전부] 벗어
[ 450/0, 0/0 ] 
```

---

### `말 안녕` - PASS

- **Output Length:** 52 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:06.003186

**Output Preview:**
```
당신이 말합니다 : '안녕'
[ 450/0, 0/0 ] 
```

---

### `대화` - PASS

- **Output Length:** 62 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:07.004797

**Output Preview:**
```
☞ 무슨 말인지 모르겠어요. *^_^*
[ 450/0, 0/0 ] 
```

---

### `물어` - PASS

- **Output Length:** 65 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:08.007221

**Output Preview:**
```
당신이 야스리를 꺼내서 이빨을 날카롭게 다듬습니다. '서걱 서걱~~'
[ 450/0, 0/0 ] 
```

---

### `정보` - PASS

- **Output Length:** 910 bytes
- **Execution Time:** 1.01s
- **Timestamp:** 2026-02-08T04:27:09.012258

**Output Preview:**
```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑
 ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ │
┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥
│ [레  벨]       [     1] │ [나  이]              18 │
│ [체  력] 450/0           │ [성  격] ----------      │
│ [  힘  ]     0 +     15 │ [성  별]                 │
│ [맷  집]      0 +      0 │ [소  속] ----------      │
│ [민  첩]               0 │ [직  위] ----------      │
│ [命  中]               0 │ [回  避]               0 │
│ [必  殺]               0 │ [  運  ]               0 │
│ [내  공] 0/0             │ [배...
```

---

### `퀘스트` - PASS

- **Output Length:** 62 bytes
- **Execution Time:** 1.00s
- **Timestamp:** 2026-02-08T04:27:10.013901

**Output Preview:**
```
☞ 무슨 말인지 모르겠어요. *^_^*
[ 450/0, 0/0 ] 
```

---

