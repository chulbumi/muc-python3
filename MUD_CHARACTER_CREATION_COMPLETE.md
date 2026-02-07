# Complete MUD Character Creation Flow Transcript

**Server**: localhost:9900
**Date**: 2026-01-24
**Encoding**: UTF-8

---

## FULL TRANSCRIPT - CHARACTER CREATION FLOW

### STEP 1: Connect to Server

```
$ nc localhost 9900
# or
$ telnet localhost 9900
```

### STEP 2: Initial Welcome Screen

**Server Output:**
```
[0m[37m[40m[H[2J
             [47;34;1m ▶   무림크래프트뉴얼   ◀ [0;37;40m
 ━─[30;1m───────────────────────────────[0;37;40m━━
   봄날 비바람에 날개가 찢겨 죽는 나비처럼, 한 계절의 영화에 몸을 불사
  르는 꽃처럼....... 혹은 한줄기 유성처럼.......
  그 모든 것이 정녕 화려한 기(氣)와 기(技)의 촌각속에서 한 조각 편운
  같은 명예를 쫓아 부나비처럼 명멸하는 하무군상의 광대놀음에 불과하다
  해도..
  오늘도 고독한 무인들은 백포로 검날을 닦고 한 잔의 싸구려 화주로 타는
  가슴을 달랜다.
  인간이 아니라 오직 한 자(者) 신(神)이라 불리우기 위해
  절대종사(絶代宗社) 그 위대한 이름이 이글거리는 고봉의 정상을 오른다
 ━─[30;1m───────────────────────────────[0;37;40m━━


[0;37m[40m무림에서 불리우는 존함을 알려주세요. (처음 오시는 분은 [1m무명객[0;40m이라고 하세요)
무림존함ː
```

**Notes:**
- Screen clears with `[H[2J` ANSI codes
- Logo displays with blue background `[47;34;1m`
- Prompt uses Korean colon `ː` character
- Instructions suggest entering "무명객" (guest) for new players

### STEP 3: Enter Name (Quick Path)

**User Input:** `나만바라바` (Quick creation path)

**Server Response:**
```
노인이 말합니다. "그 아이의 이름은 무엇이던가?"
무림존함ː
```

**Alternative:** If you enter "무명객", you get a 10-15 minute cinematic story before creation.

### STEP 4: Character Name

**User Input:** `테스트` (Test character name)

**Server Response:**
```
왕대협이 말합니다. "테스트(이라/라)고 합니다."
노인이 말합니다. "음! 좋은 이름이군 그렇다면 암호는??"
존함암호ː
```

**Validation Rules:**
- Name must be Korean (Hangul) characters only
- Cannot duplicate existing character
- Cannot be already logged in

### STEP 5: Password Entry

**User Input:** `test1234`

**Server Response:**
```
암호확인ː
```

### STEP 6: Password Confirmation

**User Input:** `test1234`

**If passwords match:**
```
노인이 말합니다. "그런데 그아이는 남자인가? 여자인가?"
성별(남/여)ː
```

**If passwords don't match:**
```
☞ 존함의 암호가 일치하지 않는군요.
존함암호ː
```

**Validation Rules:**
- Password must be 3+ characters
- Must match confirmation exactly

### STEP 7: Gender Selection

**User Input:** `남` (Male) or `여` (Female)

**Server Response (if valid):**
```
노인이 말합니다 "자고로 상처입은 짐승이 찾아와도 보살펴
                 주는법 집사는 그 아이를 거두어 가르치시오"
.....
.......
..........
.............
그로부터 13년후,
```

**Server Response (if invalid):**
```
☞ [남], [여]로 말해주세요.
성별(남/여)ː
```

### STEP 8: Tutorial - Name Introduction

**Server Output:**
```
왕대협이 말합니다. "테스트야 이곳에 들어온지도 어언 13년이라는 세월이
                    흘렀구나 .. 지금의 네 나이는 18세. 이정도라면 웅지를
                    품을만한 나이가 되었는데 지금부터 세상에 나가 너의 꿈을
                    펼쳐보도록 하여라. 지금 너의 능력은 아주 미약하니 낙양성
                    주위를 돌아다니며 견문을 넓히도록 하고, 추후 내가 다시부
                    르때 돌아오도록 하여라..

【엔터키를 누르세요】
```

**User Action:** Press [Enter]

### STEP 9: Tutorial - Return Command

**Server Output:**
```
왕대협이 말합니다. "이곳으로 돌아오는 방법은 『귀환』이라 하면 되니. 이것은
                    반드시 명심하도록.. 그러면 지금 출발하도록 하여라...."

왕대협이 말합니다. "그리고 이것은 매의깃털이라는 것인데 머리에 꼽고 다니면
                    어지간한 공격은 막을수 있으니 소중하게 사용하도록 하여라"

왕대협이 당신에게 매의깃털을 선물 합니다.

【엔터키를 누르세요】
```

**User Action:** Press [Enter]

### STEP 10: Tutorial - Inventory Command

**Server Output:**
```
왕대협이 말합니다. "우선 험난한 강호에서 살아남기 위해서는 자신의 몸은
                    스스로 보호알줄 알아야 된다. 지금부터 내가 강호에서
                    살아남는 기초적인 방법을 알려주마..."

왕대협이 말합니다. "항상 무인은 자기가 가지고 있는 소지품이 무엇인지 알고
                    있어야 한다. 그것을 알아볼수 있는 명령어는 『소지품』
                    이라고 하니 이것을 암기하도록 하여라"

『소지품』을 입력 하세요
[450/450, 18/18]
>
```

