// 쪽지.rhai - 쪽지 명령어 (Python: cmds/쪽지.py)
// Rust 네이티브 command/commands/note.rs 가 우선 등록되므로 이 스크립트는 호출되지 않습니다.

fn cmd(ob, line) {
    send_line(ob, "☞ 이 기능은 현재 지원되지 않습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
