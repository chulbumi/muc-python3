// 도우미 스크립트: 빠른도우미
// Rhai 언어로 진행. set_tick, send_line, get_name, get_password, get_sex, get_enter, start_script, finish_script

start_script(ob);
set_tick(1);
send_line(ob, "");
send_line(ob, "노인이 말합니다. \"그 아이의 이름은 무엇이던가?\"");
ob["이름"] = get_name();
send_line(ob, "왕대협이 말합니다. \"[공](이라/라)고 합니다.\"");
send_line(ob, "노인이 말합니다. \"음! 좋은 이름이군 그렇다면 암호는??\"");
ob["암호"] = get_password();
send_line(ob, "노인이 말합니다. \"그런데 그아이는 남자인가? 여자인가?\"");
ob["성별"] = get_sex();
send_line(ob, "노인이 말합니다 \"자고로 상처입은 짐승이 찾아와도 보살펴");
send_line(ob, "                 주는법 집사는 그 아이를 거두어 가르치시오\"");
send_line(ob, "");
send_line(ob, "【엔터키를 누르세요】");
get_enter();
finish_script(ob);