**User Input:** `소지품`

**Server Response:**
```
━━━━━━━━━━━━
[1;44;37m  ◁  소   지   품  ▷  [0;37;40m
────────────
[0;36;40m매의깃털[0;37;40m
[0;37;40m────────────
[0;47;30m▶ 은전 :    10000 개   [0;37;40m
────────────[0;37;40m

왕대협이 말합니다. "그런 방법을 사용하면 항상 내가 소지하고 다니는 물건을
                    확인할수 있단다.."

왕대협이 말합니다. "그러나 그것을 가지고만 다닌다면 전혀 쓸모 없는 물건이다
                    매의깃털은 너의 방어력을 높여주니 『매의깃털 착용』을
                    하여 머리에 쓰고 다니면 너의 방어력이 향상될 것이다."

【엔터키를 누르세요】
```

**User Action:** Press [Enter]

### STEP 11: Tutorial - Talk Command

**Server Output:**
```
왕대협이 말합니다. "그리고 무림에서 모르는 사람을 만나게 되면 말을 해서
                    인사를 하는것도 살아가는 처세술이라 할수 있지
                    모르는 타인이 너에게 인정을 베풀거나, 타인에게 도움을
                    청할때는 『말』을 해서 너의 현재 상태를 알리도록 하여라"

『안녕하세요 말』이라고 해보거라.

[450/450, 18/18]
>
```

**User Input:** `안녕하세요 말`

**Server Response:**
```
당신이 말합니다. "안녕하세요"

왕대협이 말합니다. "테스트야 잘하는구나 ^^"

【엔터키를 누르세요】
```

**User Action:** Press [Enter]

### STEP 12: Tutorial - Shout Command

**Server Output:**
```
왕대협이 말합니다. "이번에는 먼곳에 떨어져 있는 사람에게 말하는 법을 알려주마
                    같은 자리에 있지 않은 사람에게 말할때는 『외침』 이라는
                    명령어를 사용한단다."

『안녕하세요 외침』이라고 해보거라.

[450/450, 18/18]
>
```

**User Input:** `안녕하세요 외침`

**Server Response:**
```
[테스트]〔[40m[32m외 침[0;37;40m〕 : 안녕하세요

용파리〔[40m[32m외 침[0;37;40m〕 : 안녕하세요?? 테스트님.. 하하하~~

【엔터키를 누르세요】
```

**User Action:** Press [Enter]

### STEP 13: Tutorial - Help and Final Instructions

**Server Output:**
```
왕대협이 말합니다. "지금은 시간이 없으니 우선 이정도만 배우도록하고
                    강호를 돌아다니며 모르는점이 있을때는 『도움』을
                    입력해서 새로운 명령어를 숙지하도록 하여라 그것이
                    강호를 살아나가는 처세술이란다."

왕대협이 말합니다. "또한 시간이 난다면 낙양표국을 찾아가서 표국보험에
                    가입하도록 하여라 보험에 가입하는 방법은 표국을
                    찾아가서 『안내』라고 입력한다면 알수 있단다."

【엔터키를 누르세요】
```

**User Action:** Press [Enter]

### STEP 14: Tutorial - Departure Instructions

**Server Output:**
```
왕대협이 말합니다. "어느정도 방법을 배웠으니 내 말이 끝나는 즉시
                   『매의깃털 착용』을 한다음 초보자수련장으로
                   찾아가도록 하거라. '초보자수련장' 이라고 치거나.  아니면
                   '왕대협 대화'라고 하면 초보자수련장으로 이동하게 되느니라.

왕대협이 말합니다. "어서 출발하도록 하여라 그리고 내가 다시 부르면
                    이곳에 다시 돌아오면 되느리라... 이곳에 돌아올때는
                    『귀환』이라고 하면 다시 돌아올수 있단다..

【엔터키를 누르세요】
```

**User Action:** Press [Enter]

### STEP 15: Enter Game

After pressing Enter, the notice screen is displayed and the character enters the game world.

---

## QUICK REFERENCE - VALID INPUTS

| Prompt | Input | Description |
|--------|-------|-------------|
| 무림존함ː | `테스트` | Character name (Korean) |
| 무림존함ː | `무명객` | Guest/Full story path |
| 무림존함ː | `나만바라바` | Quick path (skip story) |
| 존함암호ː | `test1234` | Password (3+ chars) |
| 암호확인ː | `test1234` | Same password |
| 성별(남/여)ː | `남` or `여` | Gender selection |
| `>` | `소지품` | Show inventory |
| `>` | `매의깃털 착용` | Equip feather |
| `>` | `안녕하세요 말` | Say hello |
| `>` | `안녕하세요 외침` | Shout hello |
| `>` | `도움` | Show help |
| `>` | `귀환` | Return to home |

---

## CHARACTER FILE LOCATION

Characters are saved in: `/home/ubuntu/muc-python3/data/character/[NAME]`

Example: `/home/ubuntu/muc-python3/data/character/테스트`

---

## SPECIAL BEHAVIORS NOTED

1. **Timeout**: Connection closes if no input for extended period
2. **Encoding**: All text is UTF-8
3. **Line Endings**: Server expects `\r\n` (CRLF)
4. **Prompt Characters**: Uses Korean colon `ː` not regular `:`
5. **Combat**: Uses `[HP/HP MP/MP]` format for status
6. **Colors**: Heavy use of ANSI escape codes for formatting
