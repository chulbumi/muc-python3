// 설치.rhai - 방 설치
// Python: cmds/설치.py

fn cmd(ob, line) {
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 설치");
        return;
    }
    let result = install_item(ob, line);
    if result == "" {
        send_line(ob, "당신이 " + line + " 설치합니다.");
    } else {
        send_line(ob, result);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
