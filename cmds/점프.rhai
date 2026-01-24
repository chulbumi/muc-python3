// 점프.rhai - 점프 (관리자 1000, 쿨타임)
// Python: cmds/점프.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { return; }
    let ct = get_body_int(ob, "cooltime");
    if ct == 1 {
        send_line(ob, "기술을 쓰기엔 너무도 바빠요~");
        return;
    }
    set_value(ob, "cooltime", 1);
    send_line(ob, "당신이 부웅~~ 날아 오릅니다");
    // 파이썬: 2초 후 cooltime=0 및 "착지" 메시지 — call_out 연동 시 구현
}

fn main(ob, line) {
    cmd(ob, line);
}
