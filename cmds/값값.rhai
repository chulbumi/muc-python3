// Python `cmds/값값.py`: 시간을 먼저 출력한 뒤 쉼표 구분 방 대상 속성을 설정한다.

fn cmd(ob, line) {
    send_line(ob, int_to_str(get_timestamp()));
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let result = admin_set_comma_value(ob, line);
    if result == "usage" {
        send_line(ob, "☞ 사용법: [대상],[키],[값] 값설정");
    } else if result == "too_long" {
        send_line(ob, "☞ 너무 길어요!");
    } else if result == "missing" {
        send_line(ob, "☞ 그런 대상이 없어요!");
    } else if result == "invalid" {
        send_line(ob, "☞ 잘못된 값입니다.");
    } else {
        send_line(ob, "☞ 값이 설정되었습니다.");
    }
}

fn main(ob, line) { cmd(ob, line); }
