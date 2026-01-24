// 값값.rhai - 값값 명령어. Python: cmds/값값.py
// 값값: 서버 시간(Unix timestamp) 출력. 값설정/값삭제는 별도 스크립트.

fn cmd(ob, line) {
    send_line(ob, int_to_str(get_timestamp()));
}

fn main(ob, line) {
    cmd(ob, line);
}
