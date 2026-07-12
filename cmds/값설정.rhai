// Python `cmds/값설정.py`: [대상] [키] [값] 형식의 관리자 속성 변경.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let result = admin_set_space_value(ob, line);
    if result == "usage" {
        send_line(ob, "☞ 사용법: [대상] [키] [값] 값설정");
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
