// 도우미 스크립트: 초기도우미 (풀 스토리) - 단계 기반(step-based)
// 각 step 함수는 ob를 인자로 받고, wait_enter/wait_input/wait_key_input으로 다음 단계 지정

// ========== Act 1: The Escape (steps 1-5) ==========

fn step1_welcome(ob) {
    set_tick(3);

    send_line(ob, "\x1b[1;30;40m옷깃을 가볍게 적시는 가랑비가 촉촉히 내리는 새벽 .......\x1b[0;37;40m");

    send_line(ob, "어둠을 깨트리는 처절한 비명 소리를 뒤로 하고 생사를 건 탈출을 하는 이들이");

    send_line(ob, "있었다.");

    send_line(ob, "두 명의 사내와 갓 세살을 넘었을 만한 아이....");

    send_line(ob, "한 남자는 중년의 건장한 모습이나 온통 피로 물들어 있었고 다른 한 남자는");

    send_line(ob, "심한 부상을 입은 듯 복부를 왼손으로 감싸고 오른 손엔 검을 쥐고 있으나");

    send_line(ob, "어깨에서 부터 흘러내린 피가 검신을 타고 끊임 없이 흘러 내리고 있었다.");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 말합니다. \"소룡!, 나는 더 이상 갈수가 없을 것 같네...\"");

    send_line(ob, "\"어서 소주인을 모시고  이 곳을 빠져나가게 .....\"");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 거친 숨을 몰아 쉬며 검 끝을 땅에 대고 간신히 서서 \x1b[1m중년의남자\x1b[0;37;40m에게");

    send_line(ob, "이야기 한다.");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 말합니다. \"일주!!.. 힘내게, 살아서 오늘의 원수를 갚아야 하지 않나..\"");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 말합니다. \"아닐세 나는 틀렸어... 이 곳에 남아 한 다경의 시간 이라도\"");

    send_line(ob, "벌어볼테니 어서 소주인을 모시고 낙양의 백부님께 가게... \"");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 몹시 괴로운듯 기침을 합니다. \"콜록~~~. 콜록~~~ \"");

    send_line(ob, "");

    send_line(ob, "기침과 함께 \x1b[31m검붉은\x1b[37m 핏덩이가 뿜어져 나온다. 외상 뿐 아니라 깊은 내상도 입은");

    send_line(ob, "것 같다.");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step2_farewell");
}

fn step2_farewell(ob) {
    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "일주는 소룡의 품에 안겨서 두려운듯 부들부들 떨고 있는 아이에게 부복 하여");

    send_line(ob, "피눈물을 흘리며 처절하게 외치듯 말한다.");

    send_line(ob, "");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 말합니다. \"소군!  죄송 합니다. 끝까지 모시지 못하고 여기서 인사를\"");

    send_line(ob, "드려야 할 것 같습니다. 부디 옥체 보존 하시어 후일을 기약");

    send_line(ob, "하시고 오늘의 한을 천배, 만배의 피로 씻어야 합니다.");

    send_line(ob, "제일세가 8대 호법 '일주'는 이 곳에서 소군께 하직 인사를");

    send_line(ob, "드리지만 원혼 이라도 도련님 곁에 남아 지켜 보겠으니 부디.");

    send_line(ob, "크억~~~ \"");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step3_monster_appear");
}

