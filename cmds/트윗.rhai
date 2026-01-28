// 트윗.rhai - 트윗터 (소셜 미지원)
// Python: cmds/트윗.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 트윗");
        return;
    }
    if length(trim(line)) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^");
        return;
    }
    send_line(ob, "☞ 트윗 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
