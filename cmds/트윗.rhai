// 트윗.rhai - 트위터 명령어
// Python: cmds/트윗.py

fn cmd(ob, line) {
    if !require_arg(ob, line, "☞ 사용법: [내용] 트윗") { return; }
    if length(trim(line)) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^");
        return;
    }
    // checkConfig, act, 전체 플레이어 전송 등 서버 연동 필요
    send_line(ob, "☞ 아직 준비 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
