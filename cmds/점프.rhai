// 점프.rhai - 점프 (관리자 1000, 쿨타임)
// Python: cmds/점프.py

// Rhai 식별자는 ASCII만 허용 → jump_land (점프_착지)
fn jump_land(ob) {
    set_value(ob, "cooltime", 0);
    send_line(ob, "당신은 안전하게 착지합니다. 만세^^v");
}

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { send_line(ob, "☞ 관리자만 사용할 수 있어요."); return; }
    let ct = get_body_int(ob, "cooltime");
    if ct == 1 {
        send_line(ob, "기술을 쓰기엔 너무도 바빠요~");
        return;
    }
    set_value(ob, "cooltime", 1);
    send_line(ob, "당신이 부웅~~ 날아 오릅니다");
    call_out(ob["이름"], "jump_land", 2);
}

fn main(ob, line) {
    cmd(ob, line);
}
