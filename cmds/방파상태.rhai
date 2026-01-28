// 방파상태.rhai - 방파 상태 확인
// Python: cmds/방파상태.py

fn cmd(ob, line) {
    let so = get_string(ob, "소속");
    if so == "" {
        send_line(ob, "☞ 당신은 소속이 없습니다.");
        return;
    }
    send_line(ob, "☞ 방파 상태 표시 기능은 현재 구현 중입니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
