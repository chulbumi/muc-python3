// say.rhai - 말하기 명령어
//
// 사용법: say
// 같은 방에 있는 사람들에게 말을 전합니다.

fn main(ob, line) {
    let name = get_string(ob, "이름");
    let message = if !is_empty(line) { trim(line) } else { "안녕하세요" };

    send_line(ob, "당신이 말합니다: '" + message + "'");
    send_room(ob, name + " 말합니다: '" + message + "'");
}