fn step3_monster_appear(ob) {
    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 던진 연리표가 북쪽에서 날아와 \x1b[1m일주\x1b[0;37;40m의 등에 박힙니다. \"퍼억~~\"");

    send_line(ob, "북쪽에서 검은 색과 흰 색의 옷을 입은 \x1b[33m괴한\x1b[0;37;40m이 경공을 펼치며 쾌속으로 다가옵니다.");

    send_line(ob, "");

    send_line(ob, "『흑백쌍괴 봐』를 입력 하세요");

    send_line(ob, ">");

    wait_key_input("step4_pre_combat", "흑백쌍괴 봐");
}

fn step4_pre_combat(ob) {
    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    send_line(ob, "\x1b[0;44m\x1b[1m◆ 이름 ▷ 흑백쌍괴                                         \x1b[0;37;40m");

    send_line(ob, "──────────────────────────────");

    send_line(ob, "쭉 찢어진 뱀눈에 검은 무복을 걸쳐입는 사내다. 음험하게 풍기");

    send_line(ob, "는 기도에서 사악함이 배어 있어 스산함과 섬뜩함이 동시에");

    send_line(ob, "묻어나온다.");

    send_line(ob, "──────────────────────────────");

    send_line(ob, "[무    기] \x1b[36m참수도\x1b[37m");

    send_line(ob, "──────────────────────────────");

    send_line(ob, "★ 흑백쌍괴는 아주 활력이 넘칩니다");

    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step5_combat_start");
}

fn step5_combat_start(ob) {
    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 말합니다. \"쥐새끼 같은 놈들 감히 우리 흑백쌍괴의 손아귀를");

    send_line(ob, "벗어나리라 생각했나?... \"");

    send_line(ob, "");

    send_line(ob, "순간 연리표에 상처를 입었던 \x1b[1m일주\x1b[0;37;40m가 몸을 날려 \x1b[33m흑백쌍괴\x1b[0;37;40m에게 달려간다.");

    send_line(ob, "");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 말합니다. \"소룡, 빨리 이곳을 빠져나가게...   어서... \"");

    send_line(ob, "");

    send_line(ob, "일주는 자신의 피로 젖은 검을 움켜쥐고 마지막 남은 진기를 모아 소군의");

    send_line(ob, "탈출을 돕기위해 불꽃을 찾아 날아드는 부나비 처럼 흑백쌍괴에게 몸을 던진다.");

    send_line(ob, "소룡은 그런 일주의 모습을 눈동자에 짧은 시간동안 담은뒤 소군을 안고");

    send_line(ob, "돌아선다. 일주를 향해 소리치는 아이.....");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step6_escape_continue");
}

// ========== Act 2: The Journey (steps 6-10) ==========

