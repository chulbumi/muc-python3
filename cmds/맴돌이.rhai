// 맴돌이.rhai - 맴돌이. Python: cmds/맴돌이.py
// [출구] ob.env['출구'] 수정, 출구를 자기 방으로 연결. get_exits_string / set_room_exit efun 없음.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 사용법: [출구] 맴돌이");
            return;
        }
        send_line(ob, "☞ (출구/방 데이터 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
