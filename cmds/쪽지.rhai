// 쪽지.rhai - 쪽지 시스템
// Python: cmds/쪽지.py
// Rust 네이티브 command/commands/note.rs 가 우선 등록됨

fn cmd(ob, line) {
    send_line(ob, "☞ 쪽지 기능은 Rust 네이티브로 구현되어 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
