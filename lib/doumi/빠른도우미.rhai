// 도우미 스크립트: 빠른도우미 (단계 기반)
// 각 step 함수는 ob를 인자로 받고, wait_enter/wait_input으로 다음 단계 지정
// 주의: 데이터 저장 시 _saved_name/_saved_password/_saved_sex 사용

fn step1_welcome(ob) {
    send_line(ob, "");
    send_line(ob, "노인이 말합니다. \"그 아이의 이름은 무엇이던가?\"");
    wait_enter("step2_name");
}

fn step2_name(ob) {
    send_line(ob, "무림존함ː");
    wait_input("step3_password", "get_name");
}

fn step3_password(ob) {
    let name = _doumi_resume_input;
    send_line(ob, "");
    send_line(ob, "왕대협이 말합니다. \"" + name + "(이라/라)고 합니다.\"");
    send_line(ob, "");
    send_line(ob, "노인이 말합니다. \"음! 좋은 이름이군 그렇다면 암호는??\"");
    send_line(ob, "비밀번호ː");
    wait_input("step4_gender", "get_password");
}

fn step4_gender(ob) {
    let pwd = _doumi_resume_input;
    send_line(ob, "");
    send_line(ob, "노인이 말합니다. \"그런데 그아이는 남자인가? 여자인가?\"");
    send_line(ob, "성별(남/여)ː");
    wait_input("step5_finish", "get_sex");
}

fn step5_finish(ob) {
    let gender = _doumi_resume_input;
    send_line(ob, "");
    send_line(ob, "노인이 말합니다 \"자고로 상처입은 짐승이 찾아와도 보살펴");
    send_line(ob, "                 주는법 집사는 그 아이를 거두어 가르치시오\"");
    send_line(ob, "");
    send_line(ob, "【엔터키를 누르세요】");
    wait_enter("finish");
}

fn finish(ob) {
    // _saved_* 변수를 새 Map에 담아 finish_script 호출
    let data = #{"이름": _saved_name, "암호": _saved_password, "성별": _saved_sex};
    finish_script(data);
}