fn step6_escape_continue(ob) {
    set_tick(7);

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 비룡신검을 왼손으로 바꾸어 쥐고 정신을 가다듬습니다");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 고함을 지르며 \x1b[33m흑백쌍괴\x1b[0;37;40m에게 무모할 정도로 돌진합니다");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 \x1b[33m흑백쌍괴\x1b[0;37;40m의 급소를 노리며 \x1b[1m비룡신검\x1b[0;37;40m을 연속으로 휘두릅니다");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 \x1b[33m흑백쌍괴\x1b[0;37;40m의 급소를 노리며 \x1b[1m비룡신검\x1b[0;37;40m을 연속으로 휘두릅니다");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 \x1b[33m흑백쌍괴\x1b[0;37;40m의 급소를 노리며 \x1b[1m비룡신검\x1b[0;37;40m을 연속으로 휘두릅니다");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 우장을 곧게 펴고 좌장으로 사방을 격합니다 \"\x1b[1;32m녹수마장~\x1b[0;37;40m\"");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 \x1b[1m비룡신검\x1b[0;37;40m을 날카롭게 휘두르며 \x1b[33m흑백쌍괴\x1b[0;37;40m를 난자 합니다.");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 수장에 기운을 끌어모으자 손끝에서 \x1b[1;36m푸른 기운\x1b[0;37;40m이 뻗어 나옵니다");

    send_line(ob, "");

    send_line(ob, "[1334/2091 220/220]");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 \x1b[1m비룡신검\x1b[0;37;40m을 날카롭게 휘두르며 흑백쌍괴를 난자 합니다.");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 뿜어낸 \x1b[34m푸른 섬광\x1b[37m이 일주를 \x1b[31m섬뜩\x1b[37m하게 휘감고 지나갑니다");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 참수도를 안듯이 품으며 일주를 재차 공격합니다");

    send_line(ob, "");

    send_line(ob, "\x1b[1m일주\x1b[0;37;40m가 허망한듯한 표정으로 쓰러지며 \x1b[33m흑백쌍괴\x1b[0;37;40m를 바라봅니다 \"쿵~\"");

    send_line(ob, "");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 약간의 경험치를 얻습니다.");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가  일주에게 약간의 은전을 획득합니다.");

    set_tick(3);

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step7_pursuit");
}

fn step7_pursuit(ob) {
    send_line(ob, "\x1b[0;37;40m\x1b[H\x1b[2J");

    send_line(ob, "아이가 애절하게 외칩니다. \"아찌~~~\"");

    send_line(ob, "아찌를 부르는 소군을 안고 당신은 달리기 시작한다.");

    send_line(ob, "빗 속에 울려퍼지는 병장기 소리...");

    send_line(ob, "얼마를 달렸을까?  멀리서 들려오는 짧은 외마디 단발마...");

    send_line(ob, "당신의 눈가에 뜨거운 눈물이 고여 시야를 어지럽히고 가늘게 뿌리던");

    send_line(ob, "빗줄기가 굵어지기 시작한다.");

    send_line(ob, "");

    send_line(ob, "목숨을 건 탈출은 이렇게 한 생명의 희생으로 질긴 시간을 이어간다.");

    send_line(ob, "계절은 가을에서 이어져 늦겨울의 매서운 북풍을 등에 지고 낙양의 서쪽 숲에");

    send_line(ob, "선 두사람, 긴 도피의 시간 탓인지 누적된 피로와 추위에 몹시 지쳐있다.");

    send_line(ob, "");

    send_line(ob, "[3324/3324 320/320]");

    send_line(ob, "당신이 말합니다. \"헉.......헉...... 이제 얼마 남지 않았습니다.");

    send_line(ob, "조금만 참으세요.\"");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step8_pursuit_arrive");
}

fn step8_pursuit_arrive(ob) {
    send_line(ob, "\x1b[0;37;40m\x1b[H\x1b[2J");

    send_line(ob, "아무런 투정도 없이 묵묵히 따르는 아이,");

    send_line(ob, "수 많은 역경을 겪으며 배운 인내, 불과 몇달 사이 세상이 아이를 무척이나");

    send_line(ob, "강하게 변화 시킨 듯 싶다.");

    send_line(ob, "그러나 이들을 집요하게 쫓는 추적의 끈은 끊어지지 않고 이곳 까지");

    send_line(ob, "이어지고 있었다.");

    send_line(ob, "");

    send_line(ob, "'사형!..  여기 놈들의 발자국을 찾았습니다.'");

    send_line(ob, "'지독한 놈들 우리의 안목을 속이고 여기까지 도망치다니 역시 호랑이의");

    send_line(ob, "자식은 고양이가 될수 없는가보군. 서두르자.. 낙양에 들어가기 전에");

    send_line(ob, "놈들의 질긴 명줄을 끊어 놓아야 한다.'");

    send_line(ob, "");

    send_line(ob, "쫓기는 자와 쫓는 자, 실오라기 같은 삶의 연줄은 끊어질듯 이어지지만");

    send_line(ob, "그 끝도 이제 얼마남지 않은 듯 싶다.");

    send_line(ob, "");

    send_line(ob, "눈밭 위에서의 혈투");

    send_line(ob, "죽립을 쓰고 한손엔 검을, 다른 한손엔 아이를 안고 처절한 싸움이 계속된다.");

    send_line(ob, "");

    send_line(ob, "'어리석은 놈. 여기를 빠져나간다 해도 너희가 찾아갈 곳은 이미");

    send_line(ob, "잿더미가 되어 있을걸?.");

    send_line(ob, "쓸데없는 저항말고 순순히 목을 내놔라!...'");

    send_line(ob, "");

    send_line(ob, "순간 \x1b[1m당신\x1b[0;37;40m의 안색이 바뀌며........");

    send_line(ob, "");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 말합니다. \"이 짐승만도 못한 놈들 자신의 죄를 감추기 위해 제일세가를");

    send_line(ob, "몰살 시킨 것도 모자라 낙양의 백부님 까지 시해 하다니");

    send_line(ob, "하늘이 용서치 않을 것이다...\"");

    send_line(ob, "");

    send_line(ob, "『봐』를 입력 하세요");

    send_line(ob, ">");

    wait_key_input("step9_combat2_start", "봐");
}

fn step9_combat2_start(ob) {
    send_line(ob, "\x1b[1m\x1b[30m[\x1b[0;37;40m[[\x1b[1m[]\x1b[0;37;40m 하남성 북망산관도 \x1b[1m[]\x1b[0;37;40m]]\x1b[30m\x1b[1m]\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "북망산으로 가는 관도가 북쪽으로 구불구불하게 이어지고 있고");

    send_line(ob, "동쪽으로 돌아서 들어가는 샛길이 보인다. 어디선가 불어오는");

    send_line(ob, "스산한 바람이 등골을 섬뜩하게 스치고 지나간다");

    send_line(ob, "\x1b[32m △\x1b[0;37;40m");

    send_line(ob, "○\x1b[32m▷\x1b[0;37;40m 〔동ː남ː북〕쪽으로 이동할 수 있습니다.");

    send_line(ob, "\x1b[32m ▽\x1b[0;37;40m");

    send_line(ob, "음흉한 살기를 풍기는 \x1b[33m흑백쌍괴\x1b[0;37;40m가 \x1b[1m당신\x1b[0;37;40m을 노려보고 있습니다.");

    send_line(ob, "");

    send_line(ob, "『흑백쌍괴 쳐』를 입력 하세요");

    send_line(ob, ">");

    wait_key_input("step10_combat_skill", "흑백쌍괴 쳐");
}

fn step10_combat_skill(ob) {
    set_tick(7);

    send_line(ob, "[2514/3324 320/320]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 살수도를 날카롭게 빼어들고 공격 합니다.");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 살기를 드러내고 빠르게 공격합니다.");

    send_line(ob, "");

    send_line(ob, "[2514/3324 320/320]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 흑백쌍괴를 살수도로 난자 합니다 \x1b[1;36m50\x1b[0;37;40m");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 당신을 참수도로 푹~ 찔렀습니다 \x1b[1;31m810\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "『혈천섬광 시전』를 입력 하세요");

    send_line(ob, ">");

    wait_key_input("step11_combat_victory", "혈천섬광 시전");
}

fn step11_combat_victory(ob) {
    send_line(ob, "[2494/3324 120/320]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 가슴 서늘한 괴성을 토하며 시뻘건 \x1b[1m\x1b[31m혈무\x1b[0;37;40m를 뿜어냅니다 \"크아악~\"");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 \x1b[1m당신\x1b[0;37;40m을 참수도로 형편없이 찔렀습니다 \x1b[1m\x1b[31m780\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "[1654/3324 120/320]");

    send_line(ob, "짙디 짙은 혈무가 \x1b[1m당신\x1b[0;37;40m의 주위를 \x1b[1m\x1b[32m靈巳\x1b[0;37;40m처럼 휘감아 돕니다.");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 \x1b[1m당신\x1b[0;37;40m을 참수도로 찔러 돌립니다 \x1b[1m\x1b[31m880\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "[854/3324 120/320]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m의 살수도에서 재차 핏물처럼 짙디 짙은 \x1b[1m\x1b[31m혈무\x1b[0;37;40m가 피어오릅니다.");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 \x1b[1m당신\x1b[0;37;40m을 참수도로 강하게 베었습니다 \x1b[1m\x1b[31m850\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "[4/3324 120/320]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 혈안을 번뜩이며 \x1b[33m흑백쌍괴\x1b[0;37;40m에게 두줄기 \x1b[1m\x1b[31m혈광\x1b[0;37;40m을 뿜어냅니다 \x1b[1m\x1b[36m2550\x1b[0;37;40m");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m의 공격을 \x1b[1m당신\x1b[0;37;40m이 아슬아슬 하게 피했습니다.");

    send_line(ob, "");

    send_line(ob, "[54/3324 120/320]");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 핏빛 기류를 혈무속에서 뻗쳐내어 그대로 \x1b[33m흑백쌍괴\x1b[0;37;40m를 격타합니다 \x1b[1m\x1b[36m3450\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "\x1b[33m흑백쌍괴\x1b[0;37;40m가 한사발 \x1b[1m\x1b[31m붉은피\x1b[0;37;40m를 쏟아내며 쓰러집니다.");

    send_line(ob, "");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 9896의 경험치를 얻습니다.");

    send_line(ob, "\x1b[1m당신\x1b[0;37;40m이 \x1b[33m흑백쌍괴\x1b[0;37;40m에게 은전 141개를 획득합니다.");

    send_line(ob, "");

    set_tick(3);

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step12_arrival");
}

// ========== Act 3: Arrival (steps 12-13) ==========

fn step12_arrival(ob) {
    send_line(ob, "\x1b[0;37;40m\x1b[H\x1b[2J");

    send_line(ob, "얼마의 시간이 흘렀을까?...............");

    send_line(ob, "피.................");

    send_line(ob, "이월의 흐른 달빛아래 섬뜩하리만큼 붉은 한줌의 핏물.");

    send_line(ob, "핏물은 흰눈이 쌓인 낙양성 관도로 촘촘이 새겨지고 있었다.");

    send_line(ob, "희디흰 눈밭에 핏물은 닿기 무섭게 가래침처럼 엉겨 붙고....");

    send_line(ob, "");

    send_line(ob, "[헉............ 헉............]");

    send_line(ob, "");

    send_line(ob, "두사람,");

    send_line(ob, "검은 죽립을 쓴 탓으로 그 용모는 알아 볼 길이 없으나,");

    send_line(ob, "일신에 검은 무복를 걸친 이,");

    send_line(ob, "그리고 그의 손에 매달려 가쁜 숨을 몰아쉬며 바쁜 걸음을 재촉하는");

    send_line(ob, "어린 아이가 그들의 상황을 말해주고 있다.");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step13_death_scene");
}

fn step13_death_scene(ob) {
    send_line(ob, "입과...");

    send_line(ob, "가슴패기에 아예 구멍이 뚫리다시피 한 퀭한 상혈로부터 피는 그칠줄");

    send_line(ob, "모르고 끊임 없이 흩뿌리고 있었다.");

    send_line(ob, "");

    send_line(ob, "그런데도 그들의 걸음은 멈춰지지 않았다.");

    send_line(ob, "");

    send_line(ob, "아니 그들은 걷는 것이 아니라 아예 기다시피 하고 있었다.");

    send_line(ob, "그 어떤 일이 있어...");

    send_line(ob, "죽음보다 더한 사명감으로 그의 걸음을 저렇듯 처절하게 인도한단 말인가?");

    send_line(ob, "이제 관도는 끝났다.");

    send_line(ob, "");

    send_line(ob, "대신, 화강암을 깍아 만든 돌사자상이 그의 앞을 가로막고 있다.");

    send_line(ob, "돌사자상의 아득한 끝으로는 나즈마한 계단과 하나의 웅장한 처마가 보이고");

    send_line(ob, "있었다. 죽립인의 떨리는 손이 계단의 턱을 움켜갔다.");

    send_line(ob, "");

    send_line(ob, "돌연, 그의 입 밖으로 핏물이 왈칵 터져 나왔다.");

    send_line(ob, "");

    send_line(ob, "좌악!!  모래와, 그리고 흰 눈 쌓인 돌사자상위로 확 핏물이 번진다.");

    send_line(ob, "어스름달, 잔인한 이월의 어스름달,");

    send_line(ob, "죽립인의 가물거리는 시선이 장천의 얼음처럼 차가운 달을 마주 보는가");

    send_line(ob, "싶었다.");

    send_line(ob, "");

    send_line(ob, "[소군.......부..부......부디....]");

    send_line(ob, "");

    send_line(ob, "처절한 절규에 가까운 소리");

    send_line(ob, "그의 처절한 절규와 더불어 그의 얼굴은 자신이 뱉어 놓은 핏물 위로");

    send_line(ob, "퍽 하고 처박히고 말았다..");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step14_aftermath");
}

fn step14_aftermath(ob) {
    send_line(ob, "무력 790년 이월의 일이었다.");

    send_line(ob, "");

    send_line(ob, "[ 가슴 부위의 상흔은 백골음시장에 의한 부패현상이고, 그 밖에 우측다리에");

    send_line(ob, "서 상문투골정과 혈천미리표가 발견되었습니다....악랄한 수법입니다 ]");

    send_line(ob, "");

    send_line(ob, "딸랑 딸랑");

    send_line(ob, "한 줄기 미풍은 처마끝에 달린 풍경을 휩쓸어가고 있었고. 처마위로는 시린");

    send_line(ob, "달빛이 은가루 처럼 부서져 내리고 있었다.");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step15_interior");
}

// ========== Act 4: Interior (step 15) ==========

fn step15_interior(ob) {
    send_line(ob, "\x1b[0;37;40m\x1b[H\x1b[2J");

    send_line(ob, "\x1b[1m\x1b[30m[\x1b[0;37;40m[[\x1b[1m[]\x1b[0;37;40m 내    실 \x1b[1m[]\x1b[0;37;40m]]\x1b[30m\x1b[1m]\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "깊은 연륜이 서린 동공 하나가 흔들리는 황촛불을 무심히 바라보고 있었다.");

    send_line(ob, "한 쌍의 눈썹은 눈처럼 흰데다 귀 밑까지 흘러내린 용모는 고귀한 인상을");

    send_line(ob, "준다.");

    send_line(ob, "");

    send_line(ob, "그의 옆에는 백옥같은 도포를 입은 단아한 \x1b[33m중년인\x1b[0;37;40m이 조용히 시립해 서 있다.");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step16_name");
}

// ========== Act 5: Character Creation (steps 16-18) ==========

fn step16_name(ob) {
    send_line(ob, "\x1b[0;37;40m\x1b[H\x1b[2J");

    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"왕대협~  아이의 상태는 어떤가??\"");

    send_line(ob, "");

    send_line(ob, "왕대협이라 불리는 중년인은 기품이 역력한 노인을 향해 공손이 허리를 굽히");

    send_line(ob, "며 말했다.");

    send_line(ob, "");

    send_line(ob, "\x1b[1m왕대협\x1b[0;37;40m이 말합니다. \"다행히도 상세는 호전되었습니다.\"");

    send_line(ob, "");

    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"그 아이의 이름은 무엇이던가?\"");

    wait_input("step17_password", "get_name");
}

fn step17_password(ob) {
    let name = _doumi_resume_input;
    ob["이름"] = name;

    send_line(ob, "\x1b[1m왕대협\x1b[0;37;40m이 말합니다. \"" + name + "(이라/라)고 합니다.\"");

    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"음! 좋은 이름이군 그렇다면 암호는??\"");

    wait_input("step18_gender", "get_password");
}

fn step18_gender(ob) {
    let password = _doumi_resume_input;
    ob["암호"] = password;

    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다. \"그런데 그아이는 남자인가? 여자인가?\"");

    wait_input("step19_time_skip", "get_sex");
}

// ========== Act 6: Time Skip (steps 19-21) ==========

fn step19_time_skip(ob) {
    let sex = _doumi_resume_input;
    ob["성별"] = sex;

    send_line(ob, "\x1b[1m노인\x1b[0;37;40m이 말합니다 \"자고로 상처입은 짐승이 찾아와도 보살펴");

    send_line(ob, "주는법 집사는 그 아이를 거두어 가르치시오\"");

    send_line(ob, "....");

    send_line(ob, ".......");

    send_line(ob, "..........");

    send_line(ob, ".............");

    send_line(ob, "그로부터 13년후");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step20_age_speech");
}

fn step20_age_speech(ob) {
    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"[공](아/야) 이곳에 들어온지도 어언 13년이라는 세월이");

    send_line(ob, "흘렀구나 .. 지금의 네 나이는 18세. 이정도라면 웅지를");

    send_line(ob, "품을만한 나이가 되었는데 지금부터 세상에 나가 너의 꿈을");

    send_line(ob, "펼쳐보도록 하여라. 지금 너의 능력은 아주 미약하니 낙양성");

    send_line(ob, "주위를 돌아다니며 견문을 넓히도록 하고, 추후 내가 다시부");

    send_line(ob, "를때 돌아오도록 하여라..");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step21_return_command");
}

fn step21_return_command(ob) {
    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"이곳으로 돌아오는 방법은 『\x1b[1m귀환\x1b[0;37;40m』이라 하면 되니. 이것은");

    send_line(ob, "반드시 명심하도록.. 그러면 지금 출발하도록 하여라....\"");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"그리고 이것은 매의깃털이라는 것인데 머리에 꼽고 다니면");

    send_line(ob, "어지간한 공격은 막을수 있으니 소중하게 사용하도록 하여라\"");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 \x1b[1m당신\x1b[0;37;40m에게 \x1b[36m매의깃털\x1b[0;37;40m을 선물 합니다.");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step22_tutorial_start");
}

// ========== Act 7: Tutorial (steps 22-25) ==========

fn step22_tutorial_start(ob) {
    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"우선 험난한 강호에서 살아남기 위해서는 자신의 몸은");

    send_line(ob, "스스로 보호알줄 알아야 된다. 지금부터 내가 강호에서");

    send_line(ob, "살아남는 기초적인 방법을 알려줄터이니 잘 암기 하도록");

    send_line(ob, "하여라\"");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"항상 무인은 자기가 가지고 있는 소지품이 무엇인지 알고");

    send_line(ob, "있어야 한다. 그것을 알아볼수 있는 명령어는 『\x1b[1m소지품\x1b[0;37;40m』");

    send_line(ob, "이라고 하니 이것을 암기하도록 하여라\"");

    send_line(ob, "");

    send_line(ob, "『소지품』를 입력 하세요");

    send_line(ob, "[450/450, 18/18]");

    wait_key_input("step23_tutorial_inventory", "소지품");
}

fn step23_tutorial_inventory(ob) {
    send_line(ob, "━━━━━━━━━━━━");

    send_line(ob, "\x1b[1;44;37m  ◁  소   지   품  ▷  \x1b[0;37;40m");

    send_line(ob, "────────────");

    send_line(ob, "\x1b[0;36;40m매의깃털\x1b[0;37;40m");

    send_line(ob, "\x1b[0;37;40m────────────");

    send_line(ob, "\x1b[0;47;30m▶ 은전 :    10000 개   \x1b[0;37;40m");

    send_line(ob, "────────────\x1b[0;37;40m");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"그런 방법을 사용하면 항상 내가 소지하고 다니는 물건을");

    send_line(ob, "확인할수 있단다..\"");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"그러나 그것을 가지고만 다닌다면 전혀 쓸모 없는 물건이다");

    send_line(ob, "매의깃털은 너의 방어력을 높여주니 『\x1b[1m매의깃털 착용\x1b[0;37;40m』을");

    send_line(ob, "하여 머리에 쓰고 다니면 너의 방어력이 향상될 것이다.\"");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step24_tutorial_say");
}

fn step24_tutorial_say(ob) {
    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"그리고 무림에서 모르는 사람을 만나게 되면 말을 해서");

    send_line(ob, "인사를 하는것도 살아가는 처세술이라 할수 있지");

    send_line(ob, "모르는 타인이 너에게 인정을 베풀거나, 타인에게 도움을");

    send_line(ob, "청할때는 『\x1b[1m말\x1b[0;37;40m』을 해서 너의 현재 상태를 알리도록 하여라");

    send_line(ob, "");

    send_line(ob, "『안녕하세요 말』이라고 해보거라.");

    send_line(ob, "");

    send_line(ob, "[450/450, 18/18]");

    wait_key_input("step25_tutorial_shout", "안녕하세요 말");
}

fn step25_tutorial_shout(ob) {
    send_line(ob, "당신이 말합니다. \"안녕하세요\"");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"[공](아/야) 잘하는구나 ^^\"");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step26_tutorial_shout_cmd");
}

fn step26_tutorial_shout_cmd(ob) {
    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"이번에는 먼곳에 떨어져 있는 사람에게 말하는 법을 알려주마");

    send_line(ob, "같은 자리에 있지 않은 사람에게 말할때는 『\x1b[1m외침\x1b[0;37;40m』 이라는");

    send_line(ob, "명령어를 사용한단다.\"");

    send_line(ob, "『\x1b[1m안녕하세요 외침\x1b[0;37;40m』이라고 해보거라.");

    send_line(ob, "");

    send_line(ob, "[450/450, 18/18]");

    wait_key_input("step27_tutorial_summary", "안녕하세요 외침");
}

fn step27_tutorial_summary(ob) {
    send_line(ob, "[공]〔\x1b[40m\x1b[32m외 침\x1b[0;37;40m〕 : 안녕하세요");

    send_line(ob, "");

    send_line(ob, "용파리〔\x1b[40m\x1b[32m외 침\x1b[0;37;40m〕 : 안녕하세요?? [공]님.. 하하하~~");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step28_tutorial_final");
}

fn step28_tutorial_final(ob) {
    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"지금은 시간이 없으니 우선 이정도만 배우도록하고");

    send_line(ob, "강호를 돌아다니며 모르는점이 있을때는 『\x1b[1m도움\x1b[0;37;40m』을");

    send_line(ob, "입력해서 새로운 명령어를 숙지하도록 하여라 그것이");

    send_line(ob, "강호를 살아나가는 처세술이란다.\"");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"또한 시간이 난다면 낙양표국을 찾아가서 표국보험에");

    send_line(ob, "가입하도록 하여라 보험에 가입하는 방법은 표국을");

    send_line(ob, "찾아가서 『\x1b[1m안내\x1b[0;37;40m』라고 입력한다면 알수 있단다.\"");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("step29_final");
}

// ========== Act 8: Departure (steps 29-30) ==========

fn step29_final(ob) {
    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"어느정도 방법을 배웠으니 내 말이 끝나는 즉시");

    send_line(ob, "『\x1b[1m매의깃털 착용\x1b[0;37;40m』을 한다음 초보자수련장으로");

    send_line(ob, "찾아가도록 하거라. '초보자수련장' 이라고 치거나.  아니면");

    send_line(ob, "'왕대협 대화'라고 하면 초보자수련장으로 이동하게 되느니라.");

    send_line(ob, "");

    send_line(ob, "\x1b[33m왕대협\x1b[0;37;40m이 말합니다. \"어서 출발하도록 하여라 그리고 내가 다시 부르면");

    send_line(ob, "이곳에 다시 돌아오면 되느리라... 이곳에 돌아올때는");

    send_line(ob, "『\x1b[1m귀환\x1b[0;37;40m』이라고 하면 다시 돌아올수 있단다..");

    send_line(ob, "");

    send_line(ob, "【\x1b[1m엔터키를 누르세요\x1b[0;37;40m】");

    wait_enter("finish");
}

fn finish(ob) {
    finish_script(ob);
}
