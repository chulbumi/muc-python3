// say.rhai - 말하기 명령어
//
// 사용법: say
// 같은 방에 있는 사람들에게 말을 전합니다.

fn main(ob, line) {
    let name = get_string(ob, "이름");
    let message = if !is_empty(line) { trim(line) } else { "안녕하세요" };

    // 발언자에게는 보여주지 않음. 나를 제외한 같은 방 다른 플레이어에게만 전송.
    send_room(ob, name + " 말합니다: '" + message + "'");
}
